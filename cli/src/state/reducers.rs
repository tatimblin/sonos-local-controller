use crate::types::*;
use super::store::AppState;

pub fn app_reducer(state: &mut AppState, action: AppAction) {
  match action {
    AppAction::AddSpeaker(speaker) => {
      state.speakers.add_speaker(speaker);
    },
    AppAction::SetSelectedSpeaker(index) => {
      
    },
    AppAction::AdjustVolume(adjustment) => {
      
    },
  }
}
