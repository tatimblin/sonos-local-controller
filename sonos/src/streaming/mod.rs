// Internal modules
mod types;
mod subscription;
mod event_stream;
mod manager;
mod callback_server;
mod av_transport;

// Public interface modules
pub mod interface;
pub mod builder;

// Re-export only the new public interface types
pub use interface::{
    StreamError, LifecycleHandlers, StreamStats
};
pub use builder::{EventStreamBuilder, ActiveEventStream};

// Re-export essential types needed by the public interface
pub use types::ServiceType;

// Internal re-exports for use within the streaming module
pub(crate) use types::StreamConfig;
pub(crate) use event_stream::EventStream;