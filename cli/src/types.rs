use sonos::Speaker;

#[derive(Debug)]
pub enum AppAction {
  AddSpeaker(Speaker),
  SetSelectedSpeaker(usize),
  AdjustVolume(i8),
}

#[derive(Clone, Copy, Debug)]
pub enum View {
  Startup,
  Control,
}
