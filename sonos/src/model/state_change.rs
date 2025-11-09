use crate::{PlaybackState, ServiceType, SpeakerId, group::Group, model::TrackInfo};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportStatus {
    Ok,
    ErrorOccurred,
}
