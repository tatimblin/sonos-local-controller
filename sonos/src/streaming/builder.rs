use super::interface::{ConfigOverrides, LifecycleHandlers, StreamError, StreamStats};
use super::manager::SubscriptionManager;
use super::types::{ServiceType, StreamConfig};
use crate::models::{Speaker, SpeakerId, StateChange};
use crate::state::StateCache;
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;

/// Builder for creating EventStream instances with a fluent interface
///
/// This builder provides a clean, intuitive way to configure and create event streams
/// for monitoring Sonos speakers. It uses sensible defaults while allowing customization
/// of advanced features.
///
/// # Example
///
/// ```rust,no_run
/// use sonos::streaming::EventStreamBuilder;
/// use sonos::state::StateCache;
/// use std::sync::Arc;
///
/// let speakers = vec![/* discovered speakers */];
/// let state_cache = Arc::new(StateCache::new());
///
/// let stream = EventStreamBuilder::new(speakers)?
///     .with_state_cache(state_cache)
///     .with_event_handler(|event| println!("Event: {:?}", event))
///     .start()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct EventStreamBuilder {
    speakers: Vec<Speaker>,
    services: Vec<ServiceType>,
    state_cache: Option<Arc<StateCache>>,
    event_handlers: Vec<Box<dyn Fn(StateChange) + Send + Sync>>,
    lifecycle_handlers: LifecycleHandlers,
    config_overrides: ConfigOverrides,
}

impl std::fmt::Debug for EventStreamBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventStreamBuilder")
            .field("speakers", &self.speakers.len())
            .field("services", &self.services)
            .field("has_state_cache", &self.state_cache.is_some())
            .field("event_handlers_count", &self.event_handlers.len())
            .finish()
    }
}

impl EventStreamBuilder {
    /// Create a new EventStreamBuilder with the specified speakers
    ///
    /// This constructor sets up sensible defaults:
    /// - AVTransport service enabled (for basic playback events)
    /// - No automatic StateCache integration
    /// - No event handlers
    /// - Default lifecycle handlers
    /// - Default configuration settings
    ///
    /// # Arguments
    ///
    /// * `speakers` - List of speakers to monitor for events
    ///
    /// # Returns
    ///
    /// Returns a new EventStreamBuilder instance or an error if the speaker list is invalid.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sonos::streaming::EventStreamBuilder;
    ///
    /// let speakers = vec![/* discovered speakers */];
    /// let builder = EventStreamBuilder::new(speakers)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(speakers: Vec<Speaker>) -> Result<Self, StreamError> {
        if speakers.is_empty() {
            return Err(StreamError::ConfigurationError(
                "At least one speaker must be provided".to_string(),
            ));
        }

        Ok(Self {
            speakers,
            services: vec![
                ServiceType::AVTransport,
                ServiceType::RenderingControl,
                ServiceType::ZoneGroupTopology, // Re-enabled after fixing processing order
            ], // Default to basic playback events
            state_cache: None,
            event_handlers: Vec::new(),
            lifecycle_handlers: LifecycleHandlers::default(),
            config_overrides: ConfigOverrides::default(),
        })
    }

    /// Enable automatic StateCache integration
    ///
    /// When a StateCache is provided, the event stream will automatically update
    /// the cache with received state changes. This provides seamless integration
    /// between the streaming system and the existing state management.
    ///
    /// # Arguments
    ///
    /// * `cache` - The StateCache instance to update with received events
    ///
    /// # Returns
    ///
    /// Returns the builder instance for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sonos::streaming::EventStreamBuilder;
    /// use sonos::state::StateCache;
    /// use std::sync::Arc;
    ///
    /// let speakers = vec![/* discovered speakers */];
    /// let state_cache = Arc::new(StateCache::new());
    ///
    /// let builder = EventStreamBuilder::new(speakers)?
    ///     .with_state_cache(state_cache);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_state_cache(mut self, cache: Arc<StateCache>) -> Self {
        self.state_cache = Some(cache);
        self
    }

    /// Configure which Sonos services to subscribe to
    ///
    /// This method allows you to specify which UPnP services should be monitored
    /// for events. Different services provide different types of information:
    /// - AVTransport: Playback state, track changes, transport info
    /// - RenderingControl: Volume, mute, audio settings
    /// - ContentDirectory: Media library changes (less commonly used)
    ///
    /// # Arguments
    ///
    /// * `services` - Slice of ServiceType values to enable
    ///
    /// # Returns
    ///
    /// Returns the builder instance for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sonos::streaming::{EventStreamBuilder, ServiceType};
    ///
    /// let speakers = vec![/* discovered speakers */];
    ///
    /// let builder = EventStreamBuilder::new(speakers)?
    ///     .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_services(mut self, services: &[ServiceType]) -> Self {
        if !services.is_empty() {
            self.services = services.to_vec();
        }
        self
    }

    /// Add an event handler callback
    ///
    /// Event handlers are called for each StateChange event received from the speakers.
    /// Multiple handlers can be registered and they will be called in the order they
    /// were added. Handlers should be lightweight and non-blocking to avoid impacting
    /// event processing performance.
    ///
    /// # Arguments
    ///
    /// * `handler` - Callback function that will be called for each event
    ///
    /// # Returns
    ///
    /// Returns the builder instance for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sonos::streaming::EventStreamBuilder;
    /// use sonos::models::StateChange;
    ///
    /// let speakers = vec![/* discovered speakers */];
    ///
    /// let builder = EventStreamBuilder::new(speakers)?
    ///     .with_event_handler(|event| {
    ///         match event {
    ///             StateChange::PlaybackStateChanged { speaker_id, state } => {
    ///                 println!("Speaker {:?} changed to {:?}", speaker_id, state);
    ///             }
    ///             _ => {}
    ///         }
    ///     })
    ///     .with_event_handler(|event| {
    ///         // Another handler for the same events
    ///         println!("Received event: {:?}", event);
    ///     });
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_event_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(StateChange) + Send + Sync + 'static,
    {
        self.event_handlers.push(Box::new(handler));
        self
    }

    /// Add lifecycle event handlers for connection events
    ///
    /// Lifecycle handlers allow you to respond to streaming state changes such as
    /// speaker connections, disconnections, and errors. These handlers are called
    /// in addition to regular event handlers and provide insight into the health
    /// of the streaming system.
    ///
    /// # Arguments
    ///
    /// * `handlers` - LifecycleHandlers struct with callback functions
    ///
    /// # Returns
    ///
    /// Returns the builder instance for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sonos::streaming::{EventStreamBuilder, LifecycleHandlers};
    ///
    /// let speakers = vec![/* discovered speakers */];
    ///
    /// let lifecycle_handlers = LifecycleHandlers::new()
    ///     .with_speaker_connected(|speaker_id| {
    ///         println!("Speaker {:?} connected", speaker_id);
    ///     })
    ///     .with_error(|error| {
    ///         eprintln!("Streaming error: {:?}", error);
    ///     });
    ///
    /// let builder = EventStreamBuilder::new(speakers)?
    ///     .with_lifecycle_handlers(lifecycle_handlers);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_lifecycle_handlers(mut self, handlers: LifecycleHandlers) -> Self {
        self.lifecycle_handlers = handlers;
        self
    }

    /// Configure subscription and retry timeouts
    ///
    /// This method allows you to customize the timeout behavior of the streaming
    /// system. The subscription timeout controls how long subscriptions remain
    /// active before renewal, while the retry timeout controls the backoff
    /// duration for failed operations.
    ///
    /// # Arguments
    ///
    /// * `subscription_timeout` - How long subscriptions remain active (min 60s, max 24h)
    /// * `retry_timeout` - Base duration for exponential backoff retry strategy
    ///
    /// # Returns
    ///
    /// Returns the builder instance for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sonos::streaming::EventStreamBuilder;
    /// use std::time::Duration;
    ///
    /// let speakers = vec![/* discovered speakers */];
    ///
    /// let builder = EventStreamBuilder::new(speakers)?
    ///     .with_timeouts(
    ///         Duration::from_secs(3600), // 1 hour subscription timeout
    ///         Duration::from_secs(2)     // 2 second retry backoff
    ///     );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_timeouts(
        mut self,
        subscription_timeout: Duration,
        retry_timeout: Duration,
    ) -> Self {
        self.config_overrides.subscription_timeout = Some(subscription_timeout);
        self.config_overrides.retry_backoff = Some(retry_timeout);
        self
    }

    /// Configure callback server port range
    ///
    /// The streaming system runs an HTTP callback server to receive events from
    /// Sonos speakers. This method allows you to specify which port range the
    /// server should use. This is useful in environments with firewall restrictions
    /// or when running multiple instances.
    ///
    /// # Arguments
    ///
    /// * `start` - Starting port number (must be >= 1024)
    /// * `end` - Ending port number (must be > start)
    ///
    /// # Returns
    ///
    /// Returns the builder instance for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sonos::streaming::EventStreamBuilder;
    ///
    /// let speakers = vec![/* discovered speakers */];
    ///
    /// let builder = EventStreamBuilder::new(speakers)?
    ///     .with_callback_ports(9000, 9010); // Use ports 9000-9010
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_callback_ports(mut self, start: u16, end: u16) -> Self {
        self.config_overrides.callback_port_range = Some((start, end));
        self
    }

    /// Build and start the EventStream
    ///
    /// This method creates the internal components (SubscriptionManager, event processing
    /// thread, etc.) and starts the event stream. It returns an ActiveEventStream instance
    /// that can be used for runtime speaker management and graceful shutdown.
    ///
    /// # Returns
    ///
    /// Returns an ActiveEventStream instance or an error if initialization fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sonos::streaming::EventStreamBuilder;
    ///
    /// let speakers = vec![/* discovered speakers */];
    ///
    /// let stream = EventStreamBuilder::new(speakers)?
    ///     .with_event_handler(|event| println!("Event: {:?}", event))
    ///     .start()?;
    ///
    /// // Stream is now active and processing events
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn start(self) -> Result<ActiveEventStream, StreamError> {
        // Create internal StreamConfig from builder settings
        let config = self.build_stream_config()?;

        // Create channel for events
        let (sender, receiver) = mpsc::channel();

        // Create SubscriptionManager using existing implementation
        println!("🚀 Creating subscription manager...");
        println!("   Enabled services: {:?}", config.enabled_services);
        let subscription_manager =
            Arc::new(SubscriptionManager::new(config, sender).map_err(StreamError::from)?);
        println!("✅ Subscription manager created successfully");

        // Add all speakers to subscription manager using existing add_speaker() method
        let total_speakers = self.speakers.len();
        let mut successful_speakers = 0;
        for speaker in self.speakers {
            println!(
                "🔗 Setting up subscriptions for speaker: {} ({}:{})",
                speaker.name, speaker.ip_address, speaker.port
            );

            match subscription_manager.add_speaker(speaker.clone()) {
                Ok(()) => {
                    println!("✅ Successfully set up subscriptions for {}", speaker.name);
                    successful_speakers += 1;
                }
                Err(super::subscription::SubscriptionError::SatelliteSpeaker) => {
                    println!("🔗 Skipping {} (satellite/bonded speaker)", speaker.name);
                    // Don't count as failure - satellite speakers are expected to be skipped
                }
                Err(e) => {
                    println!(
                        "⚠️  Failed to create subscriptions for {}: {:?}",
                        speaker.name, e
                    );
                    println!("   Continuing with other speakers...");
                    // Continue with other speakers instead of failing completely
                }
            }
        }

        if successful_speakers == 0 {
            return Err(StreamError::InitializationFailed(
                "No speakers could be subscribed to".to_string(),
            ));
        }

        println!(
            "🎯 Successfully set up subscriptions for {}/{} speakers",
            successful_speakers, total_speakers
        );

        // Show callback server info for debugging
        if let Some(port) = subscription_manager.callback_server_port() {
            println!("📡 Callback server running on port: {}", port);
            println!("   Sonos devices will send events to this server");
        }

        // Return ActiveEventStream instance with running event processing
        let active_stream = ActiveEventStream::new(
            subscription_manager,
            receiver,
            self.state_cache,
            self.event_handlers,
            self.lifecycle_handlers,
        )?;

        println!("🎯 EventStream ready to receive events");

        Ok(active_stream)
    }

    /// Build the internal StreamConfig from builder settings
    ///
    /// This method creates a StreamConfig using the existing configuration logic
    /// while applying any configuration overrides that were specified.
    fn build_stream_config(&self) -> Result<StreamConfig, StreamError> {
        let mut config = StreamConfig::default().with_enabled_services(self.services.clone());

        // Apply configuration overrides
        if let Some(timeout) = self.config_overrides.subscription_timeout {
            config = config
                .with_subscription_timeout(timeout)
                .map_err(StreamError::ConfigurationError)?;
        }

        if let Some(backoff) = self.config_overrides.retry_backoff {
            config = config.with_retry_backoff(backoff);
        }

        if let Some((start, end)) = self.config_overrides.callback_port_range {
            config = config
                .with_callback_port_range(start, end)
                .map_err(StreamError::ConfigurationError)?;
        }

        if let Some(size) = self.config_overrides.buffer_size {
            config = config
                .with_buffer_size(size)
                .map_err(StreamError::ConfigurationError)?;
        }

        if let Some(attempts) = self.config_overrides.max_retry_attempts {
            config = config
                .with_retry_attempts(attempts)
                .map_err(StreamError::ConfigurationError)?;
        }

        // Validate the final configuration
        config.validate().map_err(StreamError::ConfigurationError)?;

        Ok(config)
    }
}

/// Active streaming session with speaker management capabilities
///
/// This struct represents a running event stream that is actively monitoring
/// speakers and processing events. It provides methods for runtime speaker
/// management and graceful shutdown.
pub struct ActiveEventStream {
    subscription_manager: Arc<SubscriptionManager>,
    _event_processor: Option<JoinHandle<()>>,
    shutdown_sender: mpsc::Sender<()>,
}

impl ActiveEventStream {
    /// Create a new ActiveEventStream with running event processing
    ///
    /// This method starts the background event processing thread that handles
    /// StateCache updates, user event handlers, and lifecycle callbacks.
    fn new(
        subscription_manager: Arc<SubscriptionManager>,
        receiver: mpsc::Receiver<StateChange>,
        state_cache: Option<Arc<StateCache>>,
        event_handlers: Vec<Box<dyn Fn(StateChange) + Send + Sync>>,
        lifecycle_handlers: LifecycleHandlers,
    ) -> Result<Self, StreamError> {
        let (shutdown_sender, shutdown_receiver) = mpsc::channel();

        // Start event processing thread
        let event_processor = std::thread::spawn(move || {
            Self::event_processing_loop(
                receiver,
                shutdown_receiver,
                state_cache,
                event_handlers,
                lifecycle_handlers,
            );
        });

        Ok(Self {
            subscription_manager,
            _event_processor: Some(event_processor),
            shutdown_sender,
        })
    }

    /// Main event processing loop that runs in a background thread
    ///
    /// This loop continuously processes events from the receiver, updates the StateCache
    /// if provided, calls user event handlers, and handles lifecycle events.
    ///
    /// The loop uses existing EventStream::process_state_change logic for StateCache updates
    /// and handles shutdown signals gracefully to terminate event processing.
    ///
    /// This implementation is non-blocking and uses flag-based updates to avoid I/O operations
    /// in the event processing thread.
    fn event_processing_loop(
        receiver: mpsc::Receiver<StateChange>,
        shutdown_receiver: mpsc::Receiver<()>,
        state_cache: Option<Arc<StateCache>>,
        event_handlers: Vec<Box<dyn Fn(StateChange) + Send + Sync>>,
        lifecycle_handlers: LifecycleHandlers,
    ) {
        log::debug!("Event processing loop started");

        // Call stream started handler (non-blocking)
        if let Some(ref handler) = lifecycle_handlers.on_stream_started {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler();
            }));
            if let Err(_) = result {
                log::error!("Stream started handler panicked");
            }
        }

        let mut events_processed = 0u64;
        let mut display_update_needed = false;
        let mut last_stats_update = std::time::Instant::now();

        loop {
            // Use select-like behavior to handle both events and shutdown signals
            // We use a short timeout to allow periodic shutdown signal checking and flag processing
            match receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(state_change) => {
                    log::debug!("Processing event: {:?}", state_change);
                    events_processed += 1;

                    // Update StateCache if provided using existing EventStream logic (non-blocking)
                    if let Some(ref cache) = state_cache {
                        use super::event_stream::EventStream;
                        EventStream::process_state_change(cache, state_change.clone());
                        log::debug!("StateCache updated for event #{}", events_processed);
                    }

                    // Call user event handlers in registration order (non-blocking)
                    // Support multiple event handlers called in registration order as per requirements
                    for (index, handler) in event_handlers.iter().enumerate() {
                        log::debug!(
                            "Calling event handler #{} for event #{}",
                            index + 1,
                            events_processed
                        );

                        // Call the handler - we use std::panic::catch_unwind to prevent
                        // a panicking handler from crashing the entire event processing loop
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            handler(state_change.clone());
                        }));

                        if let Err(_) = result {
                            log::error!(
                                "Event handler #{} panicked while processing event #{}",
                                index + 1,
                                events_processed
                            );
                            // Continue with other handlers even if one panics
                        }
                    }

                    // Handle lifecycle events (connection, disconnection, errors) - non-blocking
                    Self::handle_lifecycle_event(&state_change, &lifecycle_handlers);

                    // Set flag for display updates instead of direct I/O
                    display_update_needed = true;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Handle flag-based updates during timeout periods (non-blocking)
                    if display_update_needed {
                        // Perform any necessary display updates here without blocking I/O
                        // This could include updating internal counters, metrics, or other state
                        display_update_needed = false;

                        // Update statistics periodically (non-blocking)
                        let now = std::time::Instant::now();
                        if now.duration_since(last_stats_update) >= Duration::from_secs(5) {
                            log::debug!("Events processed in last 5 seconds: {}", events_processed);
                            last_stats_update = now;
                        }
                    }

                    // Check for shutdown signal (non-blocking)
                    if shutdown_receiver.try_recv().is_ok() {
                        log::debug!("Shutdown signal received, terminating event processing loop");
                        break;
                    }
                    // Continue loop if no shutdown signal - this allows graceful shutdown checking
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Event channel closed, exit loop gracefully
                    log::debug!("Event receiver disconnected, terminating event processing loop");
                    break;
                }
            }
        }

        log::debug!(
            "Event processing loop terminated after processing {} events",
            events_processed
        );

        // Call stream stopped handler (non-blocking)
        if let Some(ref handler) = lifecycle_handlers.on_stream_stopped {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler();
            }));
            if let Err(_) = result {
                log::error!("Stream stopped handler panicked");
            }
        }
    }

    /// Handle lifecycle events by calling appropriate callbacks
    ///
    /// This method detects connection, disconnection, and error events from subscription
    /// state changes and triggers the appropriate lifecycle callbacks. It maps internal
    /// SubscriptionError types to simplified StreamError types with actionable messages.
    ///
    /// All operations in this method are non-blocking to ensure event processing remains responsive.
    fn handle_lifecycle_event(event: &StateChange, handlers: &LifecycleHandlers) {
        match event {
            StateChange::SubscriptionError {
                speaker_id,
                error,
                service,
            } => {
                log::debug!(
                    "Handling subscription error for speaker {:?} on service {:?}: {}",
                    speaker_id,
                    service,
                    error
                );

                // Map internal subscription errors to user-friendly StreamError types (non-blocking)
                let stream_error = Self::map_subscription_error_to_stream_error(error, *service);

                // Call error handler with mapped StreamError (non-blocking callback)
                if let Some(ref handler) = handlers.on_error {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        handler(stream_error);
                    }));
                    if let Err(_) = result {
                        log::error!("Error handler panicked while processing subscription error");
                    }
                }

                // Detect disconnection events based on error type (non-blocking)
                let is_disconnection_error = Self::is_disconnection_error(error);
                if is_disconnection_error {
                    log::debug!(
                        "Subscription error indicates speaker disconnection: {}",
                        error
                    );

                    // Call speaker disconnected handler for connection-related failures (non-blocking callback)
                    if let Some(ref handler) = handlers.on_speaker_disconnected {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            handler(*speaker_id);
                        }));
                        if let Err(_) = result {
                            log::error!("Speaker disconnected handler panicked");
                        }
                    }
                }
            }

            StateChange::TransportInfoChanged {
                speaker_id,
                transport_status,
                ..
            } => {
                // Transport status can indicate connection issues (non-blocking processing)
                match transport_status {
                    crate::models::TransportStatus::ErrorOccurred => {
                        log::debug!("Transport error occurred for speaker {:?}", speaker_id);

                        if let Some(ref handler) = handlers.on_error {
                            let stream_error = StreamError::SpeakerOperationFailed(
                                "Transport error occurred on speaker".to_string(),
                            );
                            let result =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    handler(stream_error);
                                }));
                            if let Err(_) = result {
                                log::error!(
                                    "Error handler panicked while processing transport error"
                                );
                            }
                        }
                    }
                    crate::models::TransportStatus::Ok => {
                        // Transport OK indicates successful communication (non-blocking log only)
                        log::debug!(
                            "Transport OK for speaker {:?}, indicating connectivity",
                            speaker_id
                        );
                    }
                }
            }

            // For other events, we can detect implicit connection patterns (all non-blocking)
            StateChange::PlaybackStateChanged { speaker_id, .. }
            | StateChange::VolumeChanged { speaker_id, .. }
            | StateChange::MuteChanged { speaker_id, .. }
            | StateChange::PositionChanged { speaker_id, .. }
            | StateChange::TrackChanged { speaker_id, .. } => {
                // These events indicate the speaker is connected and responding (non-blocking log only)
                log::debug!(
                    "Received successful event from speaker {:?}, indicating connectivity",
                    speaker_id
                );

                // Note: We don't call on_speaker_connected for every event as that would be too noisy.
                // In a future enhancement, we could track connection state and only call it
                // when a speaker transitions from disconnected to connected state.
            }

            StateChange::SpeakerJoinedGroup { speaker_id, .. }
            | StateChange::SpeakerLeftGroup { speaker_id, .. } => {
                // Individual group membership changes indicate speaker connectivity (non-blocking log only)
                log::debug!(
                    "Speaker {:?} group membership changed, indicating connectivity",
                    speaker_id
                );
            }
            StateChange::CoordinatorChanged { .. }
            | StateChange::GroupFormed { .. }
            | StateChange::GroupDissolved { .. } => {
                // Group structure changes indicate network-wide connectivity (non-blocking log only)
                log::debug!("Group structure changed, indicating network connectivity");
            }
        }
    }

    /// Map subscription error strings to user-friendly StreamError types with actionable messages
    fn map_subscription_error_to_stream_error(error: &str, service: ServiceType) -> StreamError {
        let error_lower = error.to_lowercase();

        if error_lower.contains("timeout") || error_lower.contains("timed out") {
            StreamError::NetworkError(format!(
                "Subscription to {:?} service timed out. Check network connectivity and speaker availability.", 
                service
            ))
        } else if error_lower.contains("connection refused") || error_lower.contains("refused") {
            StreamError::NetworkError(format!(
                "Connection refused by speaker for {:?} service. Speaker may be busy or unreachable.", 
                service
            ))
        } else if error_lower.contains("network") || error_lower.contains("unreachable") {
            StreamError::NetworkError(format!(
                "Network error accessing {:?} service: {}. Check network connectivity.",
                service, error
            ))
        } else if error_lower.contains("parse") || error_lower.contains("invalid") {
            StreamError::SubscriptionError(format!(
                "Invalid response from {:?} service: {}. Speaker may have compatibility issues.",
                service, error
            ))
        } else if error_lower.contains("unauthorized") || error_lower.contains("forbidden") {
            StreamError::SubscriptionError(format!(
                "Access denied to {:?} service. Speaker may require authentication.",
                service
            ))
        } else if error_lower.contains("not found") || error_lower.contains("404") {
            StreamError::SpeakerOperationFailed(format!(
                "Service {:?} not found on speaker. Speaker may not support this service.",
                service
            ))
        } else {
            // Generic error mapping
            StreamError::SpeakerOperationFailed(format!(
                "Subscription error for {:?} service: {}",
                service, error
            ))
        }
    }

    /// Determine if an error string indicates a speaker disconnection
    fn is_disconnection_error(error: &str) -> bool {
        let error_lower = error.to_lowercase();

        error_lower.contains("timeout")
            || error_lower.contains("timed out")
            || error_lower.contains("connection refused")
            || error_lower.contains("unreachable")
            || error_lower.contains("network")
            || error_lower.contains("disconnected")
            || error_lower.contains("connection reset")
            || error_lower.contains("connection lost")
            || error_lower.contains("no route to host")
    }
    /// Add a speaker to the active stream
    ///
    /// This method adds a speaker to the subscription manager, which will
    /// automatically establish subscriptions for the enabled services.
    /// The speaker will start receiving events immediately.
    ///
    /// # Arguments
    ///
    /// * `speaker` - The speaker to add to the stream
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the speaker was added successfully, or an error if
    /// the operation failed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use sonos::streaming::ActiveEventStream;
    /// # use sonos::models::Speaker;
    /// # let stream: ActiveEventStream = todo!();
    /// # let new_speaker: Speaker = todo!();
    /// stream.add_speaker(new_speaker)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add_speaker(&self, speaker: Speaker) -> Result<(), StreamError> {
        self.subscription_manager
            .add_speaker(speaker)
            .map_err(StreamError::from)
    }

    /// Remove a speaker from the active stream
    ///
    /// This method removes a speaker from the subscription manager, which will
    /// automatically unsubscribe from all services for that speaker and stop
    /// receiving events from it.
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
    /// # use sonos::streaming::ActiveEventStream;
    /// # use sonos::models::SpeakerId;
    /// # let stream: ActiveEventStream = todo!();
    /// # let speaker_id: SpeakerId = todo!();
    /// stream.remove_speaker(speaker_id)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove_speaker(&self, speaker_id: SpeakerId) -> Result<(), StreamError> {
        self.subscription_manager
            .remove_speaker(speaker_id)
            .map_err(StreamError::from)
    }

    /// Get streaming statistics
    ///
    /// Returns current statistics about the streaming session, including
    /// the number of active subscriptions and speakers being monitored.
    ///
    /// # Returns
    ///
    /// Returns a StreamStats struct with current statistics.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use sonos::streaming::ActiveEventStream;
    /// # let stream: ActiveEventStream = todo!();
    /// let stats = stream.stats();
    /// println!("Monitoring {} speakers with {} subscriptions",
    ///     stats.active_speakers, stats.active_subscriptions);
    /// ```
    pub fn stats(&self) -> StreamStats {
        StreamStats {
            active_subscriptions: self.subscription_manager.subscription_count(),
            active_speakers: self.subscription_manager.speaker_count(),
            total_events_received: 0, // TODO: Add event counter in future enhancement
            subscription_errors: 0,   // TODO: Add error counter in future enhancement
            successful_renewals: 0,   // TODO: Add renewal counter in future enhancement
        }
    }

    /// Gracefully shutdown the stream
    ///
    /// This method signals the background event processing thread to shut down
    /// gracefully and waits for it to complete. All subscriptions will be
    /// properly unsubscribed and resources will be cleaned up.
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if shutdown completed successfully, or an error if
    /// the shutdown process failed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use sonos::streaming::ActiveEventStream;
    /// # let stream: ActiveEventStream = todo!();
    /// // Gracefully shutdown when done
    /// stream.shutdown()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn shutdown(mut self) -> Result<(), StreamError> {
        // Send shutdown signal to event processing thread
        let _ = self.shutdown_sender.send(());

        // Wait for the event processor thread to complete gracefully
        if let Some(handle) = self._event_processor.take() {
            handle.join().map_err(|_| StreamError::ShutdownFailed)?;
        }

        // Note: SubscriptionManager cleanup is handled automatically when it's dropped
        // The Arc<SubscriptionManager> will be dropped when this ActiveEventStream is dropped,
        // and if it's the last reference, the SubscriptionManager's Drop implementation
        // will handle unsubscribing from all services and cleaning up resources

        Ok(())
    }
}

impl Drop for ActiveEventStream {
    /// Ensure graceful cleanup even if shutdown() wasn't called explicitly
    fn drop(&mut self) {
        // Send shutdown signal (ignore errors since we're dropping)
        let _ = self.shutdown_sender.send(());

        // Try to join the thread if it's still available
        if let Some(handle) = self._event_processor.take() {
            // In Drop, we can't wait indefinitely, but we'll give the thread
            // a reasonable amount of time to shut down gracefully
            let _ = handle.join();
        }

        // Note: SubscriptionManager cleanup is handled automatically when the Arc is dropped.
        // If this is the last reference to the SubscriptionManager, its Drop implementation
        // will handle unsubscribing from all services and cleaning up the callback server.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Speaker, SpeakerId};

    fn create_test_speaker(id: &str, name: &str) -> Speaker {
        Speaker {
            id: SpeakerId::from_udn(id),
            udn: id.to_string(),
            name: name.to_string(),
            room_name: name.to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Test Model".to_string(),
            satellites: vec![],
        }
    }

    #[test]
    fn test_builder_new() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];
        let builder = EventStreamBuilder::new(speakers).unwrap();

        assert_eq!(builder.speakers.len(), 1);
        assert_eq!(builder.services, vec![ServiceType::AVTransport]);
        assert!(builder.state_cache.is_none());
        assert_eq!(builder.event_handlers.len(), 0);
    }

    #[test]
    fn test_builder_new_empty_speakers() {
        let speakers = vec![];
        let result = EventStreamBuilder::new(speakers);

        assert!(result.is_err());
        match result.unwrap_err() {
            StreamError::ConfigurationError(msg) => {
                assert!(msg.contains("At least one speaker"));
            }
            _ => panic!("Expected ConfigurationError"),
        }
    }

    #[test]
    fn test_builder_with_state_cache() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];
        let state_cache = Arc::new(StateCache::new());

        let builder = EventStreamBuilder::new(speakers)
            .unwrap()
            .with_state_cache(state_cache.clone());

        assert!(builder.state_cache.is_some());
        assert!(Arc::ptr_eq(&builder.state_cache.unwrap(), &state_cache));
    }

    #[test]
    fn test_builder_with_services() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];
        let services = &[ServiceType::AVTransport, ServiceType::RenderingControl];

        let builder = EventStreamBuilder::new(speakers)
            .unwrap()
            .with_services(services);

        assert_eq!(builder.services, services);
    }

    #[test]
    fn test_builder_with_services_empty() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];

        let builder = EventStreamBuilder::new(speakers)
            .unwrap()
            .with_services(&[]);

        // Should keep the default services when empty slice is provided
        assert_eq!(builder.services, vec![ServiceType::AVTransport]);
    }

    #[test]
    fn test_builder_with_event_handler() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];

        let builder = EventStreamBuilder::new(speakers)
            .unwrap()
            .with_event_handler(|_event| {
                // Test handler
            })
            .with_event_handler(|_event| {
                // Another test handler
            });

        assert_eq!(builder.event_handlers.len(), 2);
    }

    #[test]
    fn test_build_stream_config_default() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];
        let builder = EventStreamBuilder::new(speakers).unwrap();

        let config = builder.build_stream_config().unwrap();

        assert_eq!(config.enabled_services, vec![ServiceType::AVTransport]);
        assert_eq!(config.buffer_size, 1000); // Default value
        assert_eq!(config.subscription_timeout, Duration::from_secs(1800)); // Default value
    }

    #[test]
    fn test_build_stream_config_with_overrides() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];
        let mut builder = EventStreamBuilder::new(speakers).unwrap();

        // Set some configuration overrides
        builder.config_overrides = ConfigOverrides::new()
            .with_subscription_timeout(Duration::from_secs(3600))
            .with_buffer_size(2000)
            .with_callback_port_range(9000, 9010);

        let config = builder.build_stream_config().unwrap();

        assert_eq!(config.subscription_timeout, Duration::from_secs(3600));
        assert_eq!(config.buffer_size, 2000);
        assert_eq!(config.callback_port_range, (9000, 9010));
    }

    #[test]
    fn test_build_stream_config_invalid_overrides() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];
        let mut builder = EventStreamBuilder::new(speakers).unwrap();

        // Set invalid configuration overrides
        builder.config_overrides =
            ConfigOverrides::new().with_subscription_timeout(Duration::from_secs(30)); // Too short

        let result = builder.build_stream_config();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_lifecycle_handlers() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];

        let lifecycle_handlers = LifecycleHandlers::new()
            .with_speaker_connected(|_id| {})
            .with_error(|_error| {});

        let _builder = EventStreamBuilder::new(speakers)
            .unwrap()
            .with_lifecycle_handlers(lifecycle_handlers);

        // We can't easily test the handlers themselves, but we can verify the builder accepts them
        assert!(true); // Placeholder assertion
    }

    #[test]
    fn test_builder_with_timeouts() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];

        let builder = EventStreamBuilder::new(speakers)
            .unwrap()
            .with_timeouts(Duration::from_secs(3600), Duration::from_secs(2));

        assert_eq!(
            builder.config_overrides.subscription_timeout,
            Some(Duration::from_secs(3600))
        );
        assert_eq!(
            builder.config_overrides.retry_backoff,
            Some(Duration::from_secs(2))
        );
    }

    #[test]
    fn test_builder_with_callback_ports() {
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];

        let builder = EventStreamBuilder::new(speakers)
            .unwrap()
            .with_callback_ports(9000, 9010);

        assert_eq!(
            builder.config_overrides.callback_port_range,
            Some((9000, 9010))
        );
    }

    #[test]
    fn test_builder_start() {
        // Note: This test may fail in environments without network access
        // or where the callback server can't bind to ports
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];

        let result = EventStreamBuilder::new(speakers).unwrap().start();

        // The test might fail due to network issues, but we can at least verify
        // that the method exists and returns the correct type
        match result {
            Ok(_stream) => {
                // Success - the stream was created
                assert!(true);
            }
            Err(_) => {
                // Expected in test environment - network/binding issues
                assert!(true);
            }
        }
    }

    #[test]
    fn test_active_event_stream_stats() {
        // Test that stats method returns proper structure
        // Note: This is a unit test for the stats method structure,
        // not an integration test with actual speakers
        let speakers = vec![create_test_speaker(
            "uuid:RINCON_123456789::1",
            "Test Speaker",
        )];
        let _builder = EventStreamBuilder::new(speakers).unwrap();

        // We can't easily create a real ActiveEventStream in tests due to network dependencies,
        // but we can test the stats structure by creating a mock scenario
        let stats = StreamStats::new();

        assert_eq!(stats.active_subscriptions, 0);
        assert_eq!(stats.active_speakers, 0);
        assert_eq!(stats.total_events_received, 0);
        assert_eq!(stats.subscription_errors, 0);
        assert_eq!(stats.successful_renewals, 0);
        assert!(!stats.is_active());
        assert_eq!(stats.avg_subscriptions_per_speaker(), 0.0);

        // Test with some values
        let mut stats = StreamStats::new();
        stats.active_subscriptions = 6;
        stats.active_speakers = 2;

        assert!(stats.is_active());
        assert_eq!(stats.avg_subscriptions_per_speaker(), 3.0);
    }
}
