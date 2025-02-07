use sonos::Speaker;

use super::store::AppState;

#[derive(Debug)]
pub enum AppAction {
  AddSpeaker(Speaker),
  AdjustVolume(i8),
  SetSelectedSpeaker(usize),
  SetStatusMessage(String),
}

pub fn app_reducer(state: &mut AppState, action: AppAction) {
  match action {
    AppAction::AddSpeaker(_speaker) => {

    },
    AppAction::AdjustVolume(_adjustment) => {
      
    },
    AppAction::SetSelectedSpeaker(_index) => {
      
    },
    AppAction::SetStatusMessage(message) => {
      state.status_message = message;
    }
  }
}
