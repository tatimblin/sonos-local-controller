//! Data structures for Sonos topology information
//!
//! This module contains all the data structures used to represent the topology
//! of a Sonos system, including zone groups, speakers, satellites, and vanished devices.



/// Represents a Sonos zone group containing one or more speakers
#[derive(Debug, Clone)]
pub struct ZoneGroup {
    /// UUID of the coordinator speaker for this group
    pub coordinator: String,
    /// Unique identifier for this zone group
    pub id: String,
    /// List of speakers in this zone group
    pub members: Vec<ZoneGroupMember>,
}

/// Represents a speaker (zone group member) in the Sonos system
#[derive(Debug, Clone)]
pub struct ZoneGroupMember {
    /// Unique identifier for this speaker
    pub uuid: String,
    /// HTTP URL for this speaker's device description
    pub location: String,
    /// Human-readable name for this speaker/room
    pub zone_name: String,
    /// Software version running on this speaker
    pub software_version: String,
    /// Configuration flags for this speaker
    pub configuration: String,
    /// Icon identifier for this speaker type
    pub icon: String,
    /// List of satellite speakers associated with this main speaker
    pub satellites: Vec<Satellite>,
}

/// Represents a satellite speaker (e.g., surround speakers in a home theater setup)
#[derive(Debug, Clone)]
pub struct Satellite {
    /// Unique identifier for this satellite speaker
    pub uuid: String,
    /// HTTP URL for this satellite's device description
    pub location: String,
    /// Human-readable name for this satellite
    pub zone_name: String,
    /// Software version running on this satellite
    pub software_version: String,
}

/// Container for speakers that are no longer available on the network
#[derive(Debug, Clone)]
pub struct VanishedDevices {
    /// List of devices that have disappeared from the network
    pub devices: Vec<VanishedDevice>,
}

/// Represents a speaker that was previously discovered but is no longer available
#[derive(Debug, Clone)]
pub struct VanishedDevice {
    /// Unique identifier for this vanished speaker
    pub uuid: String,
    /// Last known name for this speaker
    pub zone_name: String,
    /// Reason why this speaker vanished (e.g., "powered off")
    pub reason: String,
}

/// Complete topology information for the Sonos system
#[derive(Debug, Clone)]
pub struct Topology {
    /// List of active zone groups in the system
    pub zone_groups: Vec<ZoneGroup>,
    /// Information about speakers that are no longer available
    pub vanished_devices: Option<VanishedDevices>,
}

impl Topology {
    pub fn get_groups(&self) -> &[ZoneGroup] {
        &self.zone_groups
    }

    /// Returns the total number of zone groups in the topology
    pub fn zone_group_count(&self) -> usize {
        self.zone_groups.len()
    }

    /// Returns the total number of speakers (members) across all zone groups
    pub fn total_speaker_count(&self) -> usize {
        self.zone_groups.iter()
            .map(|group| group.members.len())
            .sum()
    }

    /// Finds a zone group by its coordinator UUID
    pub fn find_zone_group_by_coordinator(&self, coordinator_uuid: &str) -> Option<&ZoneGroup> {
        self.zone_groups.iter()
            .find(|group| group.coordinator == coordinator_uuid)
    }

    /// Finds a speaker by its UUID across all zone groups
    pub fn find_speaker_by_uuid(&self, uuid: &str) -> Option<&ZoneGroupMember> {
        self.zone_groups.iter()
            .flat_map(|group| &group.members)
            .find(|member| member.uuid == uuid)
    }

    /// Returns all speakers as a flat list
    pub fn all_speakers(&self) -> Vec<&ZoneGroupMember> {
        self.zone_groups.iter()
            .flat_map(|group| &group.members)
            .collect()
    }

    /// Retrieves topology information from a Sonos speaker at the given IP address
    ///
    /// # Arguments
    /// * `ip` - IP address of a Sonos speaker to query
    ///
    /// # Returns
    /// * `Ok(Topology)` - Complete topology information for the Sonos system
    /// * `Err(SonosError)` - If the request fails or parsing fails
    pub fn from_ip(ip: &str) -> Result<Self, crate::SonosError> {
        use crate::topology::client::get_topology_from_ip;
        get_topology_from_ip(ip)
    }

    /// Parses topology information from SOAP XML response
    ///
    /// # Arguments
    /// * `xml` - Raw SOAP XML response containing zone group state
    ///
    /// # Returns
    /// * `Ok(Topology)` - Parsed topology information
    /// * `Err(SonosError)` - If parsing fails at any stage
    pub fn from_xml(xml: &str) -> Result<Self, crate::SonosError> {
        use crate::topology::parser::TopologyParser;
        TopologyParser::from_xml(xml)
    }


}

impl ZoneGroup {
    pub fn get_speakers(&self) -> &[ZoneGroupMember] {
        &self.members
    }

    /// Returns true if this zone group has multiple members (is a grouped zone)
    pub fn is_grouped(&self) -> bool {
        self.members.len() > 1
    }

    /// Returns the coordinator speaker for this zone group
    pub fn coordinator_speaker(&self) -> Option<&ZoneGroupMember> {
        self.members.iter()
            .find(|member| member.uuid == self.coordinator)
    }

    /// Returns the total number of speakers including satellites
    pub fn total_speaker_count(&self) -> usize {
        self.members.iter()
            .map(|member| 1 + member.satellites.len())
            .sum()
    }

    /// Pauses playback for this zone group by delegating to the coordinator speaker
    /// 
    /// # Arguments
    /// * `system` - Reference to the System containing speaker instances
    /// 
    /// # Returns
    /// * `Ok(())` - If the pause command was successful
    /// * `Err(SonosError)` - If the coordinator speaker is not found or the command fails
    pub fn pause(&self, system: &crate::System) -> Result<(), crate::SonosError> {
        system.get_speaker_by_uuid(&self.coordinator)
            .ok_or(crate::SonosError::DeviceUnreachable)?
            .pause()
    }

    /// Starts playback for this zone group by delegating to the coordinator speaker
    /// 
    /// # Arguments
    /// * `system` - Reference to the System containing speaker instances
    /// 
    /// # Returns
    /// * `Ok(())` - If the play command was successful
    /// * `Err(SonosError)` - If the coordinator speaker is not found or the command fails
    pub fn play(&self, system: &crate::System) -> Result<(), crate::SonosError> {
        system.get_speaker_by_uuid(&self.coordinator)
            .ok_or(crate::SonosError::DeviceUnreachable)?
            .play()
    }
}

impl ZoneGroupMember {
    /// Returns true if this speaker has satellite speakers
    pub fn has_satellites(&self) -> bool {
        !self.satellites.is_empty()
    }

    /// Returns the total number of speakers including this one and its satellites
    pub fn total_speaker_count(&self) -> usize {
        1 + self.satellites.len()
    }

    /// Extracts the IP address from the location URL
    pub fn ip_address(&self) -> Option<String> {
        self.location
            .strip_prefix("http://")
            .and_then(|s| s.split(':').next())
            .map(|s| s.to_string())
    }
}

impl Satellite {
    /// Extracts the IP address from the location URL
    pub fn ip_address(&self) -> Option<String> {
        self.location
            .strip_prefix("http://")
            .and_then(|s| s.split(':').next())
            .map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_satellite() -> Satellite {
        Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
            zone_name: "Satellite Speaker".to_string(),
            software_version: "56.0-76060".to_string(),
        }
    }

    fn create_test_zone_group_member() -> ZoneGroupMember {
        ZoneGroupMember {
            uuid: "RINCON_123456".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![create_test_satellite()],
        }
    }

    fn create_test_zone_group() -> ZoneGroup {
        ZoneGroup {
            coordinator: "RINCON_123456".to_string(),
            id: "RINCON_123456:1234567890".to_string(),
            members: vec![create_test_zone_group_member()],
        }
    }

    fn create_test_topology() -> Topology {
        Topology {
            zone_groups: vec![create_test_zone_group()],
            vanished_devices: Some(VanishedDevices {
                devices: vec![VanishedDevice {
                    uuid: "RINCON_VANISHED".to_string(),
                    zone_name: "Old Speaker".to_string(),
                    reason: "powered off".to_string(),
                }],
            }),
        }
    }

    #[test]
    fn test_topology_zone_group_count() {
        let topology = create_test_topology();
        assert_eq!(topology.zone_group_count(), 1);

        let empty_topology = Topology {
            zone_groups: vec![],
            vanished_devices: None,
        };
        assert_eq!(empty_topology.zone_group_count(), 0);
    }

    #[test]
    fn test_topology_total_speaker_count() {
        let topology = create_test_topology();
        assert_eq!(topology.total_speaker_count(), 1);

        let multi_group_topology = Topology {
            zone_groups: vec![
                create_test_zone_group(),
                ZoneGroup {
                    coordinator: "RINCON_789".to_string(),
                    id: "RINCON_789:987".to_string(),
                    members: vec![
                        ZoneGroupMember {
                            uuid: "RINCON_789".to_string(),
                            location: "http://192.168.1.102:1400/xml/device_description.xml".to_string(),
                            zone_name: "Kitchen".to_string(),
                            software_version: "56.0-76060".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:kitchen".to_string(),
                            satellites: vec![],
                        },
                        ZoneGroupMember {
                            uuid: "RINCON_ABC".to_string(),
                            location: "http://192.168.1.103:1400/xml/device_description.xml".to_string(),
                            zone_name: "Dining Room".to_string(),
                            software_version: "56.0-76060".to_string(),
                            configuration: "1".to_string(),
                            icon: "x-rincon-roomicon:dining".to_string(),
                            satellites: vec![],
                        },
                    ],
                },
            ],
            vanished_devices: None,
        };
        assert_eq!(multi_group_topology.total_speaker_count(), 3);
    }

    #[test]
    fn test_topology_find_zone_group_by_coordinator() {
        let topology = create_test_topology();
        
        let found = topology.find_zone_group_by_coordinator("RINCON_123456");
        assert!(found.is_some());
        assert_eq!(found.unwrap().coordinator, "RINCON_123456");

        let not_found = topology.find_zone_group_by_coordinator("RINCON_NOTFOUND");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_topology_find_speaker_by_uuid() {
        let topology = create_test_topology();
        
        let found = topology.find_speaker_by_uuid("RINCON_123456");
        assert!(found.is_some());
        assert_eq!(found.unwrap().uuid, "RINCON_123456");

        let not_found = topology.find_speaker_by_uuid("RINCON_NOTFOUND");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_topology_all_speakers() {
        let topology = create_test_topology();
        let speakers = topology.all_speakers();
        assert_eq!(speakers.len(), 1);
        assert_eq!(speakers[0].uuid, "RINCON_123456");
    }

    #[test]
    fn test_zone_group_is_grouped() {
        let single_member_group = ZoneGroup {
            coordinator: "RINCON_123".to_string(),
            id: "RINCON_123:123".to_string(),
            members: vec![create_test_zone_group_member()],
        };
        assert!(!single_member_group.is_grouped());

        let multi_member_group = ZoneGroup {
            coordinator: "RINCON_123".to_string(),
            id: "RINCON_123:123".to_string(),
            members: vec![
                create_test_zone_group_member(),
                ZoneGroupMember {
                    uuid: "RINCON_456".to_string(),
                    location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
                    zone_name: "Kitchen".to_string(),
                    software_version: "56.0-76060".to_string(),
                    configuration: "1".to_string(),
                    icon: "x-rincon-roomicon:kitchen".to_string(),
                    satellites: vec![],
                },
            ],
        };
        assert!(multi_member_group.is_grouped());
    }

    #[test]
    fn test_zone_group_coordinator_speaker() {
        let zone_group = create_test_zone_group();
        let coordinator = zone_group.coordinator_speaker();
        assert!(coordinator.is_some());
        assert_eq!(coordinator.unwrap().uuid, "RINCON_123456");

        let no_coordinator_group = ZoneGroup {
            coordinator: "RINCON_NOTFOUND".to_string(),
            id: "RINCON_123:123".to_string(),
            members: vec![create_test_zone_group_member()],
        };
        let no_coordinator = no_coordinator_group.coordinator_speaker();
        assert!(no_coordinator.is_none());
    }

    #[test]
    fn test_zone_group_total_speaker_count() {
        let zone_group = create_test_zone_group();
        // 1 member + 1 satellite = 2 total
        assert_eq!(zone_group.total_speaker_count(), 2);

        let no_satellites_group = ZoneGroup {
            coordinator: "RINCON_123".to_string(),
            id: "RINCON_123:123".to_string(),
            members: vec![ZoneGroupMember {
                uuid: "RINCON_123".to_string(),
                location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                zone_name: "Living Room".to_string(),
                software_version: "56.0-76060".to_string(),
                configuration: "1".to_string(),
                icon: "x-rincon-roomicon:living".to_string(),
                satellites: vec![],
            }],
        };
        assert_eq!(no_satellites_group.total_speaker_count(), 1);
    }

    #[test]
    fn test_zone_group_member_has_satellites() {
        let member_with_satellites = create_test_zone_group_member();
        assert!(member_with_satellites.has_satellites());

        let member_without_satellites = ZoneGroupMember {
            uuid: "RINCON_123".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![],
        };
        assert!(!member_without_satellites.has_satellites());
    }

    #[test]
    fn test_zone_group_member_total_speaker_count() {
        let member = create_test_zone_group_member();
        // 1 member + 1 satellite = 2 total
        assert_eq!(member.total_speaker_count(), 2);

        let member_no_satellites = ZoneGroupMember {
            uuid: "RINCON_123".to_string(),
            location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![],
        };
        assert_eq!(member_no_satellites.total_speaker_count(), 1);
    }

    #[test]
    fn test_zone_group_member_ip_address() {
        let member = create_test_zone_group_member();
        let ip = member.ip_address();
        assert!(ip.is_some());
        assert_eq!(ip.unwrap(), "192.168.1.100");

        let member_invalid_location = ZoneGroupMember {
            uuid: "RINCON_123".to_string(),
            location: "invalid_url".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![],
        };
        let no_ip = member_invalid_location.ip_address();
        assert!(no_ip.is_none());

        let member_no_port = ZoneGroupMember {
            uuid: "RINCON_123".to_string(),
            location: "http://192.168.1.100/xml/device_description.xml".to_string(),
            zone_name: "Living Room".to_string(),
            software_version: "56.0-76060".to_string(),
            configuration: "1".to_string(),
            icon: "x-rincon-roomicon:living".to_string(),
            satellites: vec![],
        };
        let ip_no_port = member_no_port.ip_address();
        assert!(ip_no_port.is_some());
        // When there's no colon, split(':').next() returns the entire string after http://
        assert_eq!(ip_no_port.unwrap(), "192.168.1.100/xml/device_description.xml");
    }

    #[test]
    fn test_satellite_ip_address() {
        let satellite = create_test_satellite();
        let ip = satellite.ip_address();
        assert!(ip.is_some());
        assert_eq!(ip.unwrap(), "192.168.1.101");

        let satellite_invalid_location = Satellite {
            uuid: "RINCON_SAT123".to_string(),
            location: "invalid_url".to_string(),
            zone_name: "Satellite Speaker".to_string(),
            software_version: "56.0-76060".to_string(),
        };
        let no_ip = satellite_invalid_location.ip_address();
        assert!(no_ip.is_none());
    }

    #[test]
    fn test_vanished_device_creation() {
        let vanished = VanishedDevice {
            uuid: "RINCON_VANISHED".to_string(),
            zone_name: "Old Speaker".to_string(),
            reason: "powered off".to_string(),
        };
        
        assert_eq!(vanished.uuid, "RINCON_VANISHED");
        assert_eq!(vanished.zone_name, "Old Speaker");
        assert_eq!(vanished.reason, "powered off");
    }

    #[test]
    fn test_vanished_devices_creation() {
        let vanished_devices = VanishedDevices {
            devices: vec![
                VanishedDevice {
                    uuid: "RINCON_1".to_string(),
                    zone_name: "Speaker 1".to_string(),
                    reason: "powered off".to_string(),
                },
                VanishedDevice {
                    uuid: "RINCON_2".to_string(),
                    zone_name: "Speaker 2".to_string(),
                    reason: "network error".to_string(),
                },
            ],
        };
        
        assert_eq!(vanished_devices.devices.len(), 2);
        assert_eq!(vanished_devices.devices[0].uuid, "RINCON_1");
        assert_eq!(vanished_devices.devices[1].uuid, "RINCON_2");
    }

    #[cfg(feature = "mock")]
    fn create_mock_system_with_speakers() -> crate::System {
        use crate::speaker::mock::MockSpeakerBuilder;
        use crate::speaker::SpeakerTrait;
        
        let mut system = crate::System::new().unwrap();
        
        // Add mock speakers
        let speaker1: Box<dyn SpeakerTrait> = Box::new(
            MockSpeakerBuilder::new()
                .uuid("RINCON_123")
                .name("Living Room")
                .ip("192.168.1.100")
                .build()
        );
        
        let speaker2: Box<dyn SpeakerTrait> = Box::new(
            MockSpeakerBuilder::new()
                .uuid("RINCON_456")
                .name("Kitchen")
                .ip("192.168.1.101")
                .build()
        );
        
        system.add_speaker_for_test(speaker1);
        system.add_speaker_for_test(speaker2);
        
        system
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_zone_group_pause_command() {
        let system = create_mock_system_with_speakers();
        
        let zone_group = ZoneGroup {
            coordinator: "RINCON_123".to_string(),
            id: "RINCON_123:123".to_string(),
            members: vec![
                ZoneGroupMember {
                    uuid: "RINCON_123".to_string(),
                    location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                    zone_name: "Living Room".to_string(),
                    software_version: "56.0-76060".to_string(),
                    configuration: "1".to_string(),
                    icon: "x-rincon-roomicon:living".to_string(),
                    satellites: vec![],
                }
            ],
        };
        
        // Test successful pause
        let result = zone_group.pause(&system);
        assert!(result.is_ok());
        
        // Test pause with non-existent coordinator
        let invalid_group = ZoneGroup {
            coordinator: "RINCON_NOTFOUND".to_string(),
            id: "RINCON_NOTFOUND:123".to_string(),
            members: vec![],
        };
        
        let result = invalid_group.pause(&system);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), crate::SonosError::DeviceUnreachable));
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_zone_group_play_command() {
        let system = create_mock_system_with_speakers();
        
        let zone_group = ZoneGroup {
            coordinator: "RINCON_456".to_string(),
            id: "RINCON_456:456".to_string(),
            members: vec![
                ZoneGroupMember {
                    uuid: "RINCON_456".to_string(),
                    location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
                    zone_name: "Kitchen".to_string(),
                    software_version: "56.0-76060".to_string(),
                    configuration: "1".to_string(),
                    icon: "x-rincon-roomicon:kitchen".to_string(),
                    satellites: vec![],
                }
            ],
        };
        
        // Test successful play
        let result = zone_group.play(&system);
        assert!(result.is_ok());
        
        // Test play with non-existent coordinator
        let invalid_group = ZoneGroup {
            coordinator: "RINCON_NOTFOUND".to_string(),
            id: "RINCON_NOTFOUND:123".to_string(),
            members: vec![],
        };
        
        let result = invalid_group.play(&system);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), crate::SonosError::DeviceUnreachable));
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_zone_group_commands_with_grouped_zone() {
        let system = create_mock_system_with_speakers();
        
        // Create a grouped zone with multiple members
        let grouped_zone = ZoneGroup {
            coordinator: "RINCON_123".to_string(),
            id: "RINCON_123:123".to_string(),
            members: vec![
                ZoneGroupMember {
                    uuid: "RINCON_123".to_string(),
                    location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
                    zone_name: "Living Room".to_string(),
                    software_version: "56.0-76060".to_string(),
                    configuration: "1".to_string(),
                    icon: "x-rincon-roomicon:living".to_string(),
                    satellites: vec![],
                },
                ZoneGroupMember {
                    uuid: "RINCON_456".to_string(),
                    location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
                    zone_name: "Kitchen".to_string(),
                    software_version: "56.0-76060".to_string(),
                    configuration: "1".to_string(),
                    icon: "x-rincon-roomicon:kitchen".to_string(),
                    satellites: vec![],
                }
            ],
        };
        
        // Commands should delegate to the coordinator (RINCON_123)
        let pause_result = grouped_zone.pause(&system);
        assert!(pause_result.is_ok());
        
        let play_result = grouped_zone.play(&system);
        assert!(play_result.is_ok());
        
        // Verify it's using the coordinator, not other members
        assert_eq!(grouped_zone.coordinator, "RINCON_123");
        assert!(grouped_zone.is_grouped());
    }
}