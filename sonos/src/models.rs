use crate::{group::Group, streaming::ServiceType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SpeakerId(String);

impl<'de> Deserialize<'de> for SpeakerId {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let id = String::deserialize(deserializer)?;
    Ok(Self::new(id))
  }
}

impl SpeakerId {
  /// Creates a new SpeakerId, stripping the "uuid:" prefix if present
  pub fn new(id: impl Into<String>) -> Self {
    let id = id.into();
    let normalized = id.strip_prefix("uuid:").unwrap_or(&id);
    Self(normalized.to_string())
  }

  /// Returns the ID as a string slice
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl std::fmt::Display for SpeakerId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl AsRef<str> for SpeakerId {
  fn as_ref(&self) -> &str {
    &self.0
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GroupId(String);

impl GroupId {
  pub fn new(id: impl Into<String>) -> Self {
    GroupId(id.into())
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }
}

#[derive(Debug, Clone)]
pub struct Speaker {
    pub id: SpeakerId,
    pub name: String,
    pub room_name: String,
    pub ip_address: String,
    pub port: u16,
    pub model_name: String,
    pub satellites: Vec<SpeakerId>,
}

impl Speaker {
  pub fn get_id(&self) -> &SpeakerId {
    &self.id
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
    Transitioning,
}

#[derive(Debug, Clone)]
pub struct SpeakerState {
    pub speaker: Speaker,
    pub playback_state: PlaybackState,
    pub volume: u8,
    pub muted: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub is_coordinator: bool,
    pub group_id: Option<GroupId>,
}

#[derive(Debug, Clone)]
pub enum StateChange {
    VolumeChanged {
        speaker_id: SpeakerId,
        volume: u8,
    },
    MuteChanged {
        speaker_id: SpeakerId,
        muted: bool,
    },
    PlaybackStateChanged {
        speaker_id: SpeakerId,
        state: PlaybackState,
    },
    PositionChanged {
        speaker_id: SpeakerId,
        position_ms: u64,
    },
    GroupChange {
      groups: Vec<Group>
    },
    // New streaming-specific variants
    TrackChanged {
        speaker_id: SpeakerId,
        track_info: Option<TrackInfo>,
    },
    TransportInfoChanged {
        speaker_id: SpeakerId,
        transport_state: PlaybackState,
        transport_status: TransportStatus,
    },
    SubscriptionError {
        speaker_id: SpeakerId,
        service: ServiceType,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportStatus {
    Ok,
    ErrorOccurred,
}
