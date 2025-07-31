/// Complete topology information for the Sonos system
#[derive(Debug, Clone)]
pub struct Topology {
    /// List of active zone groups in the system
    pub zone_groups: Vec<ZoneGroup>,
    /// Information about speakers that are no longer available
    pub vanished_devices: Option<VanishedDevices>,
}

/// Represents a Sonos zone group containing one or more speakers
#[derive(Debug, Clone)]
pub struct ZoneGroup {
    /// UUID of the coordinator speaker for this group
    pub coordinator: String,
    /// Unique identifier for this zone group
    pub id: String,
    /// List of speakers in this zone group
    pub members: Vec<ZoneGroupMember>,
}

/// Represents a speaker (zone group member) in the Sonos system
#[derive(Debug, Clone)]
pub struct ZoneGroupMember {
    /// Unique identifier for this speaker
    pub uuid: String,
    /// HTTP URL for this speaker's device description
    pub location: String,
    /// Human-readable name for this speaker/room
    pub zone_name: String,
    /// Software version running on this speaker
    pub software_version: String,
    /// Configuration flags for this speaker
    pub configuration: String,
    /// Icon identifier for this speaker type
    pub icon: String,
    /// List of satellite speakers associated with this main speaker
    pub satellites: Vec<Satellite>,
}

/// Represents a satellite speaker (e.g., surround speakers in a home theater setup)
#[derive(Debug, Clone)]
pub struct Satellite {
    /// Unique identifier for this satellite speaker
    pub uuid: String,
    /// HTTP URL for this satellite's device description
    pub location: String,
    /// Human-readable name for this satellite
    pub zone_name: String,
    /// Software version running on this satellite
    pub software_version: String,
}

/// Container for speakers that are no longer available on the network
#[derive(Debug, Clone)]
pub struct VanishedDevices {
    /// List of devices that have disappeared from the network
    pub devices: Vec<VanishedDevice>,
}

/// Represents a speaker that was previously discovered but is no longer available
#[derive(Debug, Clone)]
pub struct VanishedDevice {
    /// Unique identifier for this vanished speaker
    pub uuid: String,
    /// Last known name for this speaker
    pub zone_name: String,
    /// Reason why this speaker vanished (e.g., "powered off")
    pub reason: String,
}
