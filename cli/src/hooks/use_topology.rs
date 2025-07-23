use std::io;

use sonos::discover_topology;

use crate::state::reducers::AppAction;
use crate::state::store::Store;
use crate::topology::topology_list::TopologyList;

pub fn use_topology(
    store: &Store,
    mut render_callback: impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    match discover_topology() {
        Ok(sonos_topology) => {
            let topology_list = TopologyList::new(sonos_topology);
            store.dispatch(AppAction::UpdateTopology(topology_list));
        }
        Err(e) => {
            eprintln!("Failed to discover topology: {}", e);
            // You might want to handle this error differently based on your needs
        }
    }

    render_callback().ok();

    Ok(())
}
