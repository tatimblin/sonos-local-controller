use std::io;

use sonos::discover_speakers_iter;

use crate::state::reducers::AppAction;
use crate::state::store::Store;

/**
 * Event hook to iterate through speakers as they're found on the network
 */
pub fn use_speakers(
    store: &Store,
    mut render_callback: impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    for speaker in discover_speakers_iter() {
        store.dispatch(AppAction::SetStatusMessage(speaker.name.clone()));
        store.dispatch(AppAction::HydrateSpeakerTopology(speaker));
        render_callback().ok();
    }

    store.dispatch(AppAction::SetControlView);

    Ok(())
}
