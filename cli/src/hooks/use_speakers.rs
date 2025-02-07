use std::io::Result;

use sonos::{ SystemEvent, System };

use crate::state::store::Store;
use crate::state::reducers::AppAction;

pub fn use_speakers(store: &Store, mut render_callback: impl FnMut() -> Result<()>) -> Result<()> {
  let system = System::new()?;

  Ok(for event in system.discover() {
    match event {
      SystemEvent::Found(speaker) => {
        store.dispatch(AppAction::SetStatusMessage(speaker.name().to_string()));
        render_callback()?;
      },
      _ => {}
    }
  })
}
