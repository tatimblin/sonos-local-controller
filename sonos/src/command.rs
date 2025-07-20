/// Commands that can be sent to speakers
#[derive(Debug, Clone)]
pub enum SpeakerCommand {
    /// Play the current track
    Play,
    /// Pause playback
    Pause,
    /// Set volume to a specific level (0-100)
    SetVolume(u8),
    /// Adjust volume by a relative amount (positive or negative)
    AdjustVolume(i8),
}