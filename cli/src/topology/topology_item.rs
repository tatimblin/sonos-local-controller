use sonos::{Satellite, SpeakerInfo, ZoneGroup, ZoneGroupMember};

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyItem {
    Group {
        ip: String,
        name: String,
        uuid: String,
    },
    Speaker {
        ip: String,
        name: String,
        uuid: String,
    },
    Satellite {
        uuid: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopologyType {
    Group,
    Speaker,
    Satellite,
}

impl TopologyItem {
    pub fn from_group(group: &ZoneGroup) -> Self {
        TopologyItem::Group {
            ip: group.get_coordinator().get_ip(),
            uuid: group.id.to_string(),
            name: group.get_name().to_string(),
        }
    }

    pub fn from_speaker(speaker: &ZoneGroupMember) -> Self {
        TopologyItem::Speaker {
            ip: speaker.get_ip(),
            name: speaker.zone_name.to_string(),
            uuid: speaker.uuid.to_string(),
        }
    }

    pub fn from_satellite(satellite: &Satellite) -> Self {
        TopologyItem::Satellite {
            uuid: satellite.uuid.to_string(),
        }
    }

    pub fn get_type(&self) -> TopologyType {
        match self {
            TopologyItem::Group { .. } => TopologyType::Group,
            TopologyItem::Speaker { .. } => TopologyType::Speaker,
            TopologyItem::Satellite { .. } => TopologyType::Satellite,
        }
    }

    pub fn get_uuid(&self) -> &str {
        match self {
            TopologyItem::Group { uuid, .. }
            | TopologyItem::Speaker { uuid, .. }
            | TopologyItem::Satellite { uuid } => uuid,
        }
    }
}

#[cfg(test)]
mod tests {
    use sonos::{Topology, VanishedDevice, VanishedDevices};

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
    fn test_from_group() {
        let zone_group = create_test_zone_group();

        let group = TopologyItem::from_group(&zone_group);

        assert_eq!(zone_group.id, group.get_uuid());
    }

    #[test]
    fn test_from_speaker() {
        let zone_group_member = create_test_zone_group_member();

        let speaker = TopologyItem::from_speaker(&zone_group_member);

        assert_eq!(zone_group_member.uuid, speaker.get_uuid());
    }

    #[test]
    fn test_from_satellite() {
        let satellite = create_test_satellite();

        let satellite_item = TopologyItem::from_satellite(&satellite);

        assert_eq!(satellite.uuid, satellite_item.get_uuid());
    }
}
