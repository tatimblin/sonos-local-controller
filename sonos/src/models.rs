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
    pub satellites: Vec<SpeakerId>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpeakerWithSatellites {
    pub speaker_id: SpeakerId,
    pub satellites: Vec<SpeakerId>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub id: GroupId,
    pub coordinator: SpeakerId,
    pub members: Vec<SpeakerWithSatellites>,
}

impl Group {
    pub fn new(coordinator: SpeakerId) -> Self {
        Self {
            id: GroupId::from_coordinator(coordinator),
            coordinator,
            members: vec![SpeakerWithSatellites {
                speaker_id: coordinator,
                satellites: vec![],
            }],
        }
    }

    pub fn add_member(&mut self, speaker_id: SpeakerId) {
        if !self.is_member(speaker_id) {
            self.members.push(SpeakerWithSatellites {
                speaker_id,
                satellites: vec![],
            });
        }
    }

    pub fn add_member_with_satellites(&mut self, speaker_id: SpeakerId, satellites: Vec<SpeakerId>) {
        if let Some(existing) = self.members.iter_mut().find(|m| m.speaker_id == speaker_id) {
            existing.satellites = satellites;
        } else {
            self.members.push(SpeakerWithSatellites {
                speaker_id,
                satellites,
            });
        }
    }

    pub fn remove_member(&mut self, speaker_id: SpeakerId) {
        self.members.retain(|member| member.speaker_id != speaker_id);
    }

    pub fn is_member(&self, speaker_id: SpeakerId) -> bool {
        self.members.iter().any(|member| member.speaker_id == speaker_id)
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get all speaker IDs including satellites
    pub fn all_speaker_ids(&self) -> Vec<SpeakerId> {
        let mut all_ids = Vec::new();
        for member in &self.members {
            all_ids.push(member.speaker_id);
            all_ids.extend(&member.satellites);
        }
        all_ids
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
    /// Enhanced group topology change with detailed information
    GroupTopologyChanged {
        groups: Vec<Group>,
        /// Speakers that joined groups
        speakers_joined: Vec<(SpeakerId, GroupId)>,
        /// Speakers that left groups  
        speakers_left: Vec<(SpeakerId, Option<GroupId>)>,
        /// New coordinators assigned (group_id, old_coordinator, new_coordinator)
        coordinator_changes: Vec<(GroupId, SpeakerId, SpeakerId)>,
    },
    /// Speaker joined a group
    SpeakerJoinedGroup {
        speaker_id: SpeakerId,
        group_id: GroupId,
        coordinator_id: SpeakerId,
    },
    /// Speaker left a group
    SpeakerLeftGroup {
        speaker_id: SpeakerId,
        former_group_id: GroupId,
    },
    /// Group coordinator changed
    CoordinatorChanged {
        group_id: GroupId,
        old_coordinator: SpeakerId,
        new_coordinator: SpeakerId,
    },
    /// New group was formed
    GroupFormed {
        group_id: GroupId,
        coordinator_id: SpeakerId,
        initial_members: Vec<SpeakerId>,
    },
    /// Group was dissolved
    GroupDissolved {
        group_id: GroupId,
        former_coordinator: SpeakerId,
        former_members: Vec<SpeakerId>,
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
        assert!(group.is_member(coordinator_id));
        
        // Coordinator should have no satellites initially
        let coordinator_member = &group.members[0];
        assert_eq!(coordinator_member.speaker_id, coordinator_id);
        assert_eq!(coordinator_member.satellites.len(), 0);
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
    fn test_group_add_member_with_satellites() {
        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");
        let satellite1_id = SpeakerId::from_udn("uuid:RINCON_SAT001::1");
        let satellite2_id = SpeakerId::from_udn("uuid:RINCON_SAT002::1");
        let mut group = Group::new(coordinator_id);

        group.add_member_with_satellites(coordinator_id, vec![satellite1_id, satellite2_id]);

        assert_eq!(group.members.len(), 1);
        assert!(group.is_member(coordinator_id));
        
        let coordinator_member = &group.members[0];
        assert_eq!(coordinator_member.satellites.len(), 2);
        assert!(coordinator_member.satellites.contains(&satellite1_id));
        assert!(coordinator_member.satellites.contains(&satellite2_id));
        
        // Check all_speaker_ids includes satellites
        let all_ids = group.all_speaker_ids();
        assert_eq!(all_ids.len(), 3);
        assert!(all_ids.contains(&coordinator_id));
        assert!(all_ids.contains(&satellite1_id));
        assert!(all_ids.contains(&satellite2_id));
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
