pub mod types;
pub mod subscription;
pub mod event_stream;
pub mod manager;
pub mod callback_server;

// Re-export key types for easier access
pub use types::{
    ServiceType, SubscriptionId, StreamConfig, SubscriptionConfig, RawEvent
};
pub use subscription::{ServiceSubscription, SubscriptionError};
pub use event_stream::EventStream;
pub use manager::SubscriptionManager;
pub use callback_server::{CallbackServer, EventRouter, ServerStatus};