use crate::types::*;
use super::store::AppState;

pub fn app_reducer(state: &mut AppState, action: AppAction) {
  match action {
    AppAction::AddSpeaker(speaker) => {
      state.speakers.push(speaker);
    },
    AppAction::SetSelectedSpeaker(index) => {
      state.selected_speaker = Some(index);
    },
    AppAction::AdjustVolume(adjustment) => {
      if let Some(index) = state.selected_speaker {
        if let Some(speaker) = state.speakers.get(index) {
          let _ = speaker.set_relative_volume(adjustment);
        }
      }
    },
  }
}
