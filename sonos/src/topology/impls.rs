use crate::{topology::types::ZoneGroup, Topology, ZoneGroupMember};

impl Topology {
    pub fn from_ip(ip: &str) -> Result<Self, crate::SonosError> {
        use crate::topology::client::get_topology_from_ip;
        get_topology_from_ip(ip)
    }

    pub fn from_xml(xml: &str) -> Result<Self, crate::SonosError> {
        use crate::topology::parser::TopologyParser;
        TopologyParser::from_xml(xml)
    }

    pub fn get_groups(&self) -> &Vec<ZoneGroup> {
        &self.zone_groups
    }

    pub fn len(&self) -> usize {
        self.zone_groups.len()
    }
}

impl ZoneGroup {
    pub fn get_name(&self) -> &str {
        self.members
            .first()
            .map(|member| member.zone_name.as_str())
            .unwrap_or("Unknown Group")
    }

    pub fn get_coordinator(&self) -> &ZoneGroupMember {
        self.members
            .iter()
            .find(|member| member.uuid == self.coordinator)
            .expect("Coordinator must exist in zone group")
    }

    pub fn get_speakers(&self) -> &Vec<ZoneGroupMember> {
        &self.members
    }
}

impl ZoneGroupMember {
  pub fn get_ip(&self) -> String {
    self.location // (e.g., "http://192.168.4.65:1400/xml/device_description.xml")
      .strip_prefix("http://")
      .and_then(|url| url.split(':').next())
      .map(|ip| ip.to_string())
      .unwrap_or_else(|| "unknown".to_string())
  }
}

// impl Satellite {}
