use std::io;
use sonos::System;
use crate::state::store::Store;
use crate::types::AppAction;

pub fn use_speakers(store: &Store, mut render_callback: impl FnMut() -> io::Result<()>) -> io::Result<()> {
  let system = System::new()?;

  for speaker in system.discover() {
    store.dispatch(AppAction::AddSpeaker(speaker));
    render_callback()?;
  }

  Ok(())
}
