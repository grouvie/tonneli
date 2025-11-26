//! Core types and service wiring for the tonneli waste schedule aggregator.

/// Domain models and identifiers shared by all providers.
pub mod model;
/// Registry and helpers for plugging city-specific providers into the service.
pub mod plugin;
/// Traits describing the provider interfaces.
pub mod ports;
/// High-level service facade used by clients.
pub mod service;

pub use model::*;
pub use plugin::*;
pub use ports::*;
pub use service::*;
