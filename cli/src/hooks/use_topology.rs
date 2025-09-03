use std::io;

use sonos::discover_topology;

use crate::state::reducers::AppAction;
use crate::state::store::Store;
use crate::topology::topology_list::TopologyList;

/**
 * Event hook to return the topology
 */
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
    }
  }

  render_callback().ok();

  Ok(())
}
