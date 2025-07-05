use ratatui::Frame;

use sonos::SpeakerTrait;

use super::selectable_list::SelectableList;

pub struct SpeakerList {
  list: SelectableList,
}

impl SpeakerList {
  pub fn new(speakers: &[Box<dyn SpeakerTrait>]) -> Self {
    let labels: Vec<String> = speakers
      .iter()
      .map(|speaker| format!("Speaker: {}", speaker.name()))
      .collect();

    Self {
      list: SelectableList::new("Speakers", labels),
    }
  }

  pub fn from_names(speaker_names: &[String]) -> Self {
    let labels: Vec<String> = speaker_names
      .iter()
      .map(|name| format!("Speaker: {}", name))
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

  pub fn len(&self) -> usize {
    self.list.len()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sonos::testing::MockSpeakerBuilder;

  #[test]
  fn test_new_speaker() {
    let speaker = MockSpeakerBuilder::new().build();
    let mut speaker_list = SpeakerList::new(&[
      Box::new(speaker)
    ]);

    assert_eq!(speaker_list.len(), 1, "List should have 1 speaker");

    let selected = speaker_list.selected();
    assert_eq!(selected, Some(0), "First speaker should be initially selected");
  }

  #[test]
  fn test_select_next_speaker() {
    let speaker_a = MockSpeakerBuilder::new().build();
    let speaker_b = MockSpeakerBuilder::new().build();
    let mut speaker_list = SpeakerList::new(&[
      Box::new(speaker_a),
      Box::new(speaker_b)
    ]);

    assert_eq!(speaker_list.len(), 2, "List should have 2 speakers");

    assert_eq!(speaker_list.selected(), Some(0), "First speaker should be initially selected");
    
    speaker_list.next();

    assert_eq!(speaker_list.selected(), Some(1), "Second speaker should then be selected");
  }

  #[test]
  fn test_select_previous_speaker() {
    let speaker_a = MockSpeakerBuilder::new().build();
    let speaker_b = MockSpeakerBuilder::new().build();
    let mut speaker_list = SpeakerList::new(&[
      Box::new(speaker_a),
      Box::new(speaker_b)
    ]);

    speaker_list.next();

    assert_eq!(speaker_list.len(), 2, "List should have 2 speakers");

    assert_eq!(speaker_list.selected(), Some(1), "Second speaker should be selected");
    
    speaker_list.previous();

    assert_eq!(speaker_list.selected(), Some(0), "First speaker should then be selected");
  }

  #[test]
  fn test_select_next_speaker_wrapped() {
    let speaker_a = MockSpeakerBuilder::new().build();
    let speaker_b = MockSpeakerBuilder::new().build();
    let mut speaker_list = SpeakerList::new(&[
      Box::new(speaker_a),
      Box::new(speaker_b)
      ]);

    speaker_list.next();
    
    assert_eq!(speaker_list.len(), 2, "List should have 2 speakers");
    
    assert_eq!(speaker_list.selected(), Some(1), "Seconds speaker should be selected");
    
    speaker_list.next();

    assert_eq!(speaker_list.selected(), Some(0), "First speaker should then be selected");
  }

  #[test]
  fn test_select_previous_speaker_wrapped() {
    let speaker_a = MockSpeakerBuilder::new().build();
    let speaker_b = MockSpeakerBuilder::new().build();
    let mut speaker_list = SpeakerList::new(&[
      Box::new(speaker_a),
      Box::new(speaker_b)
    ]);

    assert_eq!(speaker_list.len(), 2, "List should have 2 speakers");

    assert_eq!(speaker_list.selected(), Some(0), "First speaker should be initially selected");
    
    speaker_list.previous();

    assert_eq!(speaker_list.selected(), Some(1), "Second speaker should then be selected");
  }
}
