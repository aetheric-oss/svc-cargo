use lib_common::log_macros;

log_macros!("ut", "test");

#[cfg(test)]
pub mod test_pool {
    use deadpool_redis::redis::{ToRedisArgs, Value};
    use std::collections::HashMap;
    use std::ops::{Deref, DerefMut};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    pub struct Connection {
        store: Arc<Mutex<HashMap<String, String>>>,
    }

    #[derive(Debug, Clone)]
    pub struct Pool {
        pub fail: bool,
        pub connection: Connection,
    }

    impl Default for Pool {
        fn default() -> Self {
            Pool {
                fail: false,
                connection: Connection {
                    store: Arc::new(Mutex::new(HashMap::new())),
                },
            }
        }
    }

    impl Pool {
        pub async fn get(&self) -> Result<Connection, ()> {
            if self.fail {
                return Err(());
            }

            Ok(self.connection.clone())
        }
    }

    impl Connection {
        pub async fn hget(&self, key: &str, _field: &str) -> Result<Value, ()> {
            // if no key provided, return error
            if key.ends_with(":") {
                return Err(());
            }

            self.store
                .try_lock()
                .map_err(|_| ())?
                .deref()
                .get(key)
                .map_or(Ok(Value::Nil), |v| Ok(Value::Data(v.as_bytes().to_vec())))
        }

        pub async fn hset_nx(
            &mut self,
            key: &str,
            field: &str,
            value: impl ToRedisArgs,
        ) -> Result<Value, ()> {
            // allow ways to exercise other branches
            if key.ends_with(":") {
                return Err(());
            }

            // allow ways to exercise other branches
            if field.is_empty() {
                return Ok(Value::Nil);
            }

            let value = value
                .to_redis_args()
                .into_iter()
                .map(|v| String::from_utf8_lossy(&v).to_string())
                .collect::<Vec<String>>()
                .join("");

            match self
                .store
                .try_lock()
                .map_err(|_| ())?
                .deref_mut()
                .insert(key.to_string(), value.to_string())
            {
                None => Ok(Value::Int(1)),
                Some(_) => Ok(Value::Int(0)),
            }
        }

        pub async fn expire(&mut self, key: &str, seconds: usize) -> Result<Value, ()> {
            // allow ways to exercise other branches
            if key.ends_with(":") {
                return Err(());
            }

            // allow ways to exercise other branches
            if seconds == 0 {
                return Ok(Value::Nil);
            }

            match self.store
                .try_lock()
                .map_err(|_| ())?
                .deref()
                .contains_key(key) // hashmap
            {
                true => Ok(Value::Int(1)),
                false => Ok(Value::Int(0)),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use deadpool_redis::redis::Value;

        #[tokio::test]
        async fn test_pool_get() {
            let mut pool = Pool::default();
            pool.fail = true;
            pool.get().await.unwrap_err();

            pool.fail = false;
            pool.get().await.unwrap();
        }

        #[tokio::test]
        async fn test_connection_hget() {
            // create store
            let pool = Pool::default();
            let mut connection = pool.get().await.unwrap(); // initializes store

            // keys ending in ":" return Err in this test util
            connection.hget(":", "field").await.unwrap_err();

            // if the key exists return the value, otherwise return nil
            let value = connection.hget("key", "field").await.unwrap();
            assert_eq!(value, Value::Nil);

            connection.hset_nx("key", "field", "value").await.unwrap();
            let value = connection.hget("key", "field").await.unwrap();
            assert_eq!(value, Value::Data(b"value".to_vec()));

            let binding = connection.store.clone();

            #[allow(unused_variables)]
            let lock = binding.try_lock().unwrap();
            // will fail if the store is locked
            connection.hget("key", "field").await.unwrap_err();
        }

        #[tokio::test]
        async fn test_connection_hset_nx() {
            // create store
            let pool = Pool::default();
            let mut connection = pool.get().await.unwrap(); // initializes store

            // keys ending in ":" should return error for this test util
            connection
                .hset_nx("key:", "field", "value")
                .await
                .unwrap_err();

            // empty field should return nil for this test util
            let value = connection.hset_nx("key", "", "value").await.unwrap();
            assert_eq!(value, Value::Nil);

            // first time should return 1, second time should return 0
            assert!(connection.store.try_lock().unwrap().is_empty());
            let value = connection.hset_nx("key", "field", "value").await.unwrap();
            assert_eq!(value, Value::Int(1));

            assert!(!connection.store.try_lock().unwrap().is_empty());
            let value = connection.hset_nx("key", "field", "value").await.unwrap();
            assert_eq!(value, Value::Int(0));

            let binding = connection.store.clone();

            #[allow(unused_variables)]
            let lock = binding.try_lock().unwrap();
            // will fail if the store is locked
            connection
                .hset_nx("key", "field", "value")
                .await
                .unwrap_err();
        }

        #[tokio::test]
        async fn test_connection_expire() {
            // create store
            let pool = Pool::default();
            let mut connection = pool.get().await.unwrap(); // initializes store

            // 0 expiration seconds results in Nil result in this test util
            let value = connection.expire("key", 0).await.unwrap();
            assert_eq!(value, Value::Nil);

            // keys ending in ":" should return error for this test util
            connection.expire("key:", 1).await.unwrap_err();

            // if the key exists return 1, otherwise return 0
            let value = connection.expire("key", 1).await.unwrap();
            assert_eq!(value, Value::Int(0));

            connection.hset_nx("key", "field", "value").await.unwrap();

            assert!(!connection.store.try_lock().unwrap().is_empty());
            let value = connection.expire("key", 1).await.unwrap();
            assert_eq!(value, Value::Int(1));

            let binding = connection.store.clone();
            #[allow(unused_variables)]
            let lock = binding.try_lock().unwrap();
            // will fail if the store is locked
            connection.expire("key", 1).await.unwrap_err();
        }
    }
}
