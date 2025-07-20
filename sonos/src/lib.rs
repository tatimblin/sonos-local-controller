mod model;

pub mod topology;
pub use topology::{Topology, ZoneGroup, ZoneGroupMember, Satellite, VanishedDevices, VanishedDevice};

mod client;
pub use client::Client;

mod system;
pub use system::System;
pub use system::SystemEvent;

pub mod speaker;
pub use speaker::{Speaker, SpeakerFactory, SpeakerTrait};

mod util;
pub use util::ssdp;

mod error;
pub use error::SonosError;

mod command;
pub use command::SpeakerCommand;

#[cfg(feature = "mock")]
pub mod testing {
  pub use crate::speaker::mock::MockSpeaker;
  pub use crate::speaker::mock::MockSpeakerBuilder;
}
