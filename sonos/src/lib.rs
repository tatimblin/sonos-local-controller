pub mod transport;
pub mod models;
pub mod error;
pub mod api;

// Re-export key types for easier access
pub use models::{Speaker, Group, SpeakerId, GroupId, SpeakerState, PlaybackState, StateChange};
pub use error::{SonosError, Result};
pub use api::zone_groups::ZoneGroupsService;
