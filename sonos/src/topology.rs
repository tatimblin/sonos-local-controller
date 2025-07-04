use log::{debug, error, info};
use serde_derive::Deserialize;
use std::fs::OpenOptions;
use std::io::Write;
use xmltree::Element;

use crate::{model::Action, Client, SonosError};

// Constants for XML element and attribute names
const ZONE_GROUPS_ELEMENT: &str = "ZoneGroups";
const ZONE_GROUP_ELEMENT: &str = "ZoneGroup";
const ZONE_GROUP_MEMBER_ELEMENT: &str = "ZoneGroupMember";
const SATELLITE_ELEMENT: &str = "Satellite";
const VANISHED_DEVICES_ELEMENT: &str = "VanishedDevices";
const DEVICE_ELEMENT: &str = "Device";

const COORDINATOR_ATTR: &str = "Coordinator";
const ID_ATTR: &str = "ID";
const UUID_ATTR: &str = "UUID";
const LOCATION_ATTR: &str = "Location";
const ZONE_NAME_ATTR: &str = "ZoneName";
const SOFTWARE_VERSION_ATTR: &str = "SoftwareVersion";
const CONFIGURATION_ATTR: &str = "Configuration";
const ICON_ATTR: &str = "Icon";
const REASON_ATTR: &str = "Reason";

/// Response structure for the outer SOAP envelope containing the zone group state
#[derive(Debug, Deserialize)]
#[serde(rename = "GetZoneGroupStateResponse")]
struct GetZoneGroupStateResponse {
    #[serde(rename = "ZoneGroupState")]
    zone_group_state: String, // HTML-encoded XML content
}

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
    pub fn from_ip(ip: &str) -> Result<Self, SonosError> {
        info!("Starting topology retrieval from IP: {}", ip);
        
        let client = Client::default();
        let payload = "<InstanceID>0</InstanceID>";
        debug!("Using payload: {}", payload);

        info!("Sending GetZoneGroupState action to {}...", ip);
        let response = client.send_action(ip, Action::GetZoneGroupState, payload)
            .map_err(|e| {
                error!("Failed to send action to {}: {:?}", ip, e);
                e
            })?;

        info!("Successfully received response from {}", ip);
        
        // Log raw response for debugging (optional, can be disabled in production)
        if cfg!(debug_assertions) {
            Self::log_raw_response(&response);
        }

        let response_str = element_to_str(&response);
        debug!("Response XML length: {} characters", response_str.len());

        info!("Parsing XML response...");
        let topology = Self::from_xml(&response_str)
            .map_err(|e| {
                error!("Failed to parse XML: {:?}", e);
                e
            })?;

        info!("Successfully parsed topology with {} zone groups", topology.zone_groups.len());
        Ok(topology)
    }

    /// Logs the raw XML response for debugging purposes
    fn log_raw_response(response: &Element) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("../log.txt") {
            let response_str = element_to_str(response);
            let _ = file.write_all(response_str.as_bytes());
            let _ = file.write_all(b"\n--- END RESPONSE ---\n");
        }
    }

    /// Parses topology information from SOAP XML response
    ///
    /// # Arguments
    /// * `xml` - Raw SOAP XML response containing zone group state
    ///
    /// # Returns
    /// * `Ok(Topology)` - Parsed topology information
    /// * `Err(SonosError)` - If parsing fails at any stage
    pub fn from_xml(xml: &str) -> Result<Self, SonosError> {
        debug!("Starting XML parsing...");
        debug!("Input XML length: {} characters", xml.len());
        
        // Parse the outer SOAP response to extract the inner XML
        let decoded_xml = Self::extract_inner_xml(xml)?;
        
        // Write decoded XML for debugging (only in debug builds)
        if cfg!(debug_assertions) {
            Self::write_debug_xml(&decoded_xml);
        }
        
        // Parse the inner XML using xmltree
        debug!("Parsing inner XML with xmltree...");
        let topology = Self::parse_topology_xml(&decoded_xml)?;
        
        info!("Successfully parsed topology with {} zone groups", topology.zone_groups.len());
        Ok(topology)
    }

    /// Extracts and decodes the inner XML from the SOAP response
    fn extract_inner_xml(xml: &str) -> Result<String, SonosError> {
        debug!("Parsing outer SOAP response...");
        let outer_response: GetZoneGroupStateResponse = serde_xml_rs::from_str(xml)
            .map_err(|e| {
                error!("Failed to parse outer SOAP response: {}", e);
                SonosError::ParseError(format!("Failed to parse outer response: {}", e))
            })?;
        
        debug!("Successfully parsed outer response");
        debug!("Zone group state length: {} characters", outer_response.zone_group_state.len());
        
        // Decode HTML entities in the inner XML
        debug!("Decoding HTML entities...");
        let decoded_xml = html_escape::decode_html_entities(&outer_response.zone_group_state).to_string();
        debug!("Decoded XML length: {} characters", decoded_xml.len());
        debug!("First 200 chars of decoded XML: {}", 
                 if decoded_xml.len() > 200 { &decoded_xml[..200] } else { &decoded_xml });
        
        Ok(decoded_xml)
    }

    /// Writes decoded XML to file for debugging purposes
    fn write_debug_xml(xml: &str) {
        if let Ok(mut debug_file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("../decoded_topology.xml") {
            let _ = debug_file.write_all(xml.as_bytes());
        }
    }
  
    /// Parses the topology XML using xmltree for manual parsing
    fn parse_topology_xml(xml: &str) -> Result<Self, SonosError> {
        let root = Element::parse(xml.as_bytes())
            .map_err(|e| SonosError::ParseError(format!("Failed to parse XML with xmltree: {}", e)))?;
        
        let zone_groups = Self::parse_zone_groups(&root)?;
        let vanished_devices = Self::parse_vanished_devices(&root);
        
        Ok(Topology {
            zone_groups,
            vanished_devices,
        })
    }

    /// Parses zone groups from the XML root element
    fn parse_zone_groups(root: &Element) -> Result<Vec<ZoneGroup>, SonosError> {
        let mut zone_groups = Vec::new();
        
        if let Some(zone_groups_elem) = root.get_child(ZONE_GROUPS_ELEMENT) {
            for zone_group_elem in zone_groups_elem.children.iter() {
                if let Some(element) = zone_group_elem.as_element() {
                    if element.name == ZONE_GROUP_ELEMENT {
                        let zone_group = Self::parse_zone_group(element)?;
                        zone_groups.push(zone_group);
                    }
                }
            }
        }
        
        Ok(zone_groups)
    }

    /// Parses a single zone group element
    fn parse_zone_group(element: &Element) -> Result<ZoneGroup, SonosError> {
        let coordinator = Self::get_attribute(element, COORDINATOR_ATTR);
        let id = Self::get_attribute(element, ID_ATTR);
        let members = Self::parse_zone_group_members(element)?;
        
        Ok(ZoneGroup {
            coordinator,
            id,
            members,
        })
    }

    /// Parses zone group members from a zone group element
    fn parse_zone_group_members(zone_group_elem: &Element) -> Result<Vec<ZoneGroupMember>, SonosError> {
        let mut members = Vec::new();
        
        for member_elem in zone_group_elem.children.iter() {
            if let Some(member_element) = member_elem.as_element() {
                if member_element.name == ZONE_GROUP_MEMBER_ELEMENT {
                    let member = Self::parse_zone_group_member(member_element)?;
                    members.push(member);
                }
            }
        }
        
        Ok(members)
    }

    /// Parses a single zone group member element
    fn parse_zone_group_member(element: &Element) -> Result<ZoneGroupMember, SonosError> {
        let uuid = Self::get_attribute(element, UUID_ATTR);
        let location = Self::get_attribute(element, LOCATION_ATTR);
        let zone_name = Self::get_attribute(element, ZONE_NAME_ATTR);
        let software_version = Self::get_attribute(element, SOFTWARE_VERSION_ATTR);
        let configuration = Self::get_attribute(element, CONFIGURATION_ATTR);
        let icon = Self::get_attribute(element, ICON_ATTR);
        let satellites = Self::parse_satellites(element)?;
        
        Ok(ZoneGroupMember {
            uuid,
            location,
            zone_name,
            software_version,
            configuration,
            icon,
            satellites,
        })
    }

    /// Parses satellite speakers from a zone group member element
    fn parse_satellites(member_element: &Element) -> Result<Vec<Satellite>, SonosError> {
        let mut satellites = Vec::new();
        
        for satellite_elem in member_element.children.iter() {
            if let Some(satellite_element) = satellite_elem.as_element() {
                if satellite_element.name == SATELLITE_ELEMENT {
                    let satellite = Self::parse_satellite(satellite_element)?;
                    satellites.push(satellite);
                }
            }
        }
        
        Ok(satellites)
    }

    /// Parses a single satellite element
    fn parse_satellite(element: &Element) -> Result<Satellite, SonosError> {
        let uuid = Self::get_attribute(element, UUID_ATTR);
        let location = Self::get_attribute(element, LOCATION_ATTR);
        let zone_name = Self::get_attribute(element, ZONE_NAME_ATTR);
        let software_version = Self::get_attribute(element, SOFTWARE_VERSION_ATTR);
        
        Ok(Satellite {
            uuid,
            location,
            zone_name,
            software_version,
        })
    }

    /// Parses vanished devices from the XML root element (optional)
    fn parse_vanished_devices(root: &Element) -> Option<VanishedDevices> {
        root.get_child(VANISHED_DEVICES_ELEMENT).map(|vanished_elem| {
            let mut devices = Vec::new();
            
            for device_elem in vanished_elem.children.iter() {
                if let Some(device_element) = device_elem.as_element() {
                    if device_element.name == DEVICE_ELEMENT {
                        let uuid = Self::get_attribute(device_element, UUID_ATTR);
                        let zone_name = Self::get_attribute(device_element, ZONE_NAME_ATTR);
                        let reason = Self::get_attribute(device_element, REASON_ATTR);
                        
                        devices.push(VanishedDevice {
                            uuid,
                            zone_name,
                            reason,
                        });
                    }
                }
            }
            
            VanishedDevices { devices }
        })
    }

    /// Helper function to safely get an attribute value, returning empty string if not found
    fn get_attribute(element: &Element, attr_name: &str) -> String {
        element.attributes.get(attr_name).cloned().unwrap_or_default()
    }
}

impl ZoneGroup {
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

/// Converts an XML element to a string representation
fn element_to_str(element: &Element) -> String {
    let mut buffer = Vec::new();
    element.write(&mut buffer).expect("Failed to write XML element");
    String::from_utf8_lossy(&buffer).into_owned()
}
