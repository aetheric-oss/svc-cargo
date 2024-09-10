//! Redis connection pool implementation
use super::Itinerary;
use deadpool_redis::redis::{FromRedisValue, Value};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};
use tonic::async_trait;

#[cfg(not(test))]
use deadpool_redis::{redis::AsyncCommands, Pool, Runtime};

#[cfg(test)]
use crate::test_util::test_pool::Pool;

/// How long to keep a task in memory after it's been processed
const ITINERARY_KEEPALIVE_DURATION_SECONDS: usize = 120;

/// A global static Redis pool.
static REDIS_POOL: OnceCell<Arc<Mutex<CargoPool>>> = OnceCell::const_new();

/// Returns a Redis Pool.
/// Uses host and port configurations using a Config object generated from
/// environment variables.
/// Initializes the pool if it hasn't been initialized yet.
#[cfg(not(tarpaulin_include))]
// no_coverage: (R5) can't fail
pub async fn get_pool() -> Result<Arc<Mutex<CargoPool>>, CacheError> {
    REDIS_POOL
        .get_or_try_init(|| async {
            let config = crate::Config::try_from_env().map_err(|_| {
                cache_error!("could not build configuration for cache.");
                CacheError::CouldNotConfigure
            })?;

            let pool = CargoPool::new(config)?;

            Ok(Arc::new(Mutex::new(pool)))
        })
        .await
        .map_err(|_: CacheError| {
            cache_error!("could not get Redis pool.");
            CacheError::CouldNotConnect
        })
        .cloned()
}

/// Represents errors that can occur during cache operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheError {
    /// Could not create pool
    CouldNotCreatePool,

    /// Pool was not available
    PoolUnavailable,

    /// Could not build configuration for cache.
    CouldNotConfigure,

    /// Could not connect to the Redis pool.
    CouldNotConnect,

    /// Key was not found
    NotFound,

    /// The operation on the Redis cache failed.
    OperationFailed,

    /// Invalid Value Retrieved
    InvalidValue,

    /// Unexpected Response
    Unexpected,

    /// KeyCollision
    KeyCollision,
}

impl Display for CacheError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            CacheError::PoolUnavailable => write!(f, "Pool is unavailable."),
            CacheError::CouldNotCreatePool => write!(f, "Could not create pool."),
            CacheError::CouldNotConfigure => write!(f, "Could not configure cache."),
            CacheError::CouldNotConnect => write!(f, "Could not connect to cache."),
            CacheError::OperationFailed => write!(f, "Cache operation failed."),
            CacheError::NotFound => write!(f, "Key was not found."),
            CacheError::InvalidValue => write!(f, "Invalid value retrieved."),
            CacheError::Unexpected => write!(f, "Unexpected response."),
            CacheError::KeyCollision => write!(f, "Key collision."),
        }
    }
}

/// Represents a pool of connections to a Redis server.
///
/// The [`CargoPool`] struct provides a managed pool of connections to a Redis server.
/// It allows clients to acquire and release connections from the pool and handles
/// connection management, such as connection pooling and reusing connections.
#[derive(Clone)]
pub struct CargoPool {
    /// The underlying pool of Redis connections.
    pool: Pool,
}

impl Debug for CargoPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CargoPool")
            // .field("pool", &self.pool) // doesn't implement Debug
            .finish()
    }
}

impl CargoPool {
    /// Create a new CargoPool
    #[cfg(test)]
    pub fn new(_config: crate::config::Config) -> Result<CargoPool, CacheError> {
        return Ok(CargoPool {
            pool: Pool::default(),
        });
    }

    #[cfg(not(test))]
    /// Create a new CargoPool
    pub fn new(config: crate::config::Config) -> Result<CargoPool, CacheError> {
        // the .env file must have REDIS__URL="redis://\<host\>:\<port\>"
        let cfg: deadpool_redis::Config = config.redis;
        let details = cfg.url.clone().ok_or_else(|| {
            cache_error!("(CargoPool new) no connection address found.");
            CacheError::CouldNotConfigure
        })?;

        cache_info!(
            "(CargoPool new) creating pool with key folder 'cargo' at {:?}...",
            details
        );

        let pool = cfg.create_pool(Some(Runtime::Tokio1)).map_err(|e| {
            cache_error!("(CargoPool new) could not create pool: {}", e);
            CacheError::CouldNotCreatePool
        })?;

        cache_info!("(CargoPool new) pool created.");
        Ok(CargoPool { pool })
    }
}

impl ItineraryPool for CargoPool {
    fn pool(&self) -> &Pool {
        &self.pool
    }
}

/// Trait for interacting with a cargo task pool
#[async_trait]
pub trait ItineraryPool {
    /// Returns a reference to the underlying pool.
    fn pool(&self) -> &Pool;

    /// Creates a new task and returns the task_id for it
    #[cfg(not(tarpaulin_include))]
    // no_coverage: (R5) need redis connection, run in integration tests
    async fn store_itinerary(
        &mut self,
        itinerary_id: String,
        draft_itinerary: &Itinerary,
    ) -> Result<(), CacheError>
    where
        Self: Send + Sync + 'async_trait,
    {
        cache_debug!("entry.");
        let mut connection = self.pool().get().await.map_err(|_| {
            cache_error!("(ItineraryPool new_task) could not get connection from pool.");

            CacheError::PoolUnavailable
        })?;

        let key = format!("cargo:draft:{itinerary_id}");
        let value = connection
            .hset_nx(&key, "data", draft_itinerary)
            .await
            .map_err(|e| {
                cache_error!(
                    "(ItineraryPool new_task) unexpected redis response to hsetnx command: {:?}",
                    e
                );
                CacheError::OperationFailed
            })?;

        match value {
            Value::Int(1) => {
                cache_debug!("(ItineraryPool new_task) successfully added itinerary with UUID {itinerary_id}.");
            }
            Value::Int(0) => {
                cache_debug!("(ItineraryPool new_task) key collision: {itinerary_id}");
                return Err(CacheError::KeyCollision);
            }
            value => {
                cache_error!(
                    "(ItineraryPool new_task) unexpected redis response to hsetnx command: {:?}",
                    value
                );

                return Err(CacheError::Unexpected);
            }
        };

        // Add expiration to itinerary
        let result = connection
            .expire(&key, ITINERARY_KEEPALIVE_DURATION_SECONDS)
            .await
            .map_err(|_| {
                cache_error!(
                    "(ItineraryPool new_task) could not set itinerary #{itinerary_id} expiry.",
                );

                CacheError::OperationFailed
            })?;

        match result {
            Value::Int(1) => {}
            value => {
                cache_error!(
                    "(ItineraryPool new_task) unexpected redis response to expire command: {:?}",
                    value
                );

                return Err(CacheError::Unexpected);
            }
        }

        cache_info!("(ItineraryPool new_task) created new draft itinerary #{itinerary_id}.",);
        cache_debug!(
            "(ItineraryPool new_task) new itinerary #{itinerary_id} data: {:?}",
            draft_itinerary
        );

        Ok(())
    }

    /// Gets task information
    #[cfg(not(tarpaulin_include))]
    // no_coverage: (R5) need redis connection, run in integration tests
    async fn get_itinerary(&mut self, itinerary_id: String) -> Result<Itinerary, CacheError>
    where
        Self: Send + Sync + 'async_trait,
    {
        let key = format!("cargo:draft:{itinerary_id}");
        let value = self
            .pool()
            .get()
            .await
            .map_err(|_| {
                cache_error!("(ItineraryPool get_task_data) could not get connection from pool.");
                CacheError::PoolUnavailable
            })?
            .hget(&key, "data")
            .await
            .map_err(|_| {
                cache_error!("(ItineraryPool get_task_data) could not get itinerary from Redis.");
                CacheError::OperationFailed
            })?;

        if value == Value::Nil {
            cache_error!("(ItineraryPool get_task_data) key expired or does not exist.");
            return Err(CacheError::NotFound);
        }

        Itinerary::from_redis_value(&value).map_err(|e| {
            cache_error!(
                "(ItineraryPool get_task_data) could not deserialize itinerary {:#?}; {e}",
                value
            );

            CacheError::InvalidValue
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::api::rest_types::CurrencyUnit;
    use crate::rest::api::rest_types::Itinerary;
    use lib_common::uuid::Uuid;

    #[test]
    fn test_cargo_pool_debug() {
        let str = format!(
            "{:?}",
            CargoPool {
                pool: Pool::default()
            }
        );
        assert_eq!(str, "CargoPool");
    }

    #[test]
    fn test_cache_error_display() {
        assert_eq!(
            format!("{}", CacheError::PoolUnavailable),
            "Pool is unavailable."
        );
        assert_eq!(
            format!("{}", CacheError::CouldNotCreatePool),
            "Could not create pool."
        );
        assert_eq!(
            format!("{}", CacheError::CouldNotConfigure),
            "Could not configure cache."
        );
        assert_eq!(
            format!("{}", CacheError::CouldNotConnect),
            "Could not connect to cache."
        );
        assert_eq!(
            format!("{}", CacheError::OperationFailed),
            "Cache operation failed."
        );
        assert_eq!(format!("{}", CacheError::NotFound), "Key was not found.");
        assert_eq!(
            format!("{}", CacheError::InvalidValue),
            "Invalid value retrieved."
        );
        assert_eq!(
            format!("{}", CacheError::Unexpected),
            "Unexpected response."
        );
        assert_eq!(format!("{}", CacheError::KeyCollision), "Key collision.");
    }

    #[tokio::test]
    async fn test_store_itinerary() {
        lib_common::logger::get_log_handle().await;
        ut_info!("start");

        let config = crate::config::Config::default();
        let mut pool = CargoPool::new(config).unwrap();
        let itinerary = Itinerary {
            flight_plans: vec![],
            invoice: vec![],
            currency_unit: CurrencyUnit::Usd,
            cargo_weight_g: 10,
            user_id: Uuid::new_v4().to_string(),
            acquisition_vertiport_id: Uuid::new_v4().to_string(),
            delivery_vertiport_id: Uuid::new_v4().to_string(),
        };

        // trigger get pool failure
        pool.pool.fail = true;
        let itinerary_id = Uuid::new_v4().to_string();
        let result = pool
            .store_itinerary(itinerary_id, &itinerary)
            .await
            .unwrap_err();
        assert_eq!(result, CacheError::PoolUnavailable);

        // trigger Err(())
        pool.pool.fail = false;
        let itinerary_id = "".to_string();
        let result = pool
            .store_itinerary(itinerary_id, &itinerary)
            .await
            .unwrap_err();
        assert_eq!(result, CacheError::OperationFailed);

        // trigger key collision
        let itinerary_id = Uuid::new_v4().to_string();
        pool.store_itinerary(itinerary_id.clone(), &itinerary)
            .await
            .unwrap();

        let result = pool
            .store_itinerary(itinerary_id, &itinerary)
            .await
            .unwrap_err();
        assert_eq!(result, CacheError::KeyCollision);

        ut_info!("success");
    }

    #[tokio::test]
    async fn test_get_itinerary() {
        lib_common::logger::get_log_handle().await;
        ut_info!("start");

        let config = crate::config::Config::default();
        let mut pool = CargoPool::new(config).unwrap();

        // trigger failing pool get
        let itinerary_id = Uuid::new_v4().to_string();
        pool.pool.fail = true;
        let result = pool.get_itinerary(itinerary_id).await.unwrap_err();
        assert_eq!(result, CacheError::PoolUnavailable);

        // path to get Err(()) route
        pool.pool.fail = false;
        let itinerary_id = "".to_string();
        let result = pool.get_itinerary(itinerary_id).await.unwrap_err();
        assert_eq!(result, CacheError::OperationFailed);

        // trigger not found
        let itinerary_id = Uuid::new_v4().to_string();
        let result = pool.get_itinerary(itinerary_id).await.unwrap_err();
        assert_eq!(result, CacheError::NotFound);

        // successful get
        let itinerary = Itinerary {
            flight_plans: vec![],
            invoice: vec![],
            currency_unit: CurrencyUnit::Usd,
            cargo_weight_g: 10,
            user_id: Uuid::new_v4().to_string(),
            acquisition_vertiport_id: Uuid::new_v4().to_string(),
            delivery_vertiport_id: Uuid::new_v4().to_string(),
        };

        let itinerary_id = Uuid::new_v4().to_string();
        pool.store_itinerary(itinerary_id.clone(), &itinerary)
            .await
            .unwrap();

        assert!(pool.get_itinerary(itinerary_id).await.is_ok());

        ut_info!("success");
    }
}
