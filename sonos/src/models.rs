#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeakerId(u32);

#[derive(Debug, Clone)]
pub struct Speaker {
    pub id: SpeakerId,
    pub udn: String,
    pub name: String,
    pub room_name: String,
    pub ip_address: String,
    pub port: u16,
    pub model_name: String,
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
}

#[derive(Debug, Clone)]
pub enum StateChange {
    VolumeChanged { speaker_id: SpeakerId, volume: u8 },
    MuteChanged { speaker_id: SpeakerId, muted: bool },
    PlaybackStateChanged { speaker_id: SpeakerId, state: PlaybackState },
    PositionChanged { speaker_id: SpeakerId, position_ms: u64 },
}
