//! gRPC
//! provides Redis implementations for caching layer

#[macro_use]
pub mod macros;
pub mod pool;

use crate::rest::api::rest_types::Itinerary;
use deadpool_redis::redis::{
    ErrorKind, FromRedisValue, RedisError, RedisWrite, ToRedisArgs, Value,
};

impl FromRedisValue for Itinerary {
    fn from_redis_value(v: &Value) -> Result<Self, RedisError> {
        let Value::Data(data) = v else {
            return Err(RedisError::from((
                ErrorKind::TypeError,
                "Unexpected Redis value",
            )));
        };

        let itinerary = serde_json::from_slice(data).map_err(|e| {
            cache_warn!("error deserializing task: {}", e);
            RedisError::from((ErrorKind::TypeError, "Invalid JSON"))
        })?;

        Ok(itinerary)
    }
}

impl ToRedisArgs for Itinerary {
    fn write_redis_args<W: ?Sized>(&self, out: &mut W)
    where
        W: RedisWrite,
    {
        let Ok(data) = serde_json::to_string(self) else {
            cache_warn!("error serializing task");
            return;
        };

        out.write_arg(data.as_bytes());
    }
}
