pub mod transport;
pub mod models;
pub mod error;
pub mod api;
pub mod state;
pub mod streaming;

// Re-export key types for easier access
pub use models::{Speaker, Group, SpeakerId, GroupId, SpeakerState, PlaybackState, StateChange};
pub use error::{SonosError, Result};
pub use api::zone_groups::{ZoneGroupsService, get_zone_groups_from_speaker, get_zone_groups_from_speaker_with_timeout};
pub use state::StateCache;
pub use transport::discovery::{discover_speakers, discover_speakers_with_timeout};
pub use streaming::{EventStream, StreamConfig, ServiceType, SubscriptionId};
