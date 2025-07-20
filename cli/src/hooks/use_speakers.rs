use log::debug;
use std::io;

use sonos::{SpeakerTrait, SystemEvent};

use crate::state::reducers::AppAction;
use crate::state::store::Store;
use crate::topology::topology_list::TopologyList;

pub fn use_speakers(
    store: &Store,
    mut render_callback: impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    // Get the discovery system from the store and run discovery
    store.with_discovery_system(|discovery_system| {
        // Process discovery events - runs indefinitely for ongoing network monitoring
        for event in discovery_system.discover() {
            match event {
                SystemEvent::SpeakerFound(speaker) => {
                    store.dispatch(AppAction::SetStatusMessage(speaker.name().to_owned()));
                    render_callback().ok();
                }
                SystemEvent::TopologyReady(sonos_topology) => {
                    debug!("TopologyReady event received");
                    let topology = TopologyList::new(sonos_topology);
                    store.dispatch(AppAction::SetTopology(topology));
                    render_callback().ok();
                }
                _ => {
                    // Ignore all other events
                }
            }
        }
    });

    Ok(())
}
