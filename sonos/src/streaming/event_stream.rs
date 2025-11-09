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
    pub fn add_speaker(&self, speaker: &Speaker) -> SubscriptionResult<()> {
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
    pub fn remove_speaker(&self, speaker_id: &SpeakerId) -> SubscriptionResult<()> {
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
                log::debug!("🔊 Processing volume change: Speaker {:?} -> {}%", speaker_id, volume);
                state_cache.update_volume(&speaker_id, volume);
            }
            StateChange::MuteChanged { speaker_id, muted } => {
                log::debug!("🔇 Processing mute change: Speaker {:?} -> {}", speaker_id, if muted { "MUTED" } else { "UNMUTED" });
                state_cache.update_mute(&speaker_id, muted);
            }
            StateChange::PlaybackStateChanged { speaker_id, state } => {
                log::debug!("▶️ Processing playback state change: Speaker {:?} -> {:?}", speaker_id, state);
                state_cache.update_playback_state(&speaker_id, state);
            }
            StateChange::PositionChanged {
                speaker_id,
                position_ms,
            } => {
                state_cache.update_position(&speaker_id, position_ms);
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
                state_cache.update_playback_state(&speaker_id, transport_state);
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
            StateChange::GroupChange {
                groups
            } => {
                state_cache.set_groups(groups);
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
            id: SpeakerId::new(id),
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
        let speaker_id_str = "uuid:RINCON_123456789::1";

        // Test volume change processing
        let volume_event = StateChange::VolumeChanged {
            speaker_id: SpeakerId::new(speaker_id_str),
            volume: 50,
        };
        EventStream::process_state_change(&state_cache, volume_event);

        // Test playback state change processing
        let playback_event = StateChange::PlaybackStateChanged {
            speaker_id: SpeakerId::new(speaker_id_str),
            state: PlaybackState::Playing,
        };
        EventStream::process_state_change(&state_cache, playback_event);
    }
}
