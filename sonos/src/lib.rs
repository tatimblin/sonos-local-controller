mod model;
pub use model::PlayState;

pub mod topology;
pub use topology::{Topology, ZoneGroup, ZoneGroupMember, Satellite, VanishedDevices, VanishedDevice};

mod client;
pub use client::Client;

mod discover;
pub use discover::{discover_speakers_iter, discover_speakers, discover_topology};

pub mod speaker;
pub use speaker::{SpeakerController, SpeakerInfo};

mod util;
pub use util::ssdp;

mod error;
pub use error::SonosError;

//// #[cfg(feature = "mock")]
// pub mod testing {
//   pub use crate::speaker::mock::MockSpeaker;
//   pub use crate::speaker::mock::MockSpeakerBuilder;
// }
