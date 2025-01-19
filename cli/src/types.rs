use sonos::Speaker;

#[derive(Debug)]
pub enum AppAction {
  AddSpeaker(Speaker),
  SetSpeakers(Vec<Speaker>),
  SetSelectedSpeaker(usize),
  UpdateVolume(u8),
  Exit,
}

#[derive(Clone, Copy, Debug)]
pub enum View {
  Startup,
  Control,
}
