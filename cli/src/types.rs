#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
  Startup,
  Control,
}

#[derive(Debug, Clone)]
pub struct Topology {
    pub groups: Vec<Group>,
}

impl Topology {
    /// Find a speaker by UUID across all groups
    pub fn get_speaker_by_uuid(&self, uuid: &str) -> Option<&SpeakerInfo> {
        self.groups.iter()
            .flat_map(|group| &group.speakers)
            .find(|speaker| speaker.uuid == uuid)
    }

    /// Find a group by UUID (using any speaker's UUID in the group)
    pub fn get_group_by_uuid(&self, uuid: &str) -> Option<&Group> {
        self.groups.iter().find(|group| {
            group.speakers.iter().any(|speaker| speaker.uuid == uuid)
        })
    }

    /// Find a group by coordinator UUID
    pub fn get_group_by_coordinator_uuid(&self, coordinator_uuid: &str) -> Option<&Group> {
        self.groups.iter().find(|group| {
            group.speakers.iter().any(|speaker| speaker.uuid == coordinator_uuid && speaker.is_coordinator)
        })
    }

    /// Get the selected group based on app state selection
    pub fn get_selected_group(&self, selected_group_uuid: Option<&String>) -> Option<&Group> {
        selected_group_uuid
            .and_then(|uuid| self.get_group_by_coordinator_uuid(uuid))
    }

    /// Get the selected speaker based on app state selection
    pub fn get_selected_speaker(&self, selected_speaker_uuid: Option<&String>) -> Option<&SpeakerInfo> {
        selected_speaker_uuid
            .and_then(|uuid| self.get_speaker_by_uuid(uuid))
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,        // Name of the coordinator speaker
    pub speakers: Vec<SpeakerInfo>, // Information about all speakers in the group
}

impl Group {
    /// Pauses playback for this group by delegating to the System's zone group lookup
    /// 
    /// # Arguments
    /// * `system` - Reference to the System containing zone group instances
    /// 
    /// # Returns
    /// * `Ok(())` - If the pause command was successful
    /// * `Err(SpeakerManagerError)` - If the group is not found or the command fails
    pub fn pause(&self, system: &System) -> Result<(), SpeakerManagerError> {
        system.get_zone_group_by_name(&self.name)
            .ok_or_else(|| SpeakerManagerError::GroupNotFound(self.name.clone()))?
            .pause(system)
            .map_err(SpeakerManagerError::from)
    }

    /// Starts playback for this group by delegating to the System's zone group lookup
    /// 
    /// # Arguments
    /// * `system` - Reference to the System containing zone group instances
    /// 
    /// # Returns
    /// * `Ok(())` - If the play command was successful
    /// * `Err(SpeakerManagerError)` - If the group is not found or the command fails
    pub fn play(&self, system: &System) -> Result<(), SpeakerManagerError> {
        system.get_zone_group_by_name(&self.name)
            .ok_or_else(|| SpeakerManagerError::GroupNotFound(self.name.clone()))?
            .play(system)
            .map_err(SpeakerManagerError::from)
    }
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

    /// Pauses playback for this speaker by delegating to the System's speaker lookup
    /// 
    /// # Arguments
    /// * `system` - Reference to the System containing speaker instances
    /// 
    /// # Returns
    /// * `Ok(())` - If the pause command was successful
    /// * `Err(SpeakerManagerError)` - If the speaker is not found or the command fails
    pub fn pause(&self, system: &System) -> Result<(), SpeakerManagerError> {
        system.get_speaker_by_uuid(&self.uuid)
            .ok_or_else(|| SpeakerManagerError::SpeakerNotFound(self.uuid.clone()))?
            .pause()
            .map_err(SpeakerManagerError::from)
    }

    /// Starts playback for this speaker by delegating to the System's speaker lookup
    /// 
    /// # Arguments
    /// * `system` - Reference to the System containing speaker instances
    /// 
    /// # Returns
    /// * `Ok(())` - If the play command was successful
    /// * `Err(SpeakerManagerError)` - If the speaker is not found or the command fails
    pub fn play(&self, system: &System) -> Result<(), SpeakerManagerError> {
        system.get_speaker_by_uuid(&self.uuid)
            .ok_or_else(|| SpeakerManagerError::SpeakerNotFound(self.uuid.clone()))?
            .play()
            .map_err(SpeakerManagerError::from)
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
pub use sonos::{Topology as SonosTopology, ZoneGroup, ZoneGroupMember, Satellite, SonosError, System};

/// Error types specific to speaker management operations
#[derive(Debug)]
pub enum SpeakerManagerError {
    SpeakerNotFound(String),
    GroupNotFound(String),
    SonosError(SonosError),
}

impl std::fmt::Display for SpeakerManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SpeakerManagerError::SpeakerNotFound(uuid) => write!(f, "Speaker not found: {}", uuid),
            SpeakerManagerError::GroupNotFound(name) => write!(f, "Group not found: {}", name),
            SpeakerManagerError::SonosError(err) => write!(f, "Sonos error: {}", err),
        }
    }
}

impl std::error::Error for SpeakerManagerError {}

impl From<SonosError> for SpeakerManagerError {
    fn from(error: SonosError) -> Self {
        SpeakerManagerError::SonosError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_topology() -> Topology {
        let group1 = Group {
            name: "Living Room".to_string(),
            speakers: vec![
                SpeakerInfo {
                    name: "Living Room".to_string(),
                    uuid: "RINCON_000E58FE3AEA01400".to_string(),
                    ip: "192.168.1.100".to_string(),
                    is_coordinator: true,
                },
                SpeakerInfo {
                    name: "Kitchen".to_string(),
                    uuid: "RINCON_000E58FE3AEA01401".to_string(),
                    ip: "192.168.1.101".to_string(),
                    is_coordinator: false,
                },
            ],
        };

        let group2 = Group {
            name: "Bedroom".to_string(),
            speakers: vec![
                SpeakerInfo {
                    name: "Bedroom".to_string(),
                    uuid: "RINCON_000E58FE3AEA01402".to_string(),
                    ip: "192.168.1.102".to_string(),
                    is_coordinator: true,
                },
            ],
        };

        Topology {
            groups: vec![group1, group2],
        }
    }

    #[test]
    fn test_get_speaker_by_uuid() {
        let topology = create_test_topology();

        // Test finding existing speakers
        let living_room = topology.get_speaker_by_uuid("RINCON_000E58FE3AEA01400");
        assert!(living_room.is_some());
        assert_eq!(living_room.unwrap().name, "Living Room");
        assert!(living_room.unwrap().is_coordinator);

        let kitchen = topology.get_speaker_by_uuid("RINCON_000E58FE3AEA01401");
        assert!(kitchen.is_some());
        assert_eq!(kitchen.unwrap().name, "Kitchen");
        assert!(!kitchen.unwrap().is_coordinator);

        let bedroom = topology.get_speaker_by_uuid("RINCON_000E58FE3AEA01402");
        assert!(bedroom.is_some());
        assert_eq!(bedroom.unwrap().name, "Bedroom");
        assert!(bedroom.unwrap().is_coordinator);

        // Test non-existent UUID
        let non_existent = topology.get_speaker_by_uuid("RINCON_NONEXISTENT");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_get_group_by_uuid() {
        let topology = create_test_topology();

        // Test finding group by coordinator UUID
        let group1 = topology.get_group_by_uuid("RINCON_000E58FE3AEA01400");
        assert!(group1.is_some());
        assert_eq!(group1.unwrap().name, "Living Room");

        // Test finding group by non-coordinator UUID
        let group1_by_kitchen = topology.get_group_by_uuid("RINCON_000E58FE3AEA01401");
        assert!(group1_by_kitchen.is_some());
        assert_eq!(group1_by_kitchen.unwrap().name, "Living Room");

        // Test finding single-speaker group
        let group2 = topology.get_group_by_uuid("RINCON_000E58FE3AEA01402");
        assert!(group2.is_some());
        assert_eq!(group2.unwrap().name, "Bedroom");

        // Test non-existent UUID
        let non_existent = topology.get_group_by_uuid("RINCON_NONEXISTENT");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_get_group_by_coordinator_uuid() {
        let topology = create_test_topology();

        // Test finding group by coordinator UUID
        let group1 = topology.get_group_by_coordinator_uuid("RINCON_000E58FE3AEA01400");
        assert!(group1.is_some());
        assert_eq!(group1.unwrap().name, "Living Room");

        let group2 = topology.get_group_by_coordinator_uuid("RINCON_000E58FE3AEA01402");
        assert!(group2.is_some());
        assert_eq!(group2.unwrap().name, "Bedroom");

        // Test non-coordinator UUID should not find group
        let non_coordinator = topology.get_group_by_coordinator_uuid("RINCON_000E58FE3AEA01401");
        assert!(non_coordinator.is_none());

        // Test non-existent UUID
        let non_existent = topology.get_group_by_coordinator_uuid("RINCON_NONEXISTENT");
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_get_selected_group() {
        let topology = create_test_topology();

        // Test with valid coordinator UUID
        let coordinator_uuid = "RINCON_000E58FE3AEA01400".to_string();
        let selected_group = topology.get_selected_group(Some(&coordinator_uuid));
        assert!(selected_group.is_some());
        assert_eq!(selected_group.unwrap().name, "Living Room");

        // Test with None selection
        let no_selection = topology.get_selected_group(None);
        assert!(no_selection.is_none());

        // Test with invalid UUID
        let invalid_uuid = "RINCON_INVALID".to_string();
        let invalid_selection = topology.get_selected_group(Some(&invalid_uuid));
        assert!(invalid_selection.is_none());
    }

    #[test]
    fn test_get_selected_speaker() {
        let topology = create_test_topology();

        // Test with valid speaker UUID
        let speaker_uuid = "RINCON_000E58FE3AEA01401".to_string();
        let selected_speaker = topology.get_selected_speaker(Some(&speaker_uuid));
        assert!(selected_speaker.is_some());
        assert_eq!(selected_speaker.unwrap().name, "Kitchen");

        // Test with None selection
        let no_selection = topology.get_selected_speaker(None);
        assert!(no_selection.is_none());

        // Test with invalid UUID
        let invalid_uuid = "RINCON_INVALID".to_string();
        let invalid_selection = topology.get_selected_speaker(Some(&invalid_uuid));
        assert!(invalid_selection.is_none());
    }

    #[test]
    fn test_uuid_lookup_performance() {
        // Create a larger topology for performance testing
        let mut groups = Vec::new();
        for i in 0..100 {
            let group = Group {
                name: format!("Group {}", i),
                speakers: vec![
                    SpeakerInfo {
                        name: format!("Speaker {}", i),
                        uuid: format!("RINCON_000E58FE3AEA{:05}", i),
                        ip: format!("192.168.1.{}", i % 255),
                        is_coordinator: true,
                    },
                ],
            };
            groups.push(group);
        }

        let topology = Topology { groups };

        // Test that lookups work correctly even with many groups
        let speaker = topology.get_speaker_by_uuid("RINCON_000E58FE3AEA00050");
        assert!(speaker.is_some());
        assert_eq!(speaker.unwrap().name, "Speaker 50");

        let group = topology.get_group_by_coordinator_uuid("RINCON_000E58FE3AEA00075");
        assert!(group.is_some());
        assert_eq!(group.unwrap().name, "Group 75");
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_group_pause_command() {
        use sonos::speaker::mock::MockSpeakerBuilder;
        use std::sync::Arc;

        // Create a mock system with speakers and topology
        let mut system = System::new().unwrap();
        
        // Add mock speakers to the system
        let coordinator_speaker = MockSpeakerBuilder::new()
            .uuid("RINCON_000E58FE3AEA01400")
            .name("Living Room")
            .ip("192.168.1.100")
            .build();
        
        system.add_speaker_for_test(Box::new(coordinator_speaker));

        // Create test topology with zone group
        let test_topology = sonos::topology::types::Topology {
            zone_groups: vec![
                sonos::topology::types::ZoneGroup {
                    coordinator: "RINCON_000E58FE3AEA01400".to_string(),
                    id: "RINCON_000E58FE3AEA01400:123".to_string(),
                    members: vec![
                        sonos::topology::types::ZoneGroupMember {
                            uuid: "RINCON_000E58FE3AEA01400".to_string(),
                            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                            zone_name: "Living Room".to_string(),
                            software_version: "56.0-76060".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:living".to_string(),
                            satellites: vec![],
                        }
                    ],
                }
            ],
            vanished_devices: None,
        };

        // Set the topology on the system
        system.topology = Some(test_topology);

        // Create CLI Group
        let group = Group {
            name: "Living Room".to_string(),
            speakers: vec![
                SpeakerInfo {
                    name: "Living Room".to_string(),
                    uuid: "RINCON_000E58FE3AEA01400".to_string(),
                    ip: "192.168.1.100".to_string(),
                    is_coordinator: true,
                },
            ],
        };

        // Test successful pause command
        let result = group.pause(&system);
        assert!(result.is_ok(), "Group pause should succeed: {:?}", result);
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_group_play_command() {
        use sonos::speaker::mock::MockSpeakerBuilder;

        // Create a mock system with speakers and topology
        let mut system = System::new().unwrap();
        
        // Add mock speakers to the system
        let coordinator_speaker = MockSpeakerBuilder::new()
            .uuid("RINCON_000E58FE3AEA01400")
            .name("Living Room")
            .ip("192.168.1.100")
            .build();
        
        system.add_speaker_for_test(Box::new(coordinator_speaker));

        // Create test topology with zone group
        let test_topology = sonos::topology::types::Topology {
            zone_groups: vec![
                sonos::topology::types::ZoneGroup {
                    coordinator: "RINCON_000E58FE3AEA01400".to_string(),
                    id: "RINCON_000E58FE3AEA01400:123".to_string(),
                    members: vec![
                        sonos::topology::types::ZoneGroupMember {
                            uuid: "RINCON_000E58FE3AEA01400".to_string(),
                            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                            zone_name: "Living Room".to_string(),
                            software_version: "56.0-76060".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:living".to_string(),
                            satellites: vec![],
                        }
                    ],
                }
            ],
            vanished_devices: None,
        };

        // Set the topology on the system
        system.topology = Some(test_topology);

        // Create CLI Group
        let group = Group {
            name: "Living Room".to_string(),
            speakers: vec![
                SpeakerInfo {
                    name: "Living Room".to_string(),
                    uuid: "RINCON_000E58FE3AEA01400".to_string(),
                    ip: "192.168.1.100".to_string(),
                    is_coordinator: true,
                },
            ],
        };

        // Test successful play command
        let result = group.play(&system);
        assert!(result.is_ok(), "Group play should succeed: {:?}", result);
    }

    #[test]
    fn test_group_command_with_missing_group() {
        // Create system without the required zone group
        let system = System::new().unwrap();

        // Create CLI Group that doesn't exist in the system
        let group = Group {
            name: "Non-existent Group".to_string(),
            speakers: vec![
                SpeakerInfo {
                    name: "Non-existent Speaker".to_string(),
                    uuid: "RINCON_NONEXISTENT".to_string(),
                    ip: "192.168.1.100".to_string(),
                    is_coordinator: true,
                },
            ],
        };

        // Test pause command with missing group
        let pause_result = group.pause(&system);
        assert!(pause_result.is_err());
        match pause_result.unwrap_err() {
            SpeakerManagerError::GroupNotFound(name) => {
                assert_eq!(name, "Non-existent Group");
            }
            _ => panic!("Expected GroupNotFound error"),
        }

        // Test play command with missing group
        let play_result = group.play(&system);
        assert!(play_result.is_err());
        match play_result.unwrap_err() {
            SpeakerManagerError::GroupNotFound(name) => {
                assert_eq!(name, "Non-existent Group");
            }
            _ => panic!("Expected GroupNotFound error"),
        }
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_speaker_info_pause_command() {
        use sonos::speaker::mock::MockSpeakerBuilder;

        // Create a mock system with speakers
        let mut system = System::new().unwrap();
        
        // Add mock speaker to the system
        let speaker = MockSpeakerBuilder::new()
            .uuid("RINCON_000E58FE3AEA01400")
            .name("Living Room")
            .ip("192.168.1.100")
            .build();
        
        system.add_speaker_for_test(Box::new(speaker));

        // Create CLI SpeakerInfo
        let speaker_info = SpeakerInfo {
            name: "Living Room".to_string(),
            uuid: "RINCON_000E58FE3AEA01400".to_string(),
            ip: "192.168.1.100".to_string(),
            is_coordinator: true,
        };

        // Test successful pause command
        let result = speaker_info.pause(&system);
        assert!(result.is_ok(), "Speaker pause should succeed: {:?}", result);
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_speaker_info_play_command() {
        use sonos::speaker::mock::MockSpeakerBuilder;

        // Create a mock system with speakers
        let mut system = System::new().unwrap();
        
        // Add mock speaker to the system
        let speaker = MockSpeakerBuilder::new()
            .uuid("RINCON_000E58FE3AEA01400")
            .name("Living Room")
            .ip("192.168.1.100")
            .build();
        
        system.add_speaker_for_test(Box::new(speaker));

        // Create CLI SpeakerInfo
        let speaker_info = SpeakerInfo {
            name: "Living Room".to_string(),
            uuid: "RINCON_000E58FE3AEA01400".to_string(),
            ip: "192.168.1.100".to_string(),
            is_coordinator: true,
        };

        // Test successful play command
        let result = speaker_info.play(&system);
        assert!(result.is_ok(), "Speaker play should succeed: {:?}", result);
    }

    #[test]
    fn test_speaker_info_command_with_missing_speaker() {
        // Create system without the required speaker
        let system = System::new().unwrap();

        // Create CLI SpeakerInfo that doesn't exist in the system
        let speaker_info = SpeakerInfo {
            name: "Non-existent Speaker".to_string(),
            uuid: "RINCON_NONEXISTENT".to_string(),
            ip: "192.168.1.100".to_string(),
            is_coordinator: true,
        };

        // Test pause command with missing speaker
        let pause_result = speaker_info.pause(&system);
        assert!(pause_result.is_err());
        match pause_result.unwrap_err() {
            SpeakerManagerError::SpeakerNotFound(uuid) => {
                assert_eq!(uuid, "RINCON_NONEXISTENT");
            }
            _ => panic!("Expected SpeakerNotFound error"),
        }

        // Test play command with missing speaker
        let play_result = speaker_info.play(&system);
        assert!(play_result.is_err());
        match play_result.unwrap_err() {
            SpeakerManagerError::SpeakerNotFound(uuid) => {
                assert_eq!(uuid, "RINCON_NONEXISTENT");
            }
            _ => panic!("Expected SpeakerNotFound error"),
        }
    }

    #[test]
    fn test_speaker_manager_error_display() {
        let speaker_error = SpeakerManagerError::SpeakerNotFound("RINCON_123".to_string());
        assert_eq!(speaker_error.to_string(), "Speaker not found: RINCON_123");

        let group_error = SpeakerManagerError::GroupNotFound("Living Room".to_string());
        assert_eq!(group_error.to_string(), "Group not found: Living Room");

        let sonos_error = SpeakerManagerError::SonosError(SonosError::DeviceUnreachable);
        assert_eq!(sonos_error.to_string(), "Sonos error: Failed to call Sonos endpoint");
    }

    #[test]
    fn test_speaker_manager_error_from_sonos_error() {
        let sonos_error = SonosError::DeviceUnreachable;
        let manager_error: SpeakerManagerError = sonos_error.into();
        
        match manager_error {
            SpeakerManagerError::SonosError(SonosError::DeviceUnreachable) => {
                // Expected conversion
            }
            _ => panic!("Expected SonosError conversion"),
        }
    }
}
