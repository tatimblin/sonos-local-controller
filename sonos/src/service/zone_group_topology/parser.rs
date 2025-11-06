use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename = "propertyset")]
pub struct ZoneGroupTopologyParser {
    #[serde(rename = "property")]
    pub zone_group_state: Property,
}

#[derive(Debug, Deserialize)]
pub struct Property {
    #[serde(
        rename = "ZoneGroupState",
        deserialize_with = "crate::xml_decode::xml_decode::deserialize_nested"
    )]
    pub zone_group_state: ZoneGroupState,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "ZoneGroupState")]
pub struct ZoneGroupState {
    #[serde(rename = "ZoneGroups")]
    pub zone_groups: ZoneGroups,
    #[serde(rename = "VanishedDevices")]
    pub vanished_devices: VanishedDevices,
}

#[derive(Debug, Deserialize)]
pub struct ZoneGroups {
    #[serde(rename = "ZoneGroup", default)]
    pub zone_groups: Vec<ZoneGroup>,
}

#[derive(Debug, Deserialize)]
pub struct ZoneGroup {
    #[serde(rename = "@Coordinator")]
    pub coordinator: String,
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "ZoneGroupMember", default)]
    pub zone_group_members: Vec<ZoneGroupMember>,
}

#[derive(Debug, Deserialize)]
pub struct ZoneGroupMember {
    #[serde(rename = "@UUID")]
    pub uuid: String,
    #[serde(rename = "@Location")]
    pub location: String,
    #[serde(rename = "@ZoneName")]
    pub zone_name: String,
    #[serde(rename = "@Icon")]
    pub icon: String,
    #[serde(rename = "@Configuration")]
    pub configuration: String,
    #[serde(rename = "@SoftwareVersion")]
    pub software_version: String,
    #[serde(rename = "@SWGen")]
    pub sw_gen: String,
    #[serde(rename = "@MinCompatibleVersion")]
    pub min_compatible_version: String,
    #[serde(rename = "@LegacyCompatibleVersion")]
    pub legacy_compatible_version: String,
    #[serde(rename = "@HTSatChanMapSet", default)]
    pub ht_sat_chan_map_set: Option<String>,
    #[serde(rename = "@ActiveZoneID", default)]
    pub active_zone_id: Option<String>,
    #[serde(rename = "@BootSeq")]
    pub boot_seq: String,
    #[serde(rename = "@TVConfigurationError")]
    pub tv_configuration_error: String,
    #[serde(rename = "@HdmiCecAvailable")]
    pub hdmi_cec_available: String,
    #[serde(rename = "@WirelessMode")]
    pub wireless_mode: String,
    #[serde(rename = "@WirelessLeafOnly")]
    pub wireless_leaf_only: String,
    #[serde(rename = "@ChannelFreq")]
    pub channel_freq: String,
    #[serde(rename = "@BehindWifiExtender")]
    pub behind_wifi_extender: String,
    #[serde(rename = "@WifiEnabled")]
    pub wifi_enabled: String,
    #[serde(rename = "@EthLink")]
    pub eth_link: String,
    #[serde(rename = "@Orientation")]
    pub orientation: String,
    #[serde(rename = "@RoomCalibrationState")]
    pub room_calibration_state: String,
    #[serde(rename = "@SecureRegState")]
    pub secure_reg_state: String,
    #[serde(rename = "@VoiceConfigState")]
    pub voice_config_state: String,
    #[serde(rename = "@MicEnabled")]
    pub mic_enabled: String,
    #[serde(rename = "@HeadphoneSwapActive")]
    pub headphone_swap_active: String,
    #[serde(rename = "@AirPlayEnabled")]
    pub airplay_enabled: String,
    #[serde(rename = "@VirtualLineInSource", default)]
    pub virtual_line_in_source: Option<String>,
    #[serde(rename = "@IdleState")]
    pub idle_state: String,
    #[serde(rename = "@MoreInfo")]
    pub more_info: String,
    #[serde(rename = "@SSLPort")]
    pub ssl_port: String,
    #[serde(rename = "@HHSSLPort")]
    pub hhssl_port: String,
    #[serde(rename = "Satellite", default)]
    pub satellites: Vec<Satellite>,
}

#[derive(Debug, Deserialize)]
pub struct Satellite {
    #[serde(rename = "@UUID")]
    pub uuid: String,
    #[serde(rename = "@Location")]
    pub location: String,
    #[serde(rename = "@ZoneName")]
    pub zone_name: String,
    #[serde(rename = "@Icon")]
    pub icon: String,
    #[serde(rename = "@Configuration")]
    pub configuration: String,
    #[serde(rename = "@Invisible")]
    pub invisible: String,
    #[serde(rename = "@SoftwareVersion")]
    pub software_version: String,
    #[serde(rename = "@SWGen")]
    pub sw_gen: String,
    #[serde(rename = "@MinCompatibleVersion")]
    pub min_compatible_version: String,
    #[serde(rename = "@LegacyCompatibleVersion")]
    pub legacy_compatible_version: String,
    #[serde(rename = "@HTSatChanMapSet")]
    pub ht_sat_chan_map_set: String,
    #[serde(rename = "@ActiveZoneID")]
    pub active_zone_id: String,
    #[serde(rename = "@BootSeq")]
    pub boot_seq: String,
    #[serde(rename = "@TVConfigurationError")]
    pub tv_configuration_error: String,
    #[serde(rename = "@HdmiCecAvailable")]
    pub hdmi_cec_available: String,
    #[serde(rename = "@WirelessMode")]
    pub wireless_mode: String,
    #[serde(rename = "@WirelessLeafOnly")]
    pub wireless_leaf_only: String,
    #[serde(rename = "@ChannelFreq")]
    pub channel_freq: String,
    #[serde(rename = "@BehindWifiExtender")]
    pub behind_wifi_extender: String,
    #[serde(rename = "@WifiEnabled")]
    pub wifi_enabled: String,
    #[serde(rename = "@EthLink")]
    pub eth_link: String,
    #[serde(rename = "@Orientation")]
    pub orientation: String,
    #[serde(rename = "@RoomCalibrationState")]
    pub room_calibration_state: String,
    #[serde(rename = "@SecureRegState")]
    pub secure_reg_state: String,
    #[serde(rename = "@VoiceConfigState")]
    pub voice_config_state: String,
    #[serde(rename = "@MicEnabled")]
    pub mic_enabled: String,
    #[serde(rename = "@HeadphoneSwapActive")]
    pub headphone_swap_active: String,
    #[serde(rename = "@AirPlayEnabled")]
    pub airplay_enabled: String,
    #[serde(rename = "@IdleState")]
    pub idle_state: String,
    #[serde(rename = "@MoreInfo")]
    pub more_info: String,
    #[serde(rename = "@SSLPort")]
    pub ssl_port: String,
    #[serde(rename = "@HHSSLPort")]
    pub hhssl_port: String,
}

#[derive(Debug, Deserialize)]
pub struct VanishedDevices {
    // Empty for now, can be extended if needed
}

impl ZoneGroupTopologyParser {
    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::DeError> {
        crate::xml_decode::xml_decode::parse(xml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_speaker_leave() {
        const SAMPLE_XML: &str = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><ZoneGroupState>&lt;ZoneGroupState&gt;&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_5CAAFDAE58BD01400&quot; ID=&quot;RINCON_5CAAFDAE58BD01400:361632566&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_5CAAFDAE58BD01400&quot; Location=&quot;http://192.168.4.40:1400/xml/device_description.xml&quot; ZoneName=&quot;Basement&quot; Icon=&quot;&quot; Configuration=&quot;1&quot; SoftwareVersion=&quot;85.0-64200&quot; SWGen=&quot;2&quot; MinCompatibleVersion=&quot;84.0-00000&quot; LegacyCompatibleVersion=&quot;58.0-00000&quot; HTSatChanMapSet=&quot;RINCON_5CAAFDAE58BD01400:LF,RF;RINCON_7828CAFB9D9C01400:LR;RINCON_7828CA128F0001400:RR&quot; ActiveZoneID=&quot;289a89bc-23ff-4122-82c5-837f2f288e3b&quot; BootSeq=&quot;24&quot; TVConfigurationError=&quot;0&quot; HdmiCecAvailable=&quot;0&quot; WirelessMode=&quot;1&quot; WirelessLeafOnly=&quot;0&quot; ChannelFreq=&quot;2412&quot; BehindWifiExtender=&quot;0&quot; WifiEnabled=&quot;1&quot; EthLink=&quot;0&quot; Orientation=&quot;0&quot; RoomCalibrationState=&quot;4&quot; SecureRegState=&quot;3&quot; VoiceConfigState=&quot;0&quot; MicEnabled=&quot;0&quot; HeadphoneSwapActive=&quot;0&quot; AirPlayEnabled=&quot;0&quot; IdleState=&quot;1&quot; MoreInfo=&quot;&quot; SSLPort=&quot;1443&quot; HHSSLPort=&quot;1843&quot;&gt;&lt;Satellite UUID=&quot;RINCON_7828CA128F0001400&quot; Location=&quot;http://192.168.4.29:1400/xml/device_description.xml&quot; ZoneName=&quot;Basement&quot; Icon=&quot;&quot; Configuration=&quot;1&quot; Invisible=&quot;1&quot; SoftwareVersion=&quot;85.0-64200&quot; SWGen=&quot;2&quot; MinCompatibleVersion=&quot;84.0-00000&quot; LegacyCompatibleVersion=&quot;58.0-00000&quot; HTSatChanMapSet=&quot;RINCON_5CAAFDAE58BD01400:LF,RF;RINCON_7828CA128F0001400:RR&quot; ActiveZoneID=&quot;289a89bc-23ff-4122-82c5-837f2f288e3b&quot; BootSeq=&quot;28&quot; TVConfigurationError=&quot;0&quot; HdmiCecAvailable=&quot;0&quot; WirelessMode=&quot;2&quot; WirelessLeafOnly=&quot;0&quot; ChannelFreq=&quot;5825&quot; BehindWifiExtender=&quot;0&quot; WifiEnabled=&quot;1&quot; EthLink=&quot;0&quot; Orientation=&quot;0&quot; RoomCalibrationState=&quot;5&quot; SecureRegState=&quot;3&quot; VoiceConfigState=&quot;0&quot; MicEnabled=&quot;0&quot; HeadphoneSwapActive=&quot;0&quot; AirPlayEnabled=&quot;0&quot; IdleState=&quot;1&quot; MoreInfo=&quot;&quot; SSLPort=&quot;1443&quot; HHSSLPort=&quot;1843&quot;/&gt;&lt;Satellite UUID=&quot;RINCON_7828CAFB9D9C01400&quot; Location=&quot;http://192.168.4.30:1400/xml/device_description.xml&quot; ZoneName=&quot;Basement&quot; Icon=&quot;&quot; Configuration=&quot;1&quot; Invisible=&quot;1&quot; SoftwareVersion=&quot;85.0-64200&quot; SWGen=&quot;2&quot; MinCompatibleVersion=&quot;84.0-00000&quot; LegacyCompatibleVersion=&quot;58.0-00000&quot; HTSatChanMapSet=&quot;RINCON_5CAAFDAE58BD01400:LF,RF;RINCON_7828CAFB9D9C01400:LR&quot; ActiveZoneID=&quot;289a89bc-23ff-4122-82c5-837f2f288e3b&quot; BootSeq=&quot;27&quot; TVConfigurationError=&quot;0&quot; HdmiCecAvailable=&quot;0&quot; WirelessMode=&quot;2&quot; WirelessLeafOnly=&quot;0&quot; ChannelFreq=&quot;5825&quot; BehindWifiExtender=&quot;0&quot; WifiEnabled=&quot;1&quot; EthLink=&quot;0&quot; Orientation=&quot;0&quot; RoomCalibrationState=&quot;5&quot; SecureRegState=&quot;3&quot; VoiceConfigState=&quot;0&quot; MicEnabled=&quot;0&quot; HeadphoneSwapActive=&quot;0&quot; AirPlayEnabled=&quot;0&quot; IdleState=&quot;1&quot; MoreInfo=&quot;&quot; SSLPort=&quot;1443&quot; HHSSLPort=&quot;1843&quot;/&gt;&lt;/ZoneGroupMember&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_804AF2AA2FA201400&quot; ID=&quot;RINCON_804AF2AA2FA201400:1331296941&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_804AF2AA2FA201400&quot; Location=&quot;http://192.168.4.48:1400/xml/device_description.xml&quot; ZoneName=&quot;Living Room&quot; Icon=&quot;&quot; Configuration=&quot;1&quot; SoftwareVersion=&quot;85.0-65020&quot; SWGen=&quot;2&quot; MinCompatibleVersion=&quot;84.0-00000&quot; LegacyCompatibleVersion=&quot;58.0-00000&quot; BootSeq=&quot;52&quot; TVConfigurationError=&quot;0&quot; HdmiCecAvailable=&quot;1&quot; WirelessMode=&quot;0&quot; WirelessLeafOnly=&quot;0&quot; ChannelFreq=&quot;2437&quot; BehindWifiExtender=&quot;0&quot; WifiEnabled=&quot;0&quot; EthLink=&quot;1&quot; Orientation=&quot;0&quot; RoomCalibrationState=&quot;4&quot; SecureRegState=&quot;3&quot; VoiceConfigState=&quot;0&quot; MicEnabled=&quot;0&quot; HeadphoneSwapActive=&quot;0&quot; AirPlayEnabled=&quot;1&quot; IdleState=&quot;1&quot; MoreInfo=&quot;TargetRoomName:Living Room&quot; SSLPort=&quot;1443&quot; HHSSLPort=&quot;1843&quot;/&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_B8E937A84F0601400&quot; ID=&quot;RINCON_B8E937A84F0601400:154252828&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_B8E937A84F0601400&quot; Location=&quot;http://192.168.4.45:1400/xml/device_description.xml&quot; ZoneName=&quot;Bathroom&quot; Icon=&quot;&quot; Configuration=&quot;1&quot; SoftwareVersion=&quot;85.0-64200&quot; SWGen=&quot;2&quot; MinCompatibleVersion=&quot;84.0-00000&quot; LegacyCompatibleVersion=&quot;58.0-00000&quot; BootSeq=&quot;73&quot; TVConfigurationError=&quot;0&quot; HdmiCecAvailable=&quot;0&quot; WirelessMode=&quot;1&quot; WirelessLeafOnly=&quot;0&quot; ChannelFreq=&quot;2412&quot; BehindWifiExtender=&quot;0&quot; WifiEnabled=&quot;1&quot; EthLink=&quot;0&quot; Orientation=&quot;0&quot; RoomCalibrationState=&quot;4&quot; SecureRegState=&quot;3&quot; VoiceConfigState=&quot;0&quot; MicEnabled=&quot;0&quot; HeadphoneSwapActive=&quot;0&quot; AirPlayEnabled=&quot;0&quot; IdleState=&quot;1&quot; MoreInfo=&quot;&quot; SSLPort=&quot;1443&quot; HHSSLPort=&quot;1843&quot;/&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_000E583FEE3401400&quot; ID=&quot;RINCON_000E583FEE3401400:4075023526&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_000E583FEE3401400&quot; Location=&quot;http://192.168.4.46:1400/xml/device_description.xml&quot; ZoneName=&quot;Kitchen&quot; Icon=&quot;&quot; Configuration=&quot;1&quot; SoftwareVersion=&quot;85.0-64200&quot; SWGen=&quot;2&quot; MinCompatibleVersion=&quot;84.0-00000&quot; LegacyCompatibleVersion=&quot;58.0-00000&quot; BootSeq=&quot;164&quot; TVConfigurationError=&quot;0&quot; HdmiCecAvailable=&quot;0&quot; WirelessMode=&quot;1&quot; WirelessLeafOnly=&quot;0&quot; ChannelFreq=&quot;2412&quot; BehindWifiExtender=&quot;0&quot; WifiEnabled=&quot;1&quot; EthLink=&quot;0&quot; Orientation=&quot;0&quot; RoomCalibrationState=&quot;4&quot; SecureRegState=&quot;3&quot; VoiceConfigState=&quot;0&quot; MicEnabled=&quot;0&quot; HeadphoneSwapActive=&quot;0&quot; AirPlayEnabled=&quot;0&quot; IdleState=&quot;1&quot; MoreInfo=&quot;TargetRoomName:Kitchen&quot; SSLPort=&quot;1443&quot; HHSSLPort=&quot;1843&quot;/&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_C43875CA135801400&quot; ID=&quot;RINCON_C43875CA135801400:2858411499&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_C43875CA135801400&quot; Location=&quot;http://192.168.4.39:1400/xml/device_description.xml&quot; ZoneName=&quot;Roam / Office&quot; Icon=&quot;&quot; Configuration=&quot;1&quot; SoftwareVersion=&quot;85.0-64200&quot; SWGen=&quot;2&quot; MinCompatibleVersion=&quot;84.0-00000&quot; LegacyCompatibleVersion=&quot;58.0-00000&quot; BootSeq=&quot;137&quot; TVConfigurationError=&quot;0&quot; HdmiCecAvailable=&quot;0&quot; WirelessMode=&quot;1&quot; WirelessLeafOnly=&quot;0&quot; ChannelFreq=&quot;2412&quot; BehindWifiExtender=&quot;0&quot; WifiEnabled=&quot;1&quot; EthLink=&quot;0&quot; Orientation=&quot;3&quot; RoomCalibrationState=&quot;4&quot; SecureRegState=&quot;3&quot; VoiceConfigState=&quot;2&quot; MicEnabled=&quot;0&quot; HeadphoneSwapActive=&quot;0&quot; AirPlayEnabled=&quot;1&quot; VirtualLineInSource=&quot;spotify&quot; IdleState=&quot;0&quot; MoreInfo=&quot;RawBattPct:94,BattPct:100,BattChg:CHARGING,BattTmp:35&quot; SSLPort=&quot;1443&quot; HHSSLPort=&quot;1843&quot;/&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_5CAAFDEFEE7E01400&quot; ID=&quot;RINCON_C43875CA135801400:2858411497&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_5CAAFDEFEE7E01400&quot; Location=&quot;http://192.168.4.63:1400/xml/device_description.xml&quot; ZoneName=&quot;Bedroom&quot; Icon=&quot;&quot; Configuration=&quot;1&quot; SoftwareVersion=&quot;85.0-66270&quot; SWGen=&quot;2&quot; MinCompatibleVersion=&quot;84.0-00000&quot; LegacyCompatibleVersion=&quot;58.0-00000&quot; BootSeq=&quot;47&quot; TVConfigurationError=&quot;0&quot; HdmiCecAvailable=&quot;0&quot; WirelessMode=&quot;1&quot; WirelessLeafOnly=&quot;0&quot; ChannelFreq=&quot;2412&quot; BehindWifiExtender=&quot;0&quot; WifiEnabled=&quot;1&quot; EthLink=&quot;0&quot; Orientation=&quot;0&quot; RoomCalibrationState=&quot;4&quot; SecureRegState=&quot;3&quot; VoiceConfigState=&quot;0&quot; MicEnabled=&quot;0&quot; HeadphoneSwapActive=&quot;0&quot; AirPlayEnabled=&quot;0&quot; VirtualLineInSource=&quot;spotify&quot; IdleState=&quot;1&quot; MoreInfo=&quot;TargetRoomName:Bedroom&quot; SSLPort=&quot;1443&quot; HHSSLPort=&quot;1843&quot;/&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;&lt;VanishedDevices&gt;&lt;/VanishedDevices&gt;&lt;/ZoneGroupState&gt;</ZoneGroupState></e:property></e:propertyset>"#;
        
        let result = ZoneGroupTopologyParser::from_xml(SAMPLE_XML);
        
        assert!(
            result.is_ok(),
            "Failed to parse sample XML: {:?}",
            result.err()
        );
        
        let parsed = result.unwrap();
        let zone_groups = &parsed.zone_group_state.zone_group_state.zone_groups.zone_groups;
        
        // Test that we parsed multiple zone groups
        assert!(zone_groups.len() > 0, "Should have parsed at least one zone group");
        
        // Test the first zone group
        let first_zone = &zone_groups[0];
        assert_eq!(first_zone.coordinator, "RINCON_5CAAFDAE58BD01400");
        assert_eq!(first_zone.id, "RINCON_5CAAFDAE58BD01400:361632566");
        assert_eq!(first_zone.zone_group_members.len(), 1);
        assert_eq!(first_zone.zone_group_members[0].zone_name, "Basement");
        assert_eq!(first_zone.zone_group_members[0].satellites.len(), 2);
    }
}