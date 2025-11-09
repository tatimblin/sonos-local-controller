// Internal modules
mod types;
pub mod subscription;
mod event_stream;
mod manager;
mod callback_server;
mod av_transport;
mod rendering_control;
mod zone_group_topology;

// Public interface modules
pub mod interface;
pub mod builder;

// Re-export only the new public interface types
pub use interface::{
    StreamError, LifecycleHandlers, StreamStats
};
pub use builder::{EventStreamBuilder, ActiveEventStream};

// Re-export essential types needed by the public interface
pub use types::{ServiceType, SubscriptionScope, SubscriptionConfig};

// Internal re-exports for use within the streaming module
// (Currently no internal re-exports needed)

// Re-export for testing (available in both test and non-test builds for integration tests)
pub use zone_group_topology::ZoneGroupTopologySubscription;