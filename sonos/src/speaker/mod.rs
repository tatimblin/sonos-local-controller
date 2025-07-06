mod speaker;
pub use speaker::Speaker;
pub use speaker::SpeakerFactory;
pub use speaker::SpeakerTrait;

mod device;
pub use device::Device;

#[cfg(feature = "mock")]
pub mod mock;