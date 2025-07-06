#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
  Startup,
  Control,
}

#[derive(Debug, Clone)]
pub struct Topology {
    pub groups: Vec<Group>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,        // Name of the coordinator speaker
    pub speakers: Vec<SpeakerInfo>, // Information about all speakers in the group
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpeakerInfo {
    pub name: String,        // Human-readable name of the speaker
    pub uuid: String,        // Unique identifier for the speaker
    pub ip: String,          // IP address of the speaker
    pub is_coordinator: bool, // Whether this speaker is the group coordinator
}

impl SpeakerInfo {
    /// Helper function to create a SpeakerInfo from a name for testing
    pub fn from_name(name: &str, is_coordinator: bool) -> Self {
        Self {
            name: name.to_string(),
            uuid: format!("RINCON_{}", name.replace(" ", "_").to_uppercase()),
            ip: "192.168.1.100".to_string(), // Default IP for tests
            is_coordinator,
        }
    }
}

impl PartialEq<str> for SpeakerInfo {
    fn eq(&self, other: &str) -> bool {
        self.name == other
    }
}

impl PartialEq<&str> for SpeakerInfo {
    fn eq(&self, other: &&str) -> bool {
        self.name == *other
    }
}

// Re-export the full topology types from sonos crate for enhanced functionality
pub use sonos::{Topology as SonosTopology, ZoneGroup, ZoneGroupMember, Satellite};
