//! gRPC
//! provides Redis implementations for caching layer

#[macro_use]
pub mod macros;
pub mod pool;

use crate::rest::api::rest_types::{CurrencyUnit, FlightPlan};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DraftItinerary {
    pub flight_plans: Vec<FlightPlan>,
    pub cost: f64,
    pub currency_unit: CurrencyUnit,
}

/// Errors that can occur when processing a task
#[derive(Copy, Clone, Debug)]
pub enum ItineraryError {
    /// Task id was not found
    NotFound,

    /// Internal error with updating task
    Internal,

    /// Task was already processed
    AlreadyProcessed,

    /// Invalid data provided
    InvalidData,

    /// Schedule Conflict
    ScheduleConflict,
}

impl Display for ItineraryError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ItineraryError::NotFound => write!(f, "Itinerary not found."),
            ItineraryError::Internal => write!(f, "Internal error."),
            ItineraryError::AlreadyProcessed => write!(f, "Itinerary already processed."),
            ItineraryError::InvalidData => write!(f, "Invalid data."),
            ItineraryError::ScheduleConflict => write!(f, "Schedule conflict."),
        }
    }
}

/// Adds a draft itinerary for the user
pub async fn store_itinerary(itinerary: DraftItinerary) -> Result<(), ItineraryError> {
    let Some(mut pool) = crate::cache::pool::get_pool().await else {
        cache_error!("(store_itinerary) Couldn't get the redis pool.");
        return Err(ItineraryError::Internal);
    };

    if let Err(e) = pool.store_itinerary(itinerary).await {
        cache_warn!("(store_itinerary) error storing itinerary: {}", e);
        return Err(ItineraryError::Internal);
    }

    Ok(())
}

/// Gets a svc-cargo itinerary
pub async fn get_itinerary(itinerary_id: String) -> Result<DraftItinerary, ItineraryError> {
    let Some(mut pool) = crate::cache::pool::get_pool().await else {
        cache_error!("(get_itinerary) Couldn't get the redis pool.");
        return Err(ItineraryError::Internal);
    };

    match pool.get_itinerary(itinerary_id).await {
        Ok(itinerary) => Ok(itinerary),
        Err(e) => {
            cache_warn!("(get_itinerary) error getting itinerary: {}", e);
            Err(ItineraryError::NotFound)
        }
    }
}
