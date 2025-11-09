pub mod transport;
pub mod models;
pub mod error;
pub mod api;
pub mod state;
pub mod streaming;
pub mod service;
pub mod xml_decode;
pub mod group;

// Re-export key types for easier access
pub use models::{Speaker, SpeakerId, GroupId, SpeakerState, PlaybackState, StateChange};
pub use error::{SonosError, Result};
pub use state::StateCache;
pub use transport::discovery::{discover_speakers, discover_speakers_with_timeout};
pub use streaming::{EventStreamBuilder, ActiveEventStream, ServiceType, StreamError, LifecycleHandlers, StreamStats};
