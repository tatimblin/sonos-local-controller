use mockall::{mock, predicate::*};
use crate::error::SonosError;
use crate::speaker::SpeakerTrait;
use xmltree::Element;

mock! {
  #[derive(Debug)]
    pub Speaker {}

    impl SpeakerTrait for Speaker {
        fn name(&self) -> &str;
        fn room_name(&self) -> &str;
        fn ip(&self) -> &str;
        fn uuid(&self) -> &str;

        fn play(&self) -> Result<(), SonosError>;
        fn pause(&self) -> Result<(), SonosError>;
        fn get_volume(&self) -> Result<u8, SonosError>;
        fn set_volume(&self, volume: u8) -> Result<u8, SonosError>;
        fn adjust_volume(&self, adjustment: i8) -> Result<u8, SonosError>;
        fn parse_element_u8(&self, element: &Element, key: &str) -> Result<u8, SonosError>;
    }
}

pub struct MockSpeakerBuilder {
  name: String,
  room_name: String,
  ip: String,
  uuid: String,
}

impl MockSpeakerBuilder {
  pub fn new() -> Self {
    Self {
      name: "Living Room Speaker".into(),
      room_name: "Living Room".into(),
      ip: "192.168.1.100".into(),
      uuid: "uuid:RINCON_000E58C0123456789".into(),
    }
  }

  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = name.into();
    self
  }

  pub fn room_name(mut self, room_name: impl Into<String>) -> Self {
    self.room_name = room_name.into();
    self
  }

  pub fn ip(mut self, ip: impl Into<String>) -> Self {
    self.ip = ip.into();
    self
  }

  pub fn uuid(mut self, uuid: impl Into<String>) -> Self {
    self.uuid = uuid.into();
    self
  }

  pub fn build(self) -> MockSpeaker {
    let mut speaker = MockSpeaker::new();

    speaker.expect_name().return_const(self.name);
    speaker.expect_room_name().return_const(self.room_name);
    speaker.expect_ip().return_const(self.ip);
    speaker.expect_uuid().return_const(self.uuid);

    speaker.expect_play().returning(|| Ok(()));
    speaker.expect_pause().returning(|| Ok(()));
    speaker.expect_get_volume().returning(|| Ok(50));
    speaker.expect_set_volume().returning(|vol| Ok(vol));
    speaker.expect_adjust_volume().returning(|adj| {
      if adj > 0 { Ok(75) } else { Ok(25) }
    });
    speaker.expect_parse_element_u8().returning(|_, key| match key {
      "GetVolume" | "NewVolume" => Ok(50),
      _ => Err(SonosError::ParseError(format!("Unknown key: {}", key))),
    });

    speaker
  }
}
