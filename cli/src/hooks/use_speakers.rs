use log::debug;
use std::io;
use std::sync::Arc;

use sonos::{SpeakerTrait, System, SystemEvent};

use crate::state::reducers::AppAction;
use crate::state::store::Store;
use crate::topology::topology_list::TopologyList;

pub fn use_speakers(
    store: &Store,
    mut render_callback: impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    let mut command_system_stored = false;

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
                SystemEvent::DiscoveryComplete => {
                    debug!("Initial discovery complete");
                    
                    // Create command system only once when initial discovery is complete
                    if !command_system_stored {
                        debug!("Creating command system for speaker commands");
                        match System::new() {
                            Ok(command_system) => {
                                let system_arc = Arc::new(command_system);
                                store.dispatch(AppAction::SetSystem(system_arc));
                                command_system_stored = true;
                                debug!("Command system ready");
                            }
                            Err(e) => {
                                debug!("Failed to create command system: {:?}", e);
                            }
                        }
                    }
                    
                    // Continue discovery loop - do NOT break
                }
                _ => {}
            }
        }
    });

    Ok(())
}
