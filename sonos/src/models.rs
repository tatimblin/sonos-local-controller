#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeakerId(u32);

impl SpeakerId {
    /// Create a SpeakerId from a UDN string
    pub fn from_udn(udn: &str) -> Self {
        // Extract the RINCON part and create a hash
        let hash = if let Some(rincon_part) = udn.strip_prefix("uuid:RINCON_") {
            let rincon_id = rincon_part.split("::").next().unwrap_or(rincon_part);
            // Simple hash of the RINCON ID
            rincon_id.chars().fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32))
        } else {
            // Fallback hash for non-RINCON UDNs
            udn.chars().fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32))
        };
        
        SpeakerId(hash)
    }
}

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
