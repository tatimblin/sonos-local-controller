#[derive(Debug, Clone, PartialEq)]
pub struct SpeakerInfo {
    /// IP address of the speaker
    pub ip: String,
    /// Friendly name of the speaker (e.g., "Living Room")
    pub name: String,
    /// Room name where the speaker is located
    pub room_name: String,
    /// Unique identifier (UUID) of the speaker
    pub uuid: String,
    /// Model name of the speaker (e.g., "Sonos One")
    pub model: String,
    /// Software version running on the speaker
    pub software_version: String,
}

impl SpeakerInfo {
    /// Create a new SpeakerInfo with minimal information
    pub fn new(ip: String, uuid: String) -> Self {
        Self {
            ip,
            name: "Unknown Speaker".to_string(),
            room_name: "Unknown Room".to_string(),
            uuid,
            model: "Unknown Model".to_string(),
            software_version: "Unknown".to_string(),
        }
    }
    
    /// Create a SpeakerInfo with full information
    pub fn with_details(
        ip: String,
        name: String,
        room_name: String,
        uuid: String,
        model: String,
        software_version: String,
    ) -> Self {
        Self {
            ip,
            name,
            room_name,
            uuid,
            model,
            software_version,
        }
    }
}