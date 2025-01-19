use crate::types::*;
use super::store::AppState;

pub fn app_reducer(state: &mut AppState, action: AppAction) {
  match action {
    AppAction::AddSpeaker(speaker) => {
      state.speakers.push(speaker);
    },
    AppAction::SetSpeakers(speakers) => {
      state.speakers = speakers;
      state.view = View::Control;
    },
    AppAction::SetSelectedSpeaker(index) => {
      state.selected_speaker = Some(index);
    },
    AppAction::UpdateVolume(volume) => {
      if let Some(index) = state.selected_speaker {
        if let Some(speaker) = state.speakers.get(index) {
            speaker.set_volume(volume).unwrap_or_default();
        }
      }
    },
    AppAction::Exit => {},
  }
}
