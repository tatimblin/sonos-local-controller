use xmltree::Element;

use crate::{topology::types::ZoneGroup, util::http::get_ip_from_url, Topology, ZoneGroupMember};

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
  pub fn from_element(element: &Element) -> Result<Self, crate::SonosError> {
    let mut buffer = Vec::new();
    element.write(&mut buffer).unwrap();
    let xml_string = String::from_utf8(buffer).unwrap();
    log::debug!("\nXML String:\n{}", xml_string);
    let coordinator = element.attributes.get("Coordinator")
      .ok_or(crate::SonosError::ParseError("Missing Coordinator attribute".to_string()))?
      .clone();

    let id = element.attributes.get("ID")
      .ok_or(crate::SonosError::ParseError("Missing ID attribute".to_string()))?
      .clone();

    let mut members = Vec::new();
    for child in &element.children {
      if let Some(member_element) = child.as_element() {
        if member_element.name == "ZoneGroupMember" {
          members.push(ZoneGroupMember::from_element(member_element)?);
        }
      }
    }

    Ok(ZoneGroup { coordinator, id, members })
  }

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

    pub fn count_children(&self) -> usize {
      self.members.len()
    }

  pub fn get_children(&self) -> Vec<(String, String)> {
    self.members.iter().map(|member| (member.get_ip(), member.get_uuid())).collect()
  }
}

impl ZoneGroupMember {
  pub fn from_element(element: &Element) -> Result<Self, crate::SonosError> {
    let uuid = element.attributes.get("UUID")
      .ok_or(crate::SonosError::ParseError("Missing UUID attribute".to_string()))?
      .clone();

    let location = element.attributes.get("Location")
      .ok_or(crate::SonosError::ParseError("Missing Location attribute".to_string()))?
      .clone();

    Ok(ZoneGroupMember {
      uuid,
      location: location.clone(),
      zone_name: element.attributes.get("ZoneName").unwrap_or(&String::new()).clone(),
      icon: element.attributes.get("Icon").unwrap_or(&String::new()).clone(),
      configuration: element.attributes.get("Configuration").unwrap_or(&String::new()).clone(),
      software_version: element.attributes.get("SoftwareVersion").unwrap_or(&String::new()).clone(),
      satellites: Vec::new(),
    })
  }

  pub fn get_ip(&self) -> String {
    self.location // (e.g., "http://192.168.4.65:1400/xml/device_description.xml")
      .strip_prefix("http://")
      .and_then(|url| url.split(':').next())
      .map(|ip| ip.to_string())
      .unwrap_or_else(|| "unknown".to_string())
  }

  pub fn get_uuid(&self) -> String {
    self.uuid.clone()
  }
}

// impl Satellite {}
