use std::fs;
use sonos::{SpeakerInfo, SonosError};

// cargo test --test speaker_info_tests

#[test]
fn test_parse_speaker_info_from_xml() {
  let xml_content = fs::read_to_string("tests/speaker_info_test_data.xml")
    .expect("Failed to read XML file");

  let parsed_info: Result<SpeakerInfo, SonosError> = SpeakerInfo::from_xml(&xml_content);

  assert!(parsed_info.is_ok(), "Parsing failed: {:?}", parsed_info);

  let speaker_info = parsed_info.unwrap();

  assert_eq!(speaker_info.get_name(), "10.0.0.62 - Sonos Playbar - RINCON_5CAAFDAE58BD01400");
  assert_eq!(speaker_info.get_room_name(), "Living Room");
}