use ratatui::Frame;
use super::selectable_list::SelectableList;
use sonos::Speaker;

pub struct SpeakerList {
  list: SelectableList,
}

impl SpeakerList {
  pub fn new(speakers: &[Speaker]) -> Self {
    let labels: Vec<String> = speakers
      .iter()
      .map(|speaker| {
        let name = speaker.get_info().get_name().to_string();
        let room = speaker.get_info().get_room_name().to_string();
        let volume = speaker.get_volume().unwrap_or(0);
        format!("{} - {}: {}", name, room, volume)
      })
      .collect();

    Self {
      list: SelectableList::new("Speakers", labels),
    }
  }

  pub fn draw(&mut self, frame: &mut Frame) {
    self.list.draw(frame, frame.area());
  }

  pub fn next(&mut self) {
    self.list.next();
  }

  pub fn previous(&mut self) {
    self.list.previous();
  }

  pub fn selected(&mut self) -> Option<usize> {
    self.list.selected()
  }
}
