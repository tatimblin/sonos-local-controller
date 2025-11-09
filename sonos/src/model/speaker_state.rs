use crate::{GroupId, PlaybackState, Speaker};

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