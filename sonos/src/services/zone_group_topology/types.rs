use serde::{Deserialize, Serialize};

/// XML data structure for zone group information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlZoneGroupData {
    #[serde(rename = "@Coordinator")]
    pub coordinator: String,
    #[serde(rename = "ZoneGroupMember", default)]
    pub members: Vec<XmlZoneGroupMember>,
}

/// XML data structure for zone group member information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlZoneGroupMember {
    #[serde(rename = "@UUID")]
    pub uuid: String,
    #[serde(rename = "@Satellites", default)]
    pub satellites_attr: Option<String>,
    #[serde(rename = "Satellite", default)]
    pub satellite_elements: Vec<XmlSatellite>,
}

impl XmlZoneGroupMember {
    /// Get all satellites as a unified list
    pub fn satellites(&self) -> Vec<String> {
        let mut satellites = Vec::new();
        
        // Add satellites from attribute (comma-separated)
        if let Some(ref attr) = self.satellites_attr {
            for uuid in attr.split(',') {
                let uuid = uuid.trim();
                if !uuid.is_empty() {
                    satellites.push(uuid.to_string());
                }
            }
        }
        
        // Add satellites from nested elements
        for satellite in &self.satellite_elements {
            satellites.push(satellite.uuid.clone());
        }
        
        satellites
    }
}

/// XML data structure for satellite speakers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlSatellite {
    #[serde(rename = "@UUID")]
    pub uuid: String,
}

/// Root structure for zone group topology
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "ZoneGroups")]
pub struct XmlZoneGroups {
    #[serde(rename = "ZoneGroup", default)]
    pub zone_groups: Vec<XmlZoneGroupData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_zone_group_data() {
        let member = XmlZoneGroupMember {
            uuid: "RINCON_123456789".to_string(),
            satellites_attr: Some("RINCON_987654321".to_string()),
            satellite_elements: vec![],
        };
        assert_eq!(member.uuid, "RINCON_123456789");
        assert_eq!(member.satellites().len(), 1);

        let zone_group = XmlZoneGroupData {
            coordinator: "RINCON_123456789".to_string(),
            members: vec![member],
        };
        assert_eq!(zone_group.coordinator, "RINCON_123456789");
        assert_eq!(zone_group.members.len(), 1);
    }

    #[test]
    fn test_xml_satellite() {
        let satellite = XmlSatellite {
            uuid: "RINCON_987654321".to_string(),
        };
        assert_eq!(satellite.uuid, "RINCON_987654321");
    }

    #[test]
    fn test_xml_zone_groups() {
        let zone_groups = XmlZoneGroups {
            zone_groups: vec![],
        };
        assert_eq!(zone_groups.zone_groups.len(), 0);
    }
}