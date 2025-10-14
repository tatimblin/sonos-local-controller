use crate::streaming::ServiceType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeakerId(u32);

impl SpeakerId {
    /// Create a SpeakerId from a UDN string
    pub fn from_udn(udn: &str) -> Self {
        // Extract the RINCON part and create a hash
        let hash = if let Some(rincon_part) = udn.strip_prefix("uuid:RINCON_") {
            let rincon_id = rincon_part.split("::").next().unwrap_or(rincon_part);
            // Simple hash of the RINCON ID
            rincon_id
                .chars()
                .fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32))
        } else {
            // Fallback hash for non-RINCON UDNs
            udn.chars()
                .fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32))
        };

        SpeakerId(hash)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupId(u32);

impl GroupId {
    /// Create a GroupId from a coordinator's SpeakerId
    pub fn from_coordinator(coordinator_id: SpeakerId) -> Self {
        GroupId(coordinator_id.0)
    }
}

#[derive(Debug, Clone)]
pub struct Speaker {
    pub id: SpeakerId,
    pub udn: String,
    pub name: String,
    pub room_name: String,
    pub ip_address: String,
    pub port: u16,
    pub model_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
    Transitioning,
}

#[derive(Debug, Clone)]
pub struct SpeakerState {
    pub speaker: Speaker,
    pub playback_state: PlaybackState,
    pub volume: u8,
    pub muted: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub is_coordinator: bool,
    pub group_id: Option<GroupId>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub id: GroupId,
    pub coordinator: SpeakerId,
    pub members: Vec<SpeakerId>,
}

impl Group {
    pub fn new(coordinator: SpeakerId) -> Self {
        Self {
            id: GroupId::from_coordinator(coordinator),
            coordinator,
            members: vec![coordinator],
        }
    }

    pub fn add_member(&mut self, speaker_id: SpeakerId) {
        if !self.members.contains(&speaker_id) {
            self.members.push(speaker_id);
        }
    }

    pub fn remove_member(&mut self, speaker_id: SpeakerId) {
        self.members.retain(|&id| id != speaker_id);
    }

    pub fn is_member(&self, speaker_id: SpeakerId) -> bool {
        self.members.contains(&speaker_id)
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

#[derive(Debug, Clone)]
pub enum StateChange {
    VolumeChanged {
        speaker_id: SpeakerId,
        volume: u8,
    },
    MuteChanged {
        speaker_id: SpeakerId,
        muted: bool,
    },
    PlaybackStateChanged {
        speaker_id: SpeakerId,
        state: PlaybackState,
    },
    PositionChanged {
        speaker_id: SpeakerId,
        position_ms: u64,
    },
    GroupTopologyChanged {
        groups: Vec<Group>,
    },
    // New streaming-specific variants
    TrackChanged {
        speaker_id: SpeakerId,
        track_info: Option<TrackInfo>,
    },
    TransportInfoChanged {
        speaker_id: SpeakerId,
        transport_state: PlaybackState,
        transport_status: TransportStatus,
    },
    SubscriptionError {
        speaker_id: SpeakerId,
        service: ServiceType,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportStatus {
    Ok,
    ErrorOccurred,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speaker_id_from_udn() {
        let udn1 = "uuid:RINCON_123456789::1";
        let udn2 = "uuid:RINCON_987654321::1";
        let udn3 = "uuid:RINCON_123456789::1"; // Same as udn1

        let id1 = SpeakerId::from_udn(udn1);
        let id2 = SpeakerId::from_udn(udn2);
        let id3 = SpeakerId::from_udn(udn3);

        // Same UDN should produce same ID
        assert_eq!(id1, id3);
        // Different UDNs should produce different IDs
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_speaker_id_from_non_rincon_udn() {
        let udn = "uuid:some-other-format";
        let id = SpeakerId::from_udn(udn);

        // Should not panic and should produce a valid ID
        assert!(id.0 > 0);
    }

    #[test]
    fn test_group_id_from_coordinator() {
        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");
        let group_id = GroupId::from_coordinator(coordinator_id);

        // GroupId should be derived from the coordinator's SpeakerId
        assert_eq!(group_id.0, coordinator_id.0);

        // Same coordinator should produce same GroupId
        let group_id2 = GroupId::from_coordinator(coordinator_id);
        assert_eq!(group_id, group_id2);
    }

    #[test]
    fn test_group_new() {
        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");
        let group = Group::new(coordinator_id);

        assert_eq!(group.id, GroupId::from_coordinator(coordinator_id));
        assert_eq!(group.coordinator, coordinator_id);
        assert_eq!(group.members.len(), 1);
        assert!(group.members.contains(&coordinator_id));
    }

    #[test]
    fn test_group_add_member() {
        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");
        let member_id = SpeakerId::from_udn("uuid:RINCON_987654321::1");
        let mut group = Group::new(coordinator_id);

        group.add_member(member_id);

        assert_eq!(group.members.len(), 2);
        assert!(group.is_member(coordinator_id));
        assert!(group.is_member(member_id));

        // Adding the same member again should not duplicate
        group.add_member(member_id);
        assert_eq!(group.members.len(), 2);
    }

    #[test]
    fn test_group_remove_member() {
        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");
        let member_id = SpeakerId::from_udn("uuid:RINCON_987654321::1");
        let mut group = Group::new(coordinator_id);
        group.add_member(member_id);

        group.remove_member(member_id);

        assert_eq!(group.members.len(), 1);
        assert!(group.is_member(coordinator_id));
        assert!(!group.is_member(member_id));
    }

    #[test]
    fn test_group_member_count() {
        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");
        let member_id = SpeakerId::from_udn("uuid:RINCON_987654321::1");
        let mut group = Group::new(coordinator_id);

        assert_eq!(group.member_count(), 1);

        group.add_member(member_id);
        assert_eq!(group.member_count(), 2);

        group.remove_member(member_id);
        assert_eq!(group.member_count(), 1);
    }
}
