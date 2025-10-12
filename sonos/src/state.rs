use crate::models::{Group, GroupId, PlaybackState, Speaker, SpeakerId, SpeakerState};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct StateCache {
    speakers: Arc<RwLock<HashMap<SpeakerId, SpeakerState>>>,
    groups: Arc<RwLock<HashMap<GroupId, Group>>>,
}

impl StateCache {
    pub fn new() -> Self {
        Self {
            speakers: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn initialize(&self, speakers: Vec<Speaker>, groups: Vec<Group>) {
        // Initialize speakers
        let mut speaker_cache = self.speakers.write().unwrap();
        for speaker in speakers {
            let id = speaker.id;
            speaker_cache.insert(
                id,
                SpeakerState {
                    speaker,
                    playback_state: PlaybackState::Stopped,
                    volume: 0,
                    muted: false,
                    position_ms: 0,
                    duration_ms: 0,
                    is_coordinator: false,
                    group_id: None,
                },
            );
        }
        drop(speaker_cache); // Release the lock early

        // Initialize groups
        self.set_groups(groups);
    }

    pub fn get_speaker(&self, id: SpeakerId) -> Option<SpeakerState> {
        self.speakers.read().unwrap().get(&id).cloned()
    }

    pub fn get_all_speakers(&self) -> Vec<SpeakerState> {
        self.speakers.read().unwrap().values().cloned().collect()
    }

    pub fn get_by_room(&self, room_name: &str) -> Vec<SpeakerState> {
        self.speakers
            .read()
            .unwrap()
            .values()
            .filter(|s| s.speaker.room_name == room_name)
            .cloned()
            .collect()
    }

    pub fn get_by_name(&self, name: &str) -> Option<SpeakerState> {
        self.speakers
            .read()
            .unwrap()
            .values()
            .find(|s| s.speaker.name == name)
            .cloned()
    }

    pub fn update_volume(&self, id: SpeakerId, volume: u8) {
        if let Ok(mut speakers) = self.speakers.write() {
            if let Some(state) = speakers.get_mut(&id) {
                state.volume = volume;
            }
        }
    }

    pub fn update_mute(&self, id: SpeakerId, muted: bool) {
        if let Ok(mut speakers) = self.speakers.write() {
            if let Some(state) = speakers.get_mut(&id) {
                state.muted = muted;
            }
        }
    }

    pub fn update_playback_state(&self, id: SpeakerId, state: PlaybackState) {
        if let Ok(mut speakers) = self.speakers.write() {
            if let Some(speaker_state) = speakers.get_mut(&id) {
                speaker_state.playback_state = state;
            }
        }
    }

    pub fn update_position(&self, id: SpeakerId, position_ms: u64) {
        if let Ok(mut speakers) = self.speakers.write() {
            if let Some(state) = speakers.get_mut(&id) {
                state.position_ms = position_ms;
            }
        }
    }

    pub fn get_group(&self, group_id: GroupId) -> Option<Group> {
        self.groups.read().unwrap().get(&group_id).cloned()
    }

    pub fn get_all_groups(&self) -> Vec<Group> {
        self.groups.read().unwrap().values().cloned().collect()
    }

    fn set_groups(&self, groups: Vec<Group>) {
        let mut group_cache = self.groups.write().unwrap();
        group_cache.clear();

        for group in groups {
            group_cache.insert(group.id, group);
        }

        if let Ok(mut speakers) = self.speakers.write() {
            for speaker_state in speakers.values_mut() {
                speaker_state.group_id = None;
                speaker_state.is_coordinator = false;
            }

            for group in group_cache.values() {
                for &member_id in &group.members {
                    if let Some(speaker_state) = speakers.get_mut(&member_id) {
                        speaker_state.group_id = Some(group.id);
                        speaker_state.is_coordinator = member_id == group.coordinator;
                    }
                }
            }
        }
    }

    pub fn get_speakers_in_group(&self, group_id: GroupId) -> Vec<SpeakerState> {
        self.speakers
            .read()
            .unwrap()
            .values()
            .filter(|s| s.group_id == Some(group_id))
            .cloned()
            .collect()
    }

    pub fn get_group_coordinator(&self, group_id: GroupId) -> Option<SpeakerState> {
        self.speakers
            .read()
            .unwrap()
            .values()
            .find(|s| s.group_id == Some(group_id) && s.is_coordinator)
            .cloned()
    }
}

impl Clone for StateCache {
    fn clone(&self) -> Self {
        Self {
            speakers: self.speakers.clone(),
            groups: self.groups.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_default_speaker_state(state: &SpeakerState, expected_name: &str) {
        assert_eq!(state.speaker.name, expected_name);
        assert_eq!(state.playback_state, PlaybackState::Stopped);
        assert_eq!(state.volume, 0);
        assert_eq!(state.muted, false);
        assert_eq!(state.position_ms, 0);
        assert_eq!(state.duration_ms, 0);
    }

    fn create_test_cache() -> (StateCache, Speaker, Speaker) {
        let cache = StateCache::new();

        let speaker1 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
            name: "Living Room".to_string(),
            room_name: "Living Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Sonos One".to_string(),
        };

        let speaker2 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_987654321::1"),
            udn: "uuid:RINCON_987654321::1".to_string(),
            name: "Kitchen Speaker".to_string(),
            room_name: "Kitchen".to_string(),
            ip_address: "192.168.1.101".to_string(),
            port: 1400,
            model_name: "Sonos Play:1".to_string(),
        };

        cache.initialize(vec![speaker1.clone(), speaker2.clone()], vec![]);
        (cache, speaker1, speaker2)
    }

    #[test]
    fn test_new() {
        let cache = StateCache::new();

        // Verify that the cache is created with an empty HashMap
        assert_eq!(cache.get_all_speakers().len(), 0);
    }

    #[test]
    fn test_initialize() {
        let cache = StateCache::new();

        let speaker1 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
            name: "Living Room".to_string(),
            room_name: "Living Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Sonos One".to_string(),
        };

        let speaker2 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_987654321::1"),
            udn: "uuid:RINCON_987654321::1".to_string(),
            name: "Kitchen".to_string(),
            room_name: "Kitchen".to_string(),
            ip_address: "192.168.1.101".to_string(),
            port: 1400,
            model_name: "Sonos Play:1".to_string(),
        };

        let speakers = vec![speaker1.clone(), speaker2.clone()];
        cache.initialize(speakers, vec![]);

        let all_speakers = cache.get_all_speakers();
        assert_eq!(all_speakers.len(), 2);

        let speaker1_state = cache.get_speaker(speaker1.id).unwrap();
        assert_default_speaker_state(&speaker1_state, "Living Room");

        let speaker2_state = cache.get_speaker(speaker2.id).unwrap();
        assert_default_speaker_state(&speaker2_state, "Kitchen");
    }

    #[test]
    fn test_get_speaker() {
        let (cache, speaker1, _) = create_test_cache();

        let found = cache.get_speaker(speaker1.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().speaker.name, "Living Room");

        let not_found = cache.get_speaker(SpeakerId::from_udn("uuid:RINCON_999999999::1"));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_all_speakers() {
        let (cache, _, _) = create_test_cache();

        let all = cache.get_all_speakers();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_get_by_room() {
        let (cache, _, _) = create_test_cache();

        let living_room_speakers = cache.get_by_room("Living Room");
        assert_eq!(living_room_speakers.len(), 1);
        assert_eq!(living_room_speakers[0].speaker.name, "Living Room");

        let kitchen_speakers = cache.get_by_room("Kitchen");
        assert_eq!(kitchen_speakers.len(), 1);
        assert_eq!(kitchen_speakers[0].speaker.name, "Kitchen Speaker");

        let empty = cache.get_by_room("Bedroom");
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_get_by_name() {
        let (cache, _, _) = create_test_cache();

        let found = cache.get_by_name("Living Room");
        assert!(found.is_some());
        assert_eq!(found.unwrap().speaker.room_name, "Living Room");

        let not_found = cache.get_by_name("Bedroom");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_update_volume() {
        let (cache, speaker1, _) = create_test_cache();

        cache.update_volume(speaker1.id, 50);

        let state = cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(state.volume, 50);
    }

    #[test]
    fn test_update_mute() {
        let (cache, speaker1, _) = create_test_cache();

        cache.update_mute(speaker1.id, true);

        let state = cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(state.muted, true);
    }

    #[test]
    fn test_update_playback_state() {
        let (cache, speaker1, _) = create_test_cache();

        cache.update_playback_state(speaker1.id, PlaybackState::Playing);

        let state = cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(state.playback_state, PlaybackState::Playing);
    }

    #[test]
    fn test_update_position() {
        let (cache, speaker1, _) = create_test_cache();

        cache.update_position(speaker1.id, 30000);

        let state = cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(state.position_ms, 30000);
    }

    #[test]
    fn test_clone() {
        let (cache, speaker1, _) = create_test_cache();

        let cloned_cache = cache.clone();

        // Verify the clone has the same data
        let original_state = cache.get_speaker(speaker1.id).unwrap();
        let cloned_state = cloned_cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(original_state.speaker.name, cloned_state.speaker.name);

        // Verify they share the same underlying data (Arc)
        cache.update_volume(speaker1.id, 75);
        let updated_state = cloned_cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(updated_state.volume, 75);
    }

    #[test]
    fn test_group_functionality() {
        let cache = StateCache::new();

        let speaker1 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
            name: "Living Room".to_string(),
            room_name: "Living Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Sonos One".to_string(),
        };

        let speaker2 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_987654321::1"),
            udn: "uuid:RINCON_987654321::1".to_string(),
            name: "Kitchen Speaker".to_string(),
            room_name: "Kitchen".to_string(),
            ip_address: "192.168.1.101".to_string(),
            port: 1400,
            model_name: "Sonos Play:1".to_string(),
        };

        // Create a group with speaker1 as coordinator and speaker2 as member
        let mut group = Group::new(speaker1.id);
        group.add_member(speaker2.id);

        cache.initialize(
            vec![speaker1.clone(), speaker2.clone()],
            vec![group.clone()],
        );

        // Verify group was added
        let retrieved_group = cache.get_group(group.id).unwrap();
        assert_eq!(retrieved_group.coordinator, speaker1.id);
        assert_eq!(retrieved_group.members.len(), 2);
        assert!(retrieved_group.is_member(speaker1.id));
        assert!(retrieved_group.is_member(speaker2.id));

        // Verify speakers were updated with group info
        let speaker1_state = cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(speaker1_state.group_id, Some(group.id));
        assert_eq!(speaker1_state.is_coordinator, true);

        let speaker2_state = cache.get_speaker(speaker2.id).unwrap();
        assert_eq!(speaker2_state.group_id, Some(group.id));
        assert_eq!(speaker2_state.is_coordinator, false);
    }

    #[test]
    fn test_get_speakers_in_group() {
        let cache = StateCache::new();

        let speaker1 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
            name: "Living Room".to_string(),
            room_name: "Living Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Sonos One".to_string(),
        };

        let speaker2 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_987654321::1"),
            udn: "uuid:RINCON_987654321::1".to_string(),
            name: "Kitchen Speaker".to_string(),
            room_name: "Kitchen".to_string(),
            ip_address: "192.168.1.101".to_string(),
            port: 1400,
            model_name: "Sonos Play:1".to_string(),
        };

        let mut group = Group::new(speaker1.id);
        group.add_member(speaker2.id);
        cache.initialize(
            vec![speaker1.clone(), speaker2.clone()],
            vec![group.clone()],
        );

        let group_speakers = cache.get_speakers_in_group(group.id);
        assert_eq!(group_speakers.len(), 2);

        let speaker_names: Vec<String> = group_speakers
            .iter()
            .map(|s| s.speaker.name.clone())
            .collect();
        assert!(speaker_names.contains(&"Living Room".to_string()));
        assert!(speaker_names.contains(&"Kitchen Speaker".to_string()));
    }

    #[test]
    fn test_get_group_coordinator() {
        let cache = StateCache::new();

        let speaker1 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
            name: "Living Room".to_string(),
            room_name: "Living Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Sonos One".to_string(),
        };

        let speaker2 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_987654321::1"),
            udn: "uuid:RINCON_987654321::1".to_string(),
            name: "Kitchen Speaker".to_string(),
            room_name: "Kitchen".to_string(),
            ip_address: "192.168.1.101".to_string(),
            port: 1400,
            model_name: "Sonos Play:1".to_string(),
        };

        let mut group = Group::new(speaker1.id);
        group.add_member(speaker2.id);
        cache.initialize(
            vec![speaker1.clone(), speaker2.clone()],
            vec![group.clone()],
        );

        let coordinator = cache.get_group_coordinator(group.id).unwrap();
        assert_eq!(coordinator.speaker.id, speaker1.id);
        assert_eq!(coordinator.is_coordinator, true);
    }

    #[test]
    fn test_get_all_groups() {
        let cache = StateCache::new();

        let speaker1 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
            name: "Living Room".to_string(),
            room_name: "Living Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Sonos One".to_string(),
        };

        let speaker2 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_987654321::1"),
            udn: "uuid:RINCON_987654321::1".to_string(),
            name: "Kitchen Speaker".to_string(),
            room_name: "Kitchen".to_string(),
            ip_address: "192.168.1.101".to_string(),
            port: 1400,
            model_name: "Sonos Play:1".to_string(),
        };

        let group1 = Group::new(speaker1.id);
        let group2 = Group::new(speaker2.id);

        cache.initialize(
            vec![speaker1.clone(), speaker2.clone()],
            vec![group1.clone(), group2.clone()],
        );

        let all_groups = cache.get_all_groups();
        assert_eq!(all_groups.len(), 2);
    }

    #[test]
    fn test_initialize_with_groups() {
        let cache = StateCache::new();

        let speaker1 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
            name: "Living Room".to_string(),
            room_name: "Living Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Sonos One".to_string(),
        };

        let speaker2 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_987654321::1"),
            udn: "uuid:RINCON_987654321::1".to_string(),
            name: "Kitchen".to_string(),
            room_name: "Kitchen".to_string(),
            ip_address: "192.168.1.101".to_string(),
            port: 1400,
            model_name: "Sonos Play:1".to_string(),
        };

        // Create a group with speaker1 as coordinator and speaker2 as member
        let mut group = Group::new(speaker1.id);
        group.add_member(speaker2.id);

        let speakers = vec![speaker1.clone(), speaker2.clone()];
        let groups = vec![group.clone()];

        cache.initialize(speakers, groups);

        // Verify speakers were initialized
        let all_speakers = cache.get_all_speakers();
        assert_eq!(all_speakers.len(), 2);

        // Verify groups were initialized
        let all_groups = cache.get_all_groups();
        assert_eq!(all_groups.len(), 1);

        let retrieved_group = cache.get_group(group.id).unwrap();
        assert_eq!(retrieved_group.coordinator, speaker1.id);
        assert_eq!(retrieved_group.members.len(), 2);

        // Verify speakers have correct group information
        let speaker1_state = cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(speaker1_state.group_id, Some(group.id));
        assert_eq!(speaker1_state.is_coordinator, true);

        let speaker2_state = cache.get_speaker(speaker2.id).unwrap();
        assert_eq!(speaker2_state.group_id, Some(group.id));
        assert_eq!(speaker2_state.is_coordinator, false);
    }
}
