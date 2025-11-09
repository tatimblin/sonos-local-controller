use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename = "propertyset")]
pub struct RenderingControlParser {
    #[serde(rename = "property")]
    pub last_change: Property,
}

#[derive(Debug, Deserialize)]
pub struct Property {
    #[serde(
        rename = "LastChange",
        deserialize_with = "crate::xml_decode::xml_decode::deserialize_nested"
    )]
    pub last_change: LastChangeEvent,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "Event")]
pub struct LastChangeEvent {
    #[serde(rename = "InstanceID")]
    pub instance: InstanceID,
}

#[derive(Debug, Deserialize)]
pub struct InstanceID {
    #[serde(rename = "@val")]
    pub id: String,
    #[serde(rename = "Volume", default)]
    pub volumes: Vec<Volume>,
    #[serde(rename = "Mute", default)]
    pub mutes: Vec<Mute>,
    #[serde(rename = "Bass", default)]
    pub bass: Option<SimpleValue>,
    #[serde(rename = "Treble", default)]
    pub treble: Option<SimpleValue>,
    #[serde(rename = "Loudness", default)]
    pub loudness: Vec<Loudness>,
    #[serde(rename = "OutputFixed", default)]
    pub output_fixed: Option<SimpleValue>,
    #[serde(rename = "SpeakerSize", default)]
    pub speaker_size: Option<SimpleValue>,
    #[serde(rename = "SubGain", default)]
    pub sub_gain: Option<SimpleValue>,
    #[serde(rename = "SubCrossover", default)]
    pub sub_crossover: Option<SimpleValue>,
    #[serde(rename = "SubPolarity", default)]
    pub sub_polarity: Option<SimpleValue>,
    #[serde(rename = "SubEnabled", default)]
    pub sub_enabled: Option<SimpleValue>,
    #[serde(rename = "DialogLevel", default)]
    pub dialog_level: Option<SimpleValue>,
    #[serde(rename = "SpeechEnhanceEnabled", default)]
    pub speech_enhance_enabled: Option<SimpleValue>,
    #[serde(rename = "SurroundLevel", default)]
    pub surround_level: Option<SimpleValue>,
    #[serde(rename = "MusicSurroundLevel", default)]
    pub music_surround_level: Option<SimpleValue>,
    #[serde(rename = "AudioDelay", default)]
    pub audio_delay: Option<SimpleValue>,
    #[serde(rename = "AudioDelayLeftRear", default)]
    pub audio_delay_left_rear: Option<SimpleValue>,
    #[serde(rename = "AudioDelayRightRear", default)]
    pub audio_delay_right_rear: Option<SimpleValue>,
    #[serde(rename = "NightMode", default)]
    pub night_mode: Option<SimpleValue>,
    #[serde(rename = "SurroundEnabled", default)]
    pub surround_enabled: Option<SimpleValue>,
    #[serde(rename = "SurroundMode", default)]
    pub surround_mode: Option<SimpleValue>,
    #[serde(rename = "HeightChannelLevel", default)]
    pub height_channel_level: Option<SimpleValue>,
    #[serde(rename = "SonarEnabled", default)]
    pub sonar_enabled: Option<SimpleValue>,
    #[serde(rename = "SonarCalibrationAvailable", default)]
    pub sonar_calibration_available: Option<SimpleValue>,
    #[serde(rename = "PresetNameList", default)]
    pub preset_name_list: Option<SimpleValue>,
}

#[derive(Debug, Deserialize)]
pub struct Volume {
    #[serde(rename = "@channel")]
    pub channel: String,
    #[serde(rename = "@val")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Mute {
    #[serde(rename = "@channel")]
    pub channel: String,
    #[serde(rename = "@val")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Loudness {
    #[serde(rename = "@channel")]
    pub channel: String,
    #[serde(rename = "@val")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct SimpleValue {
    #[serde(rename = "@val")]
    pub value: String,
}

impl RenderingControlParser {
    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::DeError> {
        crate::xml_decode::xml_decode::parse(xml)
    }

    pub fn get_volume(&self) -> Option<u8> {
        let volume = self
            .last_change
            .last_change
            .instance
            .volumes
            .iter()
            .find(|v| v.channel == "Master")?;
        match volume.value.parse::<i32>() {
            Ok(volume_int) => {
                if volume_int < 0 {
                    None
                } else if volume_int > 100 {
                    None
                } else {
                    let volume_u8 = volume_int as u8;
                    Some(volume_u8)
                }
            }
            Err(_) => None,
        }
    }

    pub fn get_mute(&self) -> Option<bool> {
        let mute = self
            .last_change
            .last_change
            .instance
            .mutes
            .iter()
            .find(|m| m.channel == "Master")?;
        Some(mute.value == "1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_XML: &str = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><LastChange>&lt;Event xmlns=&quot;urn:schemas-upnp-org:metadata-1-0/RCS/&quot;&gt;&lt;InstanceID val=&quot;0&quot;&gt;&lt;Volume channel=&quot;Master&quot; val=&quot;15&quot;/&gt;&lt;Volume channel=&quot;LF&quot; val=&quot;100&quot;/&gt;&lt;Volume channel=&quot;RF&quot; val=&quot;100&quot;/&gt;&lt;Mute channel=&quot;Master&quot; val=&quot;0&quot;/&gt;&lt;Mute channel=&quot;LF&quot; val=&quot;0&quot;/&gt;&lt;Mute channel=&quot;RF&quot; val=&quot;0&quot;/&gt;&lt;Bass val=&quot;0&quot;/&gt;&lt;Treble val=&quot;0&quot;/&gt;&lt;Loudness channel=&quot;Master&quot; val=&quot;1&quot;/&gt;&lt;OutputFixed val=&quot;0&quot;/&gt;&lt;SpeakerSize val=&quot;6&quot;/&gt;&lt;SubGain val=&quot;0&quot;/&gt;&lt;SubCrossover val=&quot;0&quot;/&gt;&lt;SubPolarity val=&quot;0&quot;/&gt;&lt;SubEnabled val=&quot;1&quot;/&gt;&lt;DialogLevel val=&quot;1&quot;/&gt;&lt;SpeechEnhanceEnabled val=&quot;0&quot;/&gt;&lt;SurroundLevel val=&quot;0&quot;/&gt;&lt;MusicSurroundLevel val=&quot;0&quot;/&gt;&lt;AudioDelay val=&quot;0&quot;/&gt;&lt;AudioDelayLeftRear val=&quot;1&quot;/&gt;&lt;AudioDelayRightRear val=&quot;1&quot;/&gt;&lt;NightMode val=&quot;0&quot;/&gt;&lt;SurroundEnabled val=&quot;1&quot;/&gt;&lt;SurroundMode val=&quot;0&quot;/&gt;&lt;HeightChannelLevel val=&quot;0&quot;/&gt;&lt;SonarEnabled val=&quot;0&quot;/&gt;&lt;SonarCalibrationAvailable val=&quot;0&quot;/&gt;&lt;PresetNameList val=&quot;FactoryDefaults&quot;/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></e:property></e:propertyset>"#;

    #[test]
    fn test_parse_rendering_control_sample_xml() {
        let result = RenderingControlParser::from_xml(SAMPLE_XML);

        assert!(
            result.is_ok(),
            "Failed to parse sample XML: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let instance = &parsed.last_change.last_change.instance;

        // Test basic instance ID
        assert_eq!(instance.id, "0");

        // Test volumes
        assert_eq!(instance.volumes.len(), 3);
        let master_volume = instance
            .volumes
            .iter()
            .find(|v| v.channel == "Master")
            .unwrap();
        assert_eq!(master_volume.value, "15");

        // Test mutes
        assert_eq!(instance.mutes.len(), 3);
        let master_mute = instance
            .mutes
            .iter()
            .find(|m| m.channel == "Master")
            .unwrap();
        assert_eq!(master_mute.value, "0");

        // Test simple values
        assert_eq!(instance.bass.as_ref().unwrap().value, "0");
        assert_eq!(instance.treble.as_ref().unwrap().value, "0");
        assert_eq!(instance.speaker_size.as_ref().unwrap().value, "6");
        assert_eq!(instance.sub_enabled.as_ref().unwrap().value, "1");
        assert_eq!(instance.night_mode.as_ref().unwrap().value, "0");
        assert_eq!(instance.surround_enabled.as_ref().unwrap().value, "1");

        // Test loudness
        assert_eq!(instance.loudness.len(), 1);
        let master_loudness = &instance.loudness[0];
        assert_eq!(master_loudness.channel, "Master");
        assert_eq!(master_loudness.value, "1");

        // Test preset name list
        assert_eq!(
            instance.preset_name_list.as_ref().unwrap().value,
            "FactoryDefaults"
        );
    }

    #[test]
    fn test_parse_rendering_control_invalid_xml() {
        let invalid_xml = "<invalid>xml</invalid>";
        let result = RenderingControlParser::from_xml(invalid_xml);

        assert!(result.is_err(), "Should fail to parse invalid XML");
    }
}
