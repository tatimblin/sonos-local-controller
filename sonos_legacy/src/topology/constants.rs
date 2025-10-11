//! XML element and attribute name constants for topology parsing
//!
//! This module contains all the constant strings used for parsing XML elements
//! and attributes in Sonos topology responses.

// XML element name constants
pub const ZONE_GROUPS_ELEMENT: &str = "ZoneGroups";
pub const ZONE_GROUP_ELEMENT: &str = "ZoneGroup";
pub const ZONE_GROUP_MEMBER_ELEMENT: &str = "ZoneGroupMember";
pub const SATELLITE_ELEMENT: &str = "Satellite";
pub const VANISHED_DEVICES_ELEMENT: &str = "VanishedDevices";
pub const DEVICE_ELEMENT: &str = "Device";

// XML attribute name constants
pub const COORDINATOR_ATTR: &str = "Coordinator";
pub const ID_ATTR: &str = "ID";
pub const UUID_ATTR: &str = "UUID";
pub const LOCATION_ATTR: &str = "Location";
pub const ZONE_NAME_ATTR: &str = "ZoneName";
pub const SOFTWARE_VERSION_ATTR: &str = "SoftwareVersion";
pub const CONFIGURATION_ATTR: &str = "Configuration";
pub const ICON_ATTR: &str = "Icon";
pub const REASON_ATTR: &str = "Reason";