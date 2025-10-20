use super::manager::SubscriptionManager;
use super::subscription::SubscriptionResult;
use crate::models::{Speaker, SpeakerId, StateChange};
use crate::state::StateCache;
use std::sync::Arc;

/// Internal event stream for subscription management
///
/// EventStream is now an internal component focused solely on subscription management
/// and event forwarding. The public interface is provided by EventStreamBuilder and
/// ActiveEventStream.
pub(crate) struct EventStream {
    /// Reference to the subscription manager (kept alive for the stream's lifetime)
    subscription_manager: Arc<SubscriptionManager>,
}

impl EventStream {
    /// Create a new internal EventStream with the given subscription manager
    ///
    /// This is an internal constructor used by the new public interface.
    pub(crate) fn new(subscription_manager: Arc<SubscriptionManager>) -> Self {
        Self {
            subscription_manager,
        }
    }

    /// Add a new speaker to the event stream
    ///
    /// This method adds a speaker to the subscription manager, which will
    /// automatically establish subscriptions for the enabled services.
    ///
    /// # Arguments
    ///
    /// * `speaker` - The speaker to add
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the speaker was added successfully, or an error if
    /// the operation failed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use sonos::streaming::EventStream;
    /// # use sonos::models::Speaker;
    /// # let event_stream: EventStream = todo!();
    /// # let new_speaker: Speaker = todo!();
    /// event_stream.add_speaker(new_speaker)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add_speaker(&self, speaker: Speaker) -> SubscriptionResult<()> {
        self.subscription_manager.add_speaker(speaker)
    }

    /// Remove a speaker from the event stream
    ///
    /// This method removes a speaker from the subscription manager, which will
    /// automatically unsubscribe from all services for that speaker.
    ///
    /// # Arguments
    ///
    /// * `speaker_id` - The ID of the speaker to remove
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the speaker was removed successfully, or an error if
    /// the operation failed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use sonos::streaming::EventStream;
    /// # use sonos::models::SpeakerId;
    /// # let event_stream: EventStream = todo!();
    /// # let speaker_id: SpeakerId = todo!();
    /// event_stream.remove_speaker(speaker_id)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove_speaker(&self, speaker_id: SpeakerId) -> SubscriptionResult<()> {
        self.subscription_manager.remove_speaker(speaker_id)
    }

    /// Process events and update StateCache
    ///
    /// This method processes StateChange events and updates the StateCache accordingly.
    /// It's used internally by the new streaming interface.
    pub(crate) fn process_state_change(state_cache: &StateCache, event: StateChange) {
        Self::process_state_change_internal(state_cache, event);
    }

    /// Process a StateChange event and update the StateCache accordingly
    ///
    /// This is a helper method that handles the mapping between StateChange events
    /// and StateCache update methods.
    fn process_state_change_internal(state_cache: &StateCache, event: StateChange) {
        match event {
            StateChange::VolumeChanged { speaker_id, volume } => {
                state_cache.update_volume(speaker_id, volume);
            }
            StateChange::MuteChanged { speaker_id, muted } => {
                state_cache.update_mute(speaker_id, muted);
            }
            StateChange::PlaybackStateChanged { speaker_id, state } => {
                state_cache.update_playback_state(speaker_id, state);
            }
            StateChange::PositionChanged {
                speaker_id,
                position_ms,
            } => {
                state_cache.update_position(speaker_id, position_ms);
            }
            StateChange::GroupTopologyChanged {
                groups,
                speakers_joined: _,
                speakers_left: _,
                coordinator_changes: _,
            } => {
                // For group topology changes, we need to reinitialize the groups
                // Since StateCache doesn't have a public method to update groups,
                // we'll need to get the current speakers and reinitialize
                let current_speakers: Vec<_> = state_cache
                    .get_all_speakers()
                    .into_iter()
                    .map(|s| s.speaker)
                    .collect();
                state_cache.initialize(current_speakers, groups);
            }
            StateChange::TrackChanged {
                speaker_id,
                track_info,
            } => {
                // Track information changes don't directly update StateCache
                // as it doesn't currently store track info, but we log this
                // for future extension when StateCache supports track info
                log::debug!(
                    "Track changed for speaker {:?}: {:?}",
                    speaker_id,
                    track_info
                );
            }
            StateChange::TransportInfoChanged {
                speaker_id,
                transport_state,
                transport_status: _,
            } => {
                // Update playback state from transport info
                state_cache.update_playback_state(speaker_id, transport_state);
            }
            StateChange::SubscriptionError {
                speaker_id,
                service,
                error,
            } => {
                // Log subscription errors but don't update state cache
                log::error!(
                    "Subscription error for speaker {:?} on service {:?}: {}",
                    speaker_id,
                    service,
                    error
                );
            }
            StateChange::SpeakerJoinedGroup {
                speaker_id,
                group_id,
                coordinator_id,
            } => {
                log::debug!(
                    "Speaker {:?} joined group {:?} with coordinator {:?}",
                    speaker_id,
                    group_id,
                    coordinator_id
                );
                state_cache.add_speaker_to_group(speaker_id, group_id);
            }
            StateChange::SpeakerLeftGroup {
                speaker_id,
                former_group_id,
            } => {
                log::debug!("Speaker {:?} left group {:?}", speaker_id, former_group_id);
                state_cache.remove_speaker_from_group(speaker_id);
            }
            StateChange::CoordinatorChanged {
                group_id,
                old_coordinator,
                new_coordinator,
            } => {
                log::debug!(
                    "Group {:?} coordinator changed from {:?} to {:?}",
                    group_id,
                    old_coordinator,
                    new_coordinator
                );
                state_cache.change_group_coordinator(group_id, new_coordinator);
            }
            StateChange::GroupFormed {
                group_id,
                coordinator_id,
                initial_members,
            } => {
                log::debug!(
                    "New group {:?} formed with coordinator {:?} and {} members",
                    group_id,
                    coordinator_id,
                    initial_members.len()
                );
                state_cache.create_group(coordinator_id, initial_members);
            }
            StateChange::GroupDissolved {
                group_id,
                former_coordinator,
                former_members,
            } => {
                log::debug!(
                    "Group {:?} dissolved, former coordinator {:?}, {} former members",
                    group_id,
                    former_coordinator,
                    former_members.len()
                );
                state_cache.dissolve_group(group_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PlaybackState, Speaker, SpeakerId};

    fn create_test_speaker(id: &str) -> Speaker {
        Speaker {
            id: SpeakerId::from_udn(id),
            udn: id.to_string(),
            name: "Test Speaker".to_string(),
            room_name: "Test Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Test Model".to_string(),
            satellites: vec![],
        }
    }

    #[test]
    fn test_process_state_change() {
        use crate::state::StateCache;
        use std::sync::Arc;

        let state_cache = Arc::new(StateCache::new());
        let speaker_id = SpeakerId::from_udn("uuid:RINCON_123456789::1");

        // Test volume change processing
        let volume_event = StateChange::VolumeChanged {
            speaker_id,
            volume: 50,
        };
        EventStream::process_state_change(&state_cache, volume_event);

        // Test playback state change processing
        let playback_event = StateChange::PlaybackStateChanged {
            speaker_id,
            state: PlaybackState::Playing,
        };
        EventStream::process_state_change(&state_cache, playback_event);
    }

    #[test]
    fn test_process_group_state_changes() {
        use crate::models::{Speaker, GroupId};
        use crate::state::StateCache;
        use std::sync::Arc;

        let state_cache = Arc::new(StateCache::new());

        // Create test speakers
        let speaker1 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
            name: "Living Room".to_string(),
            room_name: "Living Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Sonos One".to_string(),
            satellites: vec![],
        };

        let speaker2 = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_987654321::1"),
            udn: "uuid:RINCON_987654321::1".to_string(),
            name: "Kitchen".to_string(),
            room_name: "Kitchen".to_string(),
            ip_address: "192.168.1.101".to_string(),
            port: 1400,
            model_name: "Sonos Play:1".to_string(),
            satellites: vec![],
        };

        // Initialize the cache with speakers
        state_cache.initialize(vec![speaker1.clone(), speaker2.clone()], vec![]);

        // Test group formation
        let group_formed_event = StateChange::GroupFormed {
            group_id: GroupId::from_coordinator(speaker1.id),
            coordinator_id: speaker1.id,
            initial_members: vec![speaker1.id, speaker2.id],
        };
        EventStream::process_state_change(&state_cache, group_formed_event);

        // Verify group was created
        let group_id = GroupId::from_coordinator(speaker1.id);
        let group = state_cache.get_group(group_id).unwrap();
        assert_eq!(group.coordinator, speaker1.id);
        assert_eq!(group.member_count(), 2);

        // Verify speaker states
        let speaker1_state = state_cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(speaker1_state.group_id, Some(group_id));
        assert_eq!(speaker1_state.is_coordinator, true);

        let speaker2_state = state_cache.get_speaker(speaker2.id).unwrap();
        assert_eq!(speaker2_state.group_id, Some(group_id));
        assert_eq!(speaker2_state.is_coordinator, false);

        // Test coordinator change
        let coordinator_changed_event = StateChange::CoordinatorChanged {
            group_id,
            old_coordinator: speaker1.id,
            new_coordinator: speaker2.id,
        };
        EventStream::process_state_change(&state_cache, coordinator_changed_event);

        // Verify coordinator changed
        let updated_group = state_cache.get_group(group_id).unwrap();
        assert_eq!(updated_group.coordinator, speaker2.id);

        let updated_speaker1_state = state_cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(updated_speaker1_state.is_coordinator, false);

        let updated_speaker2_state = state_cache.get_speaker(speaker2.id).unwrap();
        assert_eq!(updated_speaker2_state.is_coordinator, true);

        // Test speaker leaving group
        let speaker_left_event = StateChange::SpeakerLeftGroup {
            speaker_id: speaker1.id,
            former_group_id: group_id,
        };
        EventStream::process_state_change(&state_cache, speaker_left_event);

        // Verify speaker left
        let final_speaker1_state = state_cache.get_speaker(speaker1.id).unwrap();
        assert_eq!(final_speaker1_state.group_id, None);
        assert_eq!(final_speaker1_state.is_coordinator, false);

        let final_group = state_cache.get_group(group_id).unwrap();
        assert_eq!(final_group.member_count(), 1);
        assert!(!final_group.is_member(speaker1.id));

        // Test group dissolution
        let group_dissolved_event = StateChange::GroupDissolved {
            group_id,
            former_coordinator: speaker2.id,
            former_members: vec![speaker2.id],
        };
        EventStream::process_state_change(&state_cache, group_dissolved_event);

        // Verify group was dissolved
        assert!(state_cache.get_group(group_id).is_none());

        let final_speaker2_state = state_cache.get_speaker(speaker2.id).unwrap();
        assert_eq!(final_speaker2_state.group_id, None);
        assert_eq!(final_speaker2_state.is_coordinator, false);
    }
}
