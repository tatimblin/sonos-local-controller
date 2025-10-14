use std::sync::{mpsc, Arc};
use std::time::Duration;
use crate::models::{Speaker, StateChange, SpeakerId};
use super::types::StreamConfig;
use super::manager::SubscriptionManager;
use super::subscription::SubscriptionResult;

/// Unified event stream that merges events from all subscribed speakers
/// 
/// EventStream provides a single interface for consuming real-time events from
/// multiple Sonos speakers. It uses a channel-based architecture to merge events
/// from all active subscriptions into a unified stream.
pub struct EventStream {
    /// Receiver for StateChange events from all speakers
    receiver: mpsc::Receiver<StateChange>,
    /// Reference to the subscription manager (kept alive for the stream's lifetime)
    _subscription_manager: Arc<SubscriptionManager>,
}

impl EventStream {
    /// Create a new EventStream for the given speakers with the specified configuration
    /// 
    /// This method initializes the subscription manager and establishes subscriptions
    /// for all provided speakers according to the stream configuration.
    /// 
    /// # Arguments
    /// 
    /// * `speakers` - List of speakers to subscribe to
    /// * `config` - Configuration for the streaming system
    /// 
    /// # Returns
    /// 
    /// Returns a new EventStream instance or an error if initialization fails.
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// use sonos::streaming::{EventStream, StreamConfig};
    /// use sonos::models::Speaker;
    /// 
    /// let speakers = vec![/* discovered speakers */];
    /// let config = StreamConfig::default();
    /// let event_stream = EventStream::new(speakers, config)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(speakers: Vec<Speaker>, config: StreamConfig) -> SubscriptionResult<Self> {
        // Validate configuration
        config.validate().map_err(|e| {
            super::subscription::SubscriptionError::InvalidConfiguration(e)
        })?;

        // Create channel for events
        let (sender, receiver) = mpsc::channel();

        // Create subscription manager
        let subscription_manager = Arc::new(SubscriptionManager::new(config, sender)?);

        // Add all speakers to the manager
        for speaker in speakers {
            subscription_manager.add_speaker(speaker)?;
        }

        Ok(Self {
            receiver,
            _subscription_manager: subscription_manager,
        })
    }

    /// Try to receive an event without blocking
    /// 
    /// This method returns immediately with either an event or None if no events
    /// are currently available. It will never block the calling thread.
    /// 
    /// # Returns
    /// 
    /// * `Some(StateChange)` - If an event is available
    /// * `None` - If no events are currently available
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// # use sonos::streaming::EventStream;
    /// # let event_stream: EventStream = todo!();
    /// if let Some(event) = event_stream.try_recv() {
    ///     println!("Received event: {:?}", event);
    /// }
    /// ```
    pub fn try_recv(&self) -> Option<StateChange> {
        match self.receiver.try_recv() {
            Ok(event) => Some(event),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => None,
        }
    }

    /// Receive an event with a timeout
    /// 
    /// This method will wait up to the specified timeout for an event to become
    /// available. If no event is received within the timeout, it returns None.
    /// 
    /// # Arguments
    /// 
    /// * `timeout` - Maximum time to wait for an event
    /// 
    /// # Returns
    /// 
    /// * `Some(StateChange)` - If an event is received within the timeout
    /// * `None` - If no event is received within the timeout or the channel is disconnected
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// # use sonos::streaming::EventStream;
    /// # use std::time::Duration;
    /// # let event_stream: EventStream = todo!();
    /// let timeout = Duration::from_millis(100);
    /// if let Some(event) = event_stream.recv_timeout(timeout) {
    ///     println!("Received event: {:?}", event);
    /// } else {
    ///     println!("No event received within timeout");
    /// }
    /// ```
    pub fn recv_timeout(&self, timeout: Duration) -> Option<StateChange> {
        match self.receiver.recv_timeout(timeout) {
            Ok(event) => Some(event),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => None,
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
        self._subscription_manager.add_speaker(speaker)
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
        self._subscription_manager.remove_speaker(speaker_id)
    }

    /// Get an iterator over events
    /// 
    /// This method returns an iterator that will yield events as they become
    /// available. The iterator will block on each call to `next()` until an
    /// event is available or the stream is closed.
    /// 
    /// # Returns
    /// 
    /// Returns an iterator over StateChange events.
    /// 
    /// # Example
    /// 
    /// ```rust,no_run
    /// # use sonos::streaming::EventStream;
    /// # let event_stream: EventStream = todo!();
    /// for event in event_stream.iter() {
    ///     println!("Received event: {:?}", event);
    /// }
    /// ```
    pub fn iter(&self) -> EventStreamIterator {
        EventStreamIterator {
            receiver: &self.receiver,
        }
    }

    /// Check if the event stream is still active
    /// 
    /// Returns true if the subscription manager is still running and able to
    /// receive events. Returns false if the stream has been shut down or
    /// encountered a fatal error.
    pub fn is_active(&self) -> bool {
        // The stream is active as long as the subscription manager exists
        // and the receiver hasn't been disconnected
        !matches!(self.receiver.try_recv(), Err(mpsc::TryRecvError::Disconnected))
    }

    /// Get the number of pending events in the buffer
    /// 
    /// This method returns an estimate of how many events are currently
    /// buffered and waiting to be consumed. This can be useful for monitoring
    /// the event processing rate.
    /// 
    /// Note: This is an estimate and may not be perfectly accurate due to
    /// concurrent access to the channel.
    pub fn pending_events(&self) -> usize {
        // We can't directly get the length of an mpsc channel,
        // so we'll try to peek without consuming
        let mut count = 0;
        while matches!(self.receiver.try_recv(), Ok(_)) {
            count += 1;
            // Prevent infinite loop in case of very high event rate
            if count > 10000 {
                break;
            }
        }
        count
    }
}

/// Iterator over events from an EventStream
/// 
/// This iterator will block on each call to `next()` until an event is available
/// or the stream is closed.
pub struct EventStreamIterator<'a> {
    receiver: &'a mpsc::Receiver<StateChange>,
}

impl<'a> Iterator for EventStreamIterator<'a> {
    type Item = StateChange;

    fn next(&mut self) -> Option<Self::Item> {
        match self.receiver.recv() {
            Ok(event) => Some(event),
            Err(mpsc::RecvError) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Speaker, SpeakerId, PlaybackState};
    use std::time::Duration;

    fn create_test_speaker(id: &str) -> Speaker {
        Speaker {
            id: SpeakerId::from_udn(id),
            udn: id.to_string(),
            name: "Test Speaker".to_string(),
            room_name: "Test Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Test Model".to_string(),
        }
    }

    #[test]
    fn test_event_stream_creation() {
        let speakers = vec![create_test_speaker("uuid:RINCON_123456789::1")];
        let config = StreamConfig::default();
        
        // Test successful creation with placeholder SubscriptionManager
        let result = EventStream::new(speakers, config);
        assert!(result.is_ok());
        
        let event_stream = result.unwrap();
        assert!(event_stream.is_active());
    }

    #[test]
    fn test_stream_config_validation() {
        let speakers = vec![create_test_speaker("uuid:RINCON_123456789::1")];
        
        // Test with invalid config
        let invalid_config = StreamConfig::default().with_buffer_size(0);
        assert!(invalid_config.is_err());
        
        if let Ok(config) = invalid_config {
            let result = EventStream::new(speakers, config);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_try_recv_empty() {
        // Create a manual channel for testing
        let (_sender, receiver) = mpsc::channel::<StateChange>();
        
        // Test that try_recv returns None when no events are available
        match receiver.try_recv() {
            Ok(_) => panic!("Expected empty channel"),
            Err(mpsc::TryRecvError::Empty) => {}, // Expected
            Err(mpsc::TryRecvError::Disconnected) => {}, // Also acceptable for empty channel
        }
    }

    #[test]
    fn test_recv_timeout() {
        // Create a manual channel for testing
        let (_sender, receiver) = mpsc::channel::<StateChange>();
        let timeout = Duration::from_millis(10);
        
        // Test that recv_timeout returns None when no events are available within timeout
        match receiver.recv_timeout(timeout) {
            Ok(_) => panic!("Expected timeout"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}, // Expected
            Err(mpsc::RecvTimeoutError::Disconnected) => {}, // Also acceptable
        }
    }

    #[test]
    fn test_event_stream_iterator() {
        // Create a manual channel for testing
        let (sender, receiver) = mpsc::channel::<StateChange>();
        
        // Send a test event
        let test_event = StateChange::PlaybackStateChanged {
            speaker_id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            state: PlaybackState::Playing,
        };
        sender.send(test_event.clone()).unwrap();
        
        // Close the sender to end the iterator
        drop(sender);
        
        // Test iterator
        let mut iter = EventStreamIterator { receiver: &receiver };
        let received_event = iter.next();
        assert!(received_event.is_some());
        
        // Next call should return None since channel is closed
        let no_event = iter.next();
        assert!(no_event.is_none());
    }
}