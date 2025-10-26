// Internal modules
mod types;
pub mod subscription;
mod event_stream;
mod manager;
mod callback_server;

// Public interface modules
pub mod interface;
pub mod builder;

// Re-export only the new public interface types
pub use interface::{
    StreamError, LifecycleHandlers, StreamStats
};
pub use builder::{EventStreamBuilder, ActiveEventStream};

// Re-export essential types needed by the public interface
pub use types::{ServiceType, SubscriptionScope, SubscriptionConfig, SubscriptionId};