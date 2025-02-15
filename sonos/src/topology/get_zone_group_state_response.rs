use serde_derive::{ Deserialize, Serialize };
use xmltree::Element;

use crate::{model::Action, Client, SonosError};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetZoneGroupStateResponse {
    #[serde(rename = "ZoneGroupState")]
    pub zone_group_state: ZoneGroupStateWrapper,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ZoneGroupStateWrapper {
    #[serde(rename = "ZoneGroupState")]
    pub zone_group_state: ZoneGroupState,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ZoneGroupState {
    pub zone_groups: ZoneGroups,
    vanished_devices: Option<VanishedDevices>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ZoneGroups {
    pub zone_group: Vec<ZoneGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ZoneGroup {
    #[serde(rename = "Coordinator")]
    coordinator: String,
    #[serde(rename = "ID")]
    pub id: String,
    pub zone_group_member: Vec<ZoneGroupMember>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ZoneGroupMember {
    #[serde(rename = "UUID")]
    pub uuid: String,
    location: String,
    pub zone_name: String,
    icon: String,
    configuration: u8,
    software_version: String,
    #[serde(rename = "SWGen")]
    sw_gen: Option<String>,
    min_compatible_version: String,
    legacy_compatible_version: String,
    #[serde(rename = "HTSatChanMapSet")]
    ht_sat_chan_map_set: Option<String>,
    #[serde(rename = "ActiveZoneID")]
    active_zone_id: Option<String>,
    boot_seq: String,
    #[serde(rename = "TVConfigurationError")]
    tv_configuration_error: Option<String>,
    hdmi_cec_available: Option<String>,
    wireless_mode: Option<String>,
    wireless_leaf_only: Option<String>,
    channel_freq: Option<String>,
    behind_wifi_extender: Option<String>,
    wifi_enabled: Option<String>,
    eth_link: Option<String>,
    orientation: Option<String>,
    room_calibration_state: Option<String>,
    secure_reg_state: Option<String>,
    voice_config_state: Option<String>,
    mic_enabled: Option<String>,
    headphone_swap_active: Option<String>,
    airplay_enabled: Option<String>,
    virtual_line_in_source: Option<String>,
    idle_state: Option<String>,
    more_info: Option<String>,
    #[serde(rename = "SSLPort")]
    ssl_port: Option<String>,
    #[serde(rename = "HHSSLPort")]
    hhssl_port: Option<String>,
    #[serde(default)]
    satellite: Vec<Satellite>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Satellite {
    #[serde(rename = "UUID")]
    uuid: String,
    location: String,
    zone_name: String,
    icon: String,
    configuration: String,
    invisible: String,
    software_version: String,
    #[serde(rename = "SWGen")]
    sw_gen: String,
    min_compatible_version: String,
    legacy_compatible_version: String,
    #[serde(rename = "HTSatChanMapSet")]
    ht_sat_chan_map_set: String,
    #[serde(rename = "ActiveZoneID")]
    active_zone_id: String,
    boot_seq: String,
    tv_configuration_error: String,
    hdmi_cec_available: String,
    wireless_mode: String,
    wireless_leaf_only: String,
    channel_freq: String,
    behind_wifi_extender: String,
    wifi_enabled: String,
    eth_link: String,
    orientation: String,
    room_calibration_state: String,
    secure_reg_state: String,
    voice_config_state: String,
    mic_enabled: String,
    headphone_swap_active: String,
    airplay_enabled: String,
    idle_state: String,
    more_info: String,
    #[serde(rename = "SSLPort")]
    ssl_port: String,
    #[serde(rename = "HHSSLPort")]
    hhssl_port: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VanishedDevices {}

impl GetZoneGroupStateResponse {
  pub fn from_ip(ip: &str) -> Result<Self, SonosError> {
    let client = Client::default();
    let payload = "<InstanceID>0</InstanceID>";

    match client.send_action(ip, Action::GetZoneGroupState, payload) {
      Ok(response) => match Self::from_xml(&element_to_str(&response)) {
        Ok(get_zone_group_state_response) => Ok(get_zone_group_state_response),
        Err(e) => Err(e),
      },
      Err(e) => {
        println!("Failed to parse GetZoneGroupStateResponse 2: {}", e);
        Err(e)
      }
    }
  }

  fn from_xml(xml: &str) -> Result<Self, SonosError> {
    serde_xml_rs::from_str(xml)
      .map_err(|e| SonosError::ParseError(format!("Failed to parse GetZoneGroupStateResponse: {}", e)))
  }
}

fn element_to_str(element: &Element) -> String {
  let mut buffer = Vec::new();
  element
    .write(&mut buffer)
    .expect("Failed to write XML element");
  String::from_utf8_lossy(&buffer).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_XML: &str = r#"
    <u:GetZoneGroupStateResponse xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
                                 xmlns:u="urn:schemas-upnp-org:service:ZoneGroupTopology:1">
      <ZoneGroupState>
        <ZoneGroupState>
          <ZoneGroups>
            <ZoneGroup Coordinator="RINCON_B8E9375831C001400" ID="RINCON_B8E9375831C001400:0">
              <ZoneGroupMember 
                UUID="RINCON_B8E9375831C001400" 
                Location="http://192.168.1.100:1400/xml/device_description.xml" 
                ZoneName="Living Room" 
                Icon="x-rincon-roomicon:living" 
                Configuration="1" 
                SoftwareVersion="77.4-40060" 
                MinCompatibleVersion="76.0-00000" 
                LegacyCompatibleVersion="36.0-00000" 
                BootSeq="110"
              />
            </ZoneGroup>
          </ZoneGroups>
        </ZoneGroupState>
      </ZoneGroupState>
    </u:GetZoneGroupStateResponse>
  "#;

  #[test]
  fn test_topology_from_xml() {
    let get_zone_group_state_response = GetZoneGroupStateResponse::from_xml(SAMPLE_XML).unwrap();

    // Test basic structure
    assert_eq!(get_zone_group_state_response.zone_group_state.zone_group_state.zone_groups.zone_group.len(), 1);

    // Test zone group details
    let group = &get_zone_group_state_response.zone_group_state.zone_group_state.zone_groups.zone_group[0];
    assert_eq!(group.coordinator, "RINCON_B8E9375831C001400");
    assert_eq!(group.id, "RINCON_B8E9375831C001400:0");

    // Test zone group member details
    let member = &group.zone_group_member[0];
    assert_eq!(member.uuid, "RINCON_B8E9375831C001400");
    assert_eq!(member.zone_name, "Living Room");
    assert_eq!(
      member.location,
      "http://192.168.1.100:1400/xml/device_description.xml"
    );
    assert_eq!(member.icon, "x-rincon-roomicon:living");

    assert_eq!(member.configuration, 1);
    assert_eq!(member.software_version, "77.4-40060");
    assert_eq!(member.min_compatible_version, "76.0-00000");
    assert_eq!(member.legacy_compatible_version, "36.0-00000");
    assert_eq!(member.boot_seq, "110");
  }

  #[test]
  fn test_topology_from_xml_multiple_speakers() {
    let xml = r#"
      <u:GetZoneGroupStateResponse xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
                                  xmlns:u="urn:schemas-upnp-org:service:ZoneGroupTopology:1">
        <ZoneGroupState>
          <ZoneGroupState>
            <ZoneGroups>
              <ZoneGroup Coordinator="RINCON_B8E9375831C001400" ID="RINCON_B8E9375831C001400:0">
                <ZoneGroupMember 
                  UUID="RINCON_B8E9375831C001400" 
                  Location="http://192.168.1.100:1400/xml/device_description.xml" 
                  ZoneName="Living Room" 
                  Icon="x-rincon-roomicon:living" 
                  Configuration="1" 
                  SoftwareVersion="77.4-40060" 
                  MinCompatibleVersion="76.0-00000" 
                  LegacyCompatibleVersion="36.0-00000" 
                  BootSeq="110"
                />
                <ZoneGroupMember 
                  UUID="RINCON_B8E9375831C001401" 
                  Location="http://192.168.1.101:1400/xml/device_description.xml" 
                  ZoneName="Kitchen" 
                  Icon="x-rincon-roomicon:kitchen" 
                  Configuration="1" 
                  SoftwareVersion="77.4-40060" 
                  MinCompatibleVersion="76.0-00000" 
                  LegacyCompatibleVersion="36.0-00000" 
                  BootSeq="111"
                />
              </ZoneGroup>
            </ZoneGroups>
          </ZoneGroupState>
        </ZoneGroupState>
      </u:GetZoneGroupStateResponse>
    "#;

    let get_zone_group_state_response = GetZoneGroupStateResponse::from_xml(xml).unwrap();

    // Test multiple members in group
    assert_eq!(get_zone_group_state_response.zone_group_state.zone_group_state.zone_groups.zone_group[0].zone_group_member.len(), 2);
    assert_eq!(get_zone_group_state_response.zone_group_state.zone_group_state.zone_groups.zone_group[0].zone_group_member[1].zone_name, "Kitchen");
  }

  #[test]
  fn test_topology_from_xml_multiple_groups() {
    let xml = r#"
      <u:GetZoneGroupStateResponse xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
                                   xmlns:u="urn:schemas-upnp-org:service:ZoneGroupTopology:1">
        <ZoneGroupState>
          <ZoneGroupState>
            <ZoneGroups>
              <ZoneGroup Coordinator="RINCON_B8E9375831C001400" ID="RINCON_B8E9375831C001400:0">
              <ZoneGroupMember 
                UUID="RINCON_B8E9375831C001400" 
                Location="http://192.168.1.100:1400/xml/device_description.xml" 
                ZoneName="Living Room" 
                Icon="x-rincon-roomicon:living" 
                Configuration="1" 
                SoftwareVersion="77.4-40060" 
                MinCompatibleVersion="76.0-00000" 
                LegacyCompatibleVersion="36.0-00000" 
                BootSeq="110"
              />
            </ZoneGroup>
            <ZoneGroup Coordinator="RINCON_B8E9375831C001401" ID="RINCON_B8E9375831C001401:1">
              <ZoneGroupMember 
                UUID="RINCON_B8E9375831C001401" 
                Location="http://192.168.1.101:1400/xml/device_description.xml" 
                ZoneName="Kitchen" 
                Icon="x-rincon-roomicon:kitchen" 
                Configuration="1" 
                SoftwareVersion="77.4-40060" 
                MinCompatibleVersion="76.0-00000" 
                LegacyCompatibleVersion="36.0-00000" 
                BootSeq="111"
              />
              </ZoneGroup>
            </ZoneGroups>
          </ZoneGroupState>
        </ZoneGroupState>
      </u:GetZoneGroupStateResponse>
    "#;

    let get_zone_group_state_response = GetZoneGroupStateResponse::from_xml(xml).unwrap();

    // Test multiple groups
    assert_eq!(get_zone_group_state_response.zone_group_state.zone_group_state.zone_groups.zone_group.len(), 2);
    assert_eq!(get_zone_group_state_response.zone_group_state.zone_group_state.zone_groups.zone_group[0].coordinator, "RINCON_B8E9375831C001400");
    assert_eq!(get_zone_group_state_response.zone_group_state.zone_group_state.zone_groups.zone_group[1].coordinator, "RINCON_B8E9375831C001401");
  }

  #[test]
  fn test_topology_from_xml_malformed() {
      let malformed_xml = r#"
        <u:GetZoneGroupStateResponse xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
                                     xmlns:u="urn:schemas-upnp-org:service:ZoneGroupTopology:1">
          <ZoneGroupState>
            <ZoneGroupState>
              <ZoneGroups>
                <ZoneGroup>
                  <InvalidTag>
                </ZoneGroup>
              </ZoneGroups>
            </ZoneGroupState>
          </ZoneGroupState>
        </u:GetZoneGroupStateResponse>
      "#;

      let result = GetZoneGroupStateResponse::from_xml(malformed_xml);
      assert!(result.is_err());
  }
}
