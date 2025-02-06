use std::io;
use sonos::System;
use crate::state::store::Store;
// use crate::types::AppAction;

pub fn use_speakers(_: &Store, mut render_callback: impl FnMut() -> io::Result<()>) -> io::Result<()> {
  let system = System::new()?;

  for _ in system.discover() {
    // store.dispatch(AppAction::AddSpeaker(_));
    render_callback()?;
  }

  Ok(())
}
