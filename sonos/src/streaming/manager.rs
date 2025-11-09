use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc as tokio_mpsc;

use super::av_transport::AVTransportSubscription;
use super::callback_server::CallbackServer;
use super::rendering_control::RenderingControlSubscription;
use super::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use super::types::{RawEvent, ServiceType, StreamConfig, SubscriptionConfig, SubscriptionId, SubscriptionScope};
use crate::model::{Speaker, SpeakerId, StateChange};

/// Manages UPnP subscriptions across multiple speakers
///
/// The SubscriptionManager coordinates subscriptions for all discovered speakers,
/// handles subscription lifecycle (creation, renewal, cleanup), and routes events
/// to the unified event stream.
pub struct SubscriptionManager {
    /// Configuration for the subscription system
    config: StreamConfig,
    /// Channel sender for forwarding events to the EventStream
    event_sender: mpsc::Sender<StateChange>,
    /// Thread-safe storage for speakers and their subscriptions
    speakers: Arc<RwLock<HashMap<SpeakerId, Speaker>>>,
    /// Thread-safe storage for active subscriptions
    subscriptions: Arc<RwLock<HashMap<SubscriptionId, Box<dyn ServiceSubscription>>>>,
    /// Registry of active network-wide subscriptions (service_type -> subscription_id)
    network_subscriptions: Arc<RwLock<HashMap<ServiceType, SubscriptionId>>>,
    /// HTTP callback server for receiving UPnP events
    callback_server: Arc<RwLock<Option<CallbackServer>>>,
    /// Background thread handle for subscription management
    management_thread: Option<JoinHandle<()>>,
    /// Channel for sending raw events from callback server to subscription manager
    raw_event_sender: Option<tokio_mpsc::UnboundedSender<RawEvent>>,
    /// Shutdown signal for background threads
    shutdown_sender: Option<mpsc::Sender<()>>,
}

impl SubscriptionManager {
    /// Create a new SubscriptionManager
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the subscription system
    /// * `event_sender` - Channel sender for forwarding events to the EventStream
    ///
    /// # Returns
    ///
    /// Returns a new SubscriptionManager instance or an error if initialization fails.
    pub fn new(
        config: StreamConfig,
        event_sender: mpsc::Sender<StateChange>,
    ) -> SubscriptionResult<Self> {
        // Validate configuration
        config
            .validate()
            .map_err(|e| SubscriptionError::InvalidConfiguration(e))?;

        // Create callback server for receiving UPnP events
        println!("🌐 Creating callback server for port range {:?}...", config.callback_port_range);
        let (raw_event_sender, raw_event_receiver) = tokio_mpsc::unbounded_channel();
        let mut callback_server =
            CallbackServer::new(config.callback_port_range, raw_event_sender.clone())
                .map_err(|e| SubscriptionError::CallbackServerError(e.to_string()))?;

        println!("🚀 Starting callback server on port {}...", callback_server.port());
        // Start the callback server
        callback_server
            .start()
            .map_err(|e| SubscriptionError::CallbackServerError(e.to_string()))?;

        println!("✅ Callback server started successfully");
        println!("📡 Base URL: {}", callback_server.base_url());
        log::info!("Callback server started on port {}", callback_server.port());

        let speakers = Arc::new(RwLock::new(HashMap::new()));
        let subscriptions = Arc::new(RwLock::new(HashMap::new()));
        let network_subscriptions = Arc::new(RwLock::new(HashMap::new()));
        let callback_server_arc = Arc::new(RwLock::new(Some(callback_server)));

        // Create shutdown channel for background threads
        let (shutdown_sender, shutdown_receiver) = mpsc::channel();

        // Start background thread for processing raw events and subscription management
        let management_thread = Self::start_management_thread(
            Arc::clone(&subscriptions),
            event_sender.clone(),
            raw_event_receiver,
            shutdown_receiver,
            config.clone(),
        );

        Ok(Self {
            config,
            event_sender,
            speakers,
            subscriptions,
            network_subscriptions,
            callback_server: callback_server_arc,
            management_thread: Some(management_thread),
            raw_event_sender: Some(raw_event_sender),
            shutdown_sender: Some(shutdown_sender),
        })
    }

    /// Start the background thread for subscription management and event processing
    fn start_management_thread(
        subscriptions: Arc<RwLock<HashMap<SubscriptionId, Box<dyn ServiceSubscription>>>>,
        event_sender: mpsc::Sender<StateChange>,
        mut raw_event_receiver: tokio_mpsc::UnboundedReceiver<RawEvent>,
        shutdown_receiver: mpsc::Receiver<()>,
        config: StreamConfig,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();

            rt.block_on(async {
                let mut renewal_interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute
                let mut shutdown_check_interval = tokio::time::interval(Duration::from_millis(100)); // Check shutdown every 100ms

                loop {
                    tokio::select! {
                        // Process raw events from callback server
                        Some(raw_event) = raw_event_receiver.recv() => {
                            let subscriptions_clone = Arc::clone(&subscriptions);
                            let event_sender_clone = event_sender.clone();
                            
                            // Use spawn_blocking to handle potentially blocking XML parsing
                            tokio::task::spawn_blocking(move || {
                                Self::process_raw_event(&subscriptions_clone, &event_sender_clone, raw_event);
                            });
                        }

                        // Periodic subscription renewal check
                        _ = renewal_interval.tick() => {
                            Self::check_subscription_renewals(&subscriptions, &config);
                        }

                        // Check for shutdown signal periodically
                        _ = shutdown_check_interval.tick() => {
                            // Check if we received a shutdown signal (non-blocking)
                            match shutdown_receiver.try_recv() {
                                Ok(()) => {
                                    log::info!("Subscription manager shutting down");
                                    break;
                                }
                                Err(mpsc::TryRecvError::Empty) => {
                                    // No shutdown signal, continue
                                }
                                Err(mpsc::TryRecvError::Disconnected) => {
                                    log::info!("Shutdown channel disconnected, shutting down");
                                    break;
                                }
                            }
                        }
                    }
                }
            });
        })
    }

    /// Process a raw event from the callback server
    /// 
    /// This method is non-blocking and avoids I/O operations in the event processing path.
    /// Console output has been replaced with logging to prevent blocking I/O.
    fn process_raw_event(
        subscriptions: &Arc<RwLock<HashMap<SubscriptionId, Box<dyn ServiceSubscription>>>>,
        event_sender: &mpsc::Sender<StateChange>,
        raw_event: RawEvent,
    ) {
        log::debug!("Processing raw event in subscription manager");
        log::debug!("Subscription ID: {}", raw_event.subscription_id);
        log::debug!("Event XML length: {} bytes", raw_event.event_xml.len());
        
        let subscriptions_guard = match subscriptions.read() {
            Ok(guard) => guard,
            Err(_) => {
                log::error!("Failed to acquire read lock on subscriptions");
                return;
            }
        };

        log::debug!("Current subscriptions in manager: {}", subscriptions_guard.len());
        for (id, subscription) in subscriptions_guard.iter() {
            log::debug!("Subscription {} -> Speaker: {:?}, Service: {:?}, Active: {}", 
                id, subscription.speaker_id(), subscription.service_type(), subscription.is_active());
        }

        if let Some(subscription) = subscriptions_guard.get(&raw_event.subscription_id) {
            log::debug!("Found subscription in manager, parsing event");
            
            match subscription.parse_event(&raw_event.event_xml) {
                Ok(state_changes) => {
                    log::debug!("Successfully parsed {} state changes", state_changes.len());
                    for (i, change) in state_changes.iter().enumerate() {
                        log::debug!("State change {}: {:?}", i + 1, change);
                        if let Err(e) = event_sender.send(change.clone()) {
                            log::error!("Failed to send state change {}: {}", i + 1, e);
                        } else {
                            log::debug!("Sent state change {} successfully", i + 1);
                        }
                    }
                }
                Err(e) => {
                    let service_type = subscription.service_type();
                    let service_scope = service_type.subscription_scope();
                    
                    // Log with service type identification for better error isolation
                    log::warn!(
                        "[{:?}] Failed to parse event for {:?} subscription {}: {}",
                        service_scope,
                        service_type,
                        raw_event.subscription_id,
                        e
                    );

                    // Send error event with service scope identification (non-blocking channel send)
                    let error_change = StateChange::SubscriptionError {
                        speaker_id: subscription.speaker_id().clone(),
                        service: service_type,
                        error: format!("{:?} service parse error: {}", service_scope, e),
                    };

                    if let Err(send_err) = event_sender.send(error_change) {
                        log::error!("[{:?}] Failed to send error state change for {:?}: {}", service_scope, service_type, send_err);
                    } else {
                        log::debug!("[{:?}] Sent error state change for {:?} successfully", service_scope, service_type);
                    }
                }
            }
        } else {
            log::warn!(
                "Received event for unknown subscription: {}",
                raw_event.subscription_id
            );
        }
        
        log::debug!("Finished processing raw event");
    }

    /// Check subscriptions for renewal needs and representative speaker availability
    fn check_subscription_renewals(
        subscriptions: &Arc<RwLock<HashMap<SubscriptionId, Box<dyn ServiceSubscription>>>>,
        config: &StreamConfig,
    ) {
        let mut subscriptions_guard = match subscriptions.write() {
            Ok(guard) => guard,
            Err(_) => {
                log::error!("Failed to acquire write lock on subscriptions for renewal check");
                return;
            }
        };

        let mut renewals_needed = Vec::new();
        let mut expired_subscriptions = Vec::new();

        // Collect subscriptions that need renewal or have expired
        for (id, subscription) in subscriptions_guard.iter() {
            if subscription.is_active() {
                if subscription.needs_renewal() {
                    renewals_needed.push(*id);
                } else if Self::is_subscription_expired(subscription, config) {
                    expired_subscriptions.push(*id);
                }
            }
        }

        // Renew subscriptions that need renewal
        for subscription_id in renewals_needed {
            if let Some(subscription) = subscriptions_guard.get_mut(&subscription_id) {
                let service_type = subscription.service_type();
                let service_scope = service_type.subscription_scope();
                
                if Self::renew_subscription_with_retry(subscription, subscription_id, config) {
                    log::debug!("[{:?}] Successfully renewed {:?} subscription {}", service_scope, service_type, subscription_id);
                } else {
                    log::warn!(
                        "[{:?}] Failed to renew {:?} subscription {} after all retry attempts. Service will be isolated.",
                        service_scope,
                        service_type,
                        subscription_id
                    );
                    // Mark subscription as inactive
                    let _ = subscription.on_subscription_state_changed(false);
                }
            }
        }

        // Mark expired subscriptions as inactive
        for subscription_id in expired_subscriptions {
            if let Some(subscription) = subscriptions_guard.get_mut(&subscription_id) {
                let service_type = subscription.service_type();
                let service_scope = service_type.subscription_scope();
                
                log::warn!("[{:?}] {:?} subscription {} has expired and will be isolated", service_scope, service_type, subscription_id);
                let _ = subscription.on_subscription_state_changed(false);
            }
        }
    }

    /// Check if a subscription has expired based on configuration
    fn is_subscription_expired(
        subscription: &Box<dyn ServiceSubscription>,
        config: &StreamConfig,
    ) -> bool {
        if let Some(last_renewal) = subscription.last_renewal() {
            if let Ok(elapsed) = last_renewal.elapsed() {
                let expiry_time = config
                    .subscription_timeout
                    .saturating_add(Duration::from_secs(60)); // 1 minute grace period
                return elapsed >= expiry_time;
            }
        }
        false
    }

    /// Get the callback URL for a specific subscription
    fn get_callback_url(&self, subscription_id: SubscriptionId) -> String {
        let callback_server = self.callback_server.read().unwrap();
        if let Some(server) = callback_server.as_ref() {
            format!("{}/callback/{}", server.base_url(), subscription_id)
        } else {
            format!("http://127.0.0.1:8080/callback/{}", subscription_id)
        }
    }

    /// Create subscriptions for a speaker for all enabled services
    fn create_subscriptions_for_speaker(
        &self,
        speaker: &Speaker,
    ) -> SubscriptionResult<Vec<SubscriptionId>> {
        // Check if this speaker already has all required PerSpeaker subscriptions
        let per_speaker_services: Vec<ServiceType> = self.config.enabled_services.iter()
            .filter(|s| s.subscription_scope() == SubscriptionScope::PerSpeaker)
            .cloned()
            .collect();

        let existing_per_speaker_subscriptions = {
            let subscriptions = self.subscriptions.read().unwrap();
            subscriptions.iter()
                .filter(|(_, sub)| {
                    sub.speaker_id() == speaker.get_id() && 
                    sub.subscription_scope() == SubscriptionScope::PerSpeaker
                })
                .count()
        };

        // Only skip if we have PerSpeaker services configured AND they're all already created
        if !per_speaker_services.is_empty() && existing_per_speaker_subscriptions >= per_speaker_services.len() {
            log::info!(
                "Speaker {} already has all {} required PerSpeaker subscriptions, skipping creation to prevent duplicates.",
                speaker.name,
                per_speaker_services.len()
            );
            return Ok(Vec::new());
        }

        let mut subscription_ids = Vec::new();
        let mut satellite_errors = 0;
        let mut total_attempts = 0;
        let subscription_config = SubscriptionConfig::from_stream_config(&self.config);

        // Process PerSpeaker services first to avoid conflicts with NetworkWide logic
        for service_type in &self.config.enabled_services {
            if service_type.subscription_scope() == SubscriptionScope::PerSpeaker {
                total_attempts += 1;
                
                // Handle per-speaker services
                match self.create_subscription_for_service(
                    speaker,
                    *service_type,
                    subscription_config.clone(),
                ) {
                    Ok(subscription_id) => {
                        subscription_ids.push(subscription_id);
                        log::info!(
                            "Created {:?} subscription {} for speaker {}",
                            service_type,
                            subscription_id,
                            speaker.name
                        );
                    }
                    Err(SubscriptionError::SatelliteSpeaker) => {
                        satellite_errors += 1;
                        log::debug!(
                            "Speaker {} returned 503 for {:?} service (likely satellite speaker)",
                            speaker.name,
                            service_type
                        );
                    }
                    Err(e) => {
                        // Use isolated error handling for PerSpeaker services
                        self.handle_service_failure(*service_type, &speaker.name, e);
                    }
                }
            }
        }

        // Then process NetworkWide services separately
        for service_type in &self.config.enabled_services {
            if service_type.subscription_scope() == SubscriptionScope::NetworkWide {
                total_attempts += 1;
                // Simplified network-wide service handling
                println!("🌐 Attempting to create {:?} network-wide subscription for speaker {}", service_type, speaker.name);
                match self.create_simple_network_wide_subscription(speaker, *service_type, subscription_config.clone()) {
                    Ok(Some(subscription_id)) => {
                        subscription_ids.push(subscription_id);
                        log::info!(
                            "Created/reused {:?} network-wide subscription {} for speaker {}",
                            service_type,
                            subscription_id,
                            speaker.name
                        );
                    }
                    Ok(None) => {
                        log::debug!(
                            "Reusing existing {:?} network-wide subscription for speaker {}",
                            service_type,
                            speaker.name
                        );
                    }
                    Err(SubscriptionError::SatelliteSpeaker) => {
                        satellite_errors += 1;
                        log::debug!(
                            "Speaker {} returned 503 for {:?} service (likely satellite speaker)",
                            speaker.name,
                            service_type
                        );
                    }
                    Err(e) => {
                        // Use isolated error handling for NetworkWide services
                        self.handle_service_failure(*service_type, &speaker.name, e);
                    }
                }
            }
        }

        // If all services returned 503 (satellite speaker error), propagate that error
        if satellite_errors == total_attempts && total_attempts > 0 {
            return Err(SubscriptionError::SatelliteSpeaker);
        }

        Ok(subscription_ids)
    }

    /// Simplified network-wide service subscription creation
    /// 
    /// Ultra-simple approach: just check if we already have a subscription for this service type.
    /// If not, create one using the current speaker. No failover, no recovery, no complexity.
    fn create_simple_network_wide_subscription(
        &self,
        speaker: &Speaker,
        service_type: ServiceType,
        config: SubscriptionConfig,
    ) -> SubscriptionResult<Option<SubscriptionId>> {
        // Check if we already have a network-wide subscription for this service
        let existing_subscription_id = {
            let network_subscriptions = self.network_subscriptions.read().unwrap();
            network_subscriptions.get(&service_type).copied()
        };

        if let Some(subscription_id) = existing_subscription_id {
            log::debug!(
                "Reusing existing {:?} network-wide subscription {} (no availability checks)",
                service_type,
                subscription_id
            );
            return Ok(None); // Always reuse existing subscription
        }

        // Create a new network-wide subscription using the current speaker
        log::info!(
            "Creating new {:?} network-wide subscription using speaker {}",
            service_type,
            speaker.name
        );

        match self.create_subscription_for_service(speaker, service_type, config) {
            Ok(subscription_id) => {
                // Register this as a network-wide subscription
                {
                    let mut network_subscriptions = self.network_subscriptions.write().unwrap();
                    network_subscriptions.insert(service_type, subscription_id);
                }
                
                log::info!(
                    "Created new {:?} network-wide subscription {} using speaker {}",
                    service_type,
                    subscription_id,
                    speaker.name
                );
                
                Ok(Some(subscription_id))
            }
            Err(e) => {
                // Use isolated error handling for NetworkWide service creation failures
                self.handle_service_failure(service_type, &speaker.name, e.clone());
                Err(e)
            }
        }
    }



    /// Clean up an inactive network-wide subscription from the registry
    fn cleanup_inactive_network_subscription(&self, service_type: ServiceType) {
        let mut network_subscriptions = self.network_subscriptions.write().unwrap();
        if let Some(subscription_id) = network_subscriptions.remove(&service_type) {
            log::debug!(
                "Cleaned up inactive {:?} network-wide subscription {}",
                service_type,
                subscription_id
            );
        }
    }

    /// Handle service failure with proper error isolation
    /// 
    /// This method ensures that PerSpeaker service failures don't affect NetworkWide services
    /// and vice versa, providing proper error logging with service type identification.
    fn handle_service_failure(&self, service_type: ServiceType, speaker_name: &str, error: SubscriptionError) {
        match service_type.subscription_scope() {
            SubscriptionScope::PerSpeaker => {
                // Log error but continue with other speakers - don't affect NetworkWide services
                log::warn!(
                    "[PerSpeaker] Service {:?} failed for speaker '{}': {}. Other services will continue normally.",
                    service_type,
                    speaker_name,
                    error
                );
                
                // Send isolated error event for PerSpeaker service
                let error_change = StateChange::SubscriptionError {
                    speaker_id: crate::model::SpeakerId::new(&format!("uuid:RINCON_{}::1", speaker_name)),
                    service: service_type,
                    error: format!("PerSpeaker service failure: {}", error),
                };

                if let Err(send_err) = self.event_sender.send(error_change) {
                    log::error!("[PerSpeaker] Failed to send isolated error event for {:?}: {}", service_type, send_err);
                }
            }
            SubscriptionScope::NetworkWide => {
                // Clean up failed NetworkWide subscription and log error
                self.cleanup_failed_network_subscription(service_type);
                log::error!(
                    "[NetworkWide] Service {:?} failed using speaker '{}': {}. Attempting graceful degradation.",
                    service_type,
                    speaker_name,
                    error
                );

                // Send isolated error event for NetworkWide service
                let error_change = StateChange::SubscriptionError {
                    speaker_id: crate::model::SpeakerId::new(&format!("uuid:RINCON_{}::1", speaker_name)),
                    service: service_type,
                    error: format!("NetworkWide service failure: {}", error),
                };

                if let Err(send_err) = self.event_sender.send(error_change) {
                    log::error!("[NetworkWide] Failed to send isolated error event for {:?}: {}", service_type, send_err);
                }
            }
        }
    }

    /// Clean up a failed network-wide subscription
    /// 
    /// This method handles cleanup when a NetworkWide service fails, ensuring
    /// the registry remains consistent and doesn't affect PerSpeaker services.
    fn cleanup_failed_network_subscription(&self, service_type: ServiceType) {
        if service_type.subscription_scope() != SubscriptionScope::NetworkWide {
            log::warn!("Attempted to clean up non-NetworkWide service {:?} as NetworkWide", service_type);
            return;
        }

        let mut network_subscriptions = self.network_subscriptions.write().unwrap();
        if let Some(subscription_id) = network_subscriptions.remove(&service_type) {
            log::info!(
                "[NetworkWide] Cleaned up failed {:?} subscription {} from registry",
                service_type,
                subscription_id
            );

            // Also remove from main subscriptions registry
            let mut subscriptions = self.subscriptions.write().unwrap();
            if let Some(mut subscription) = subscriptions.remove(&subscription_id) {
                // Attempt graceful unsubscribe, but don't fail if it doesn't work
                if let Err(e) = subscription.unsubscribe() {
                    log::debug!("Failed to unsubscribe failed NetworkWide service (expected): {}", e);
                }
                log::debug!("[NetworkWide] Removed failed subscription {} from main registry", subscription_id);
            }
        } else {
            log::debug!("[NetworkWide] No active subscription found for {:?} during cleanup", service_type);
        }
    }

    /// Validate subscription registry consistency
    /// 
    /// This method checks for registry corruption and attempts to recover,
    /// ensuring error isolation between service types.
    fn validate_subscription_registry(&self) -> Result<(), SubscriptionError> {
        let network_subscriptions = self.network_subscriptions.read().unwrap();
        let subscriptions = self.subscriptions.read().unwrap();

        let mut corruption_detected = false;
        let mut corruption_messages = Vec::new();

        // Check that all NetworkWide subscriptions in registry exist in main subscriptions
        for (service_type, &subscription_id) in network_subscriptions.iter() {
            if !subscriptions.contains_key(&subscription_id) {
                corruption_detected = true;
                corruption_messages.push(format!(
                    "NetworkWide service {:?} references non-existent subscription {}",
                    service_type, subscription_id
                ));
            } else if let Some(subscription) = subscriptions.get(&subscription_id) {
                // Verify service type matches
                if subscription.service_type() != *service_type {
                    corruption_detected = true;
                    corruption_messages.push(format!(
                        "NetworkWide registry mismatch: expected {:?}, found {:?} for subscription {}",
                        service_type, subscription.service_type(), subscription_id
                    ));
                }
                
                // Verify subscription scope
                if subscription.subscription_scope() != SubscriptionScope::NetworkWide {
                    corruption_detected = true;
                    corruption_messages.push(format!(
                        "Non-NetworkWide subscription {} registered as NetworkWide for service {:?}",
                        subscription_id, service_type
                    ));
                }
            }
        }

        if corruption_detected {
            let message = corruption_messages.join("; ");
            log::error!("[Registry] Subscription registry corruption detected: {}", message);
            return Err(SubscriptionError::RegistryCorruption { message });
        }

        log::debug!("[Registry] Subscription registry validation passed");
        Ok(())
    }





    /// Create a subscription for a specific service on a speaker with retry logic
    fn create_subscription_for_service(
        &self,
        speaker: &Speaker,
        service_type: ServiceType,
        config: SubscriptionConfig,
    ) -> SubscriptionResult<SubscriptionId> {
        let max_attempts = self.config.retry_attempts;
        let base_backoff = self.config.retry_backoff;

        for attempt in 0..max_attempts {
            match self.try_create_subscription_for_service(speaker, service_type, config.clone()) {
                Ok(subscription_id) => {
                    if attempt > 0 {
                        log::info!(
                            "Successfully created {:?} subscription for {} after {} attempts",
                            service_type,
                            speaker.name,
                            attempt + 1
                        );
                    }
                    return Ok(subscription_id);
                }
                Err(e) => {
                    // Don't retry certain error types that will never succeed
                    match &e {
                        SubscriptionError::SatelliteSpeaker => {
                            log::debug!(
                                "Speaker {} is a satellite speaker, not retrying",
                                speaker.name
                            );
                            return Err(e);
                        }
                        SubscriptionError::ServiceNotSupported { .. } => {
                            log::debug!(
                                "Service {:?} not supported by {}, not retrying",
                                service_type,
                                speaker.name
                            );
                            return Err(e);
                        }
                        _ => {
                            // For other errors, continue with retry logic
                        }
                    }

                    if attempt < max_attempts - 1 {
                        let backoff_duration =
                            Self::calculate_backoff_duration(attempt, base_backoff);
                        log::warn!(
                            "Failed to create {:?} subscription for {} (attempt {}/{}): {}. Retrying in {:?}",
                            service_type,
                            speaker.name,
                            attempt + 1,
                            max_attempts,
                            e,
                            backoff_duration
                        );

                        // Wait before retrying
                        thread::sleep(backoff_duration);
                    } else {
                        log::error!(
                            "Failed to create {:?} subscription for {} after {} attempts: {}",
                            service_type,
                            speaker.name,
                            max_attempts,
                            e
                        );
                        return Err(e);
                    }
                }
            }
        }

        // This should never be reached, but just in case
        Err(SubscriptionError::SubscriptionFailed(
            "Maximum retry attempts exceeded".to_string(),
        ))
    }

    /// Try to create a subscription for a specific service on a speaker (single attempt)
    fn try_create_subscription_for_service(
        &self,
        speaker: &Speaker,
        service_type: ServiceType,
        config: SubscriptionConfig,
    ) -> SubscriptionResult<SubscriptionId> {
        // For network-wide services, check if we already have an existing subscription
        if service_type.subscription_scope() == SubscriptionScope::NetworkWide {
            let network_subscriptions = self.network_subscriptions.read().unwrap();
            if let Some(&existing_subscription_id) = network_subscriptions.get(&service_type) {
                // Verify the subscription is still active
                let subscriptions = self.subscriptions.read().unwrap();
                if let Some(subscription) = subscriptions.get(&existing_subscription_id) {
                    if subscription.is_active() {
                        log::debug!(
                            "Reusing existing {:?} network-wide subscription {} for speaker {}",
                            service_type,
                            existing_subscription_id,
                            speaker.name
                        );
                        return Ok(existing_subscription_id);
                    }
                }
                // If we get here, the subscription exists in the registry but is not active
                // We'll clean it up and create a new one below
                drop(subscriptions);
                drop(network_subscriptions);
                self.cleanup_inactive_network_subscription(service_type);
            }
        }

        // Generate subscription ID and callback URL
        let subscription_id = SubscriptionId::new();
        let callback_url = self.get_callback_url(subscription_id);
        
        println!("📡 Creating subscription with callback URL: {}", callback_url);

        // Create the appropriate subscription based on service type
        let mut subscription: Box<dyn ServiceSubscription> = match service_type {
            ServiceType::AVTransport => Box::new(AVTransportSubscription::new(
                speaker.clone(),
                callback_url,
                config,
            )?),
            ServiceType::RenderingControl => Box::new(RenderingControlSubscription::new(
                speaker.clone(),
                callback_url,
                config,
            )?),
            ServiceType::ContentDirectory => {
                // TODO: Implement ContentDirectorySubscription in future tasks
                return Err(SubscriptionError::ServiceNotSupported {
                    service: service_type,
                });
            }
            ServiceType::ZoneGroupTopology => {
                // Simplified ZoneGroupTopology subscription - just use the current speaker
                // No complex network speaker management needed
                use super::zone_group_topology::ZoneGroupTopologySubscription;
                Box::new(ZoneGroupTopologySubscription::new(
                    speaker.clone(),
                    callback_url,
                    config,
                )?)
            }
        };

        // Establish the subscription with the device
        println!("🔗 Attempting to subscribe to {:?} service on speaker {}", service_type, speaker.name);
        let _actual_subscription_id = subscription.subscribe()?;
        println!("✅ Successfully subscribed to {:?} service, got SID from device", service_type);

        // Register with callback server using the original subscription ID (from callback URL)
        if let Some(callback_server) = self.callback_server.read().unwrap().as_ref() {
            let callback_path = format!("/callback/{}", subscription_id);
            println!("📝 Registering subscription {} with callback path: {}", subscription_id, callback_path);
            callback_server.register_subscription(subscription_id, callback_path)?;
            println!("✅ Successfully registered subscription {} with callback server", subscription_id);
        } else {
            println!("❌ No callback server available for subscription registration!");
        }

        // Store the subscription using the original subscription ID
        {
            let mut subscriptions = self.subscriptions.write().unwrap();
            subscriptions.insert(subscription_id, subscription);
        }

        // For network-wide services, register this subscription in the network registry
        if service_type.subscription_scope() == SubscriptionScope::NetworkWide {
            let mut network_subscriptions = self.network_subscriptions.write().unwrap();
            network_subscriptions.insert(service_type, subscription_id);
            log::info!(
                "Registered {:?} network-wide subscription {} for speaker {}",
                service_type,
                subscription_id,
                speaker.name
            );
        }

        Ok(subscription_id)
    }

    /// Calculate exponential backoff duration
    fn calculate_backoff_duration(attempt: u32, base_duration: Duration) -> Duration {
        let multiplier = 2_u64.pow(attempt);
        let backoff_ms = base_duration.as_millis() as u64 * multiplier;

        // Cap at 30 seconds to avoid excessive delays
        let max_backoff_ms = 30_000;
        let capped_backoff_ms = backoff_ms.min(max_backoff_ms);

        Duration::from_millis(capped_backoff_ms)
    }









    /// Remove all subscriptions for a speaker
    fn remove_subscriptions_for_speaker(&self, speaker_id: &SpeakerId) -> SubscriptionResult<()> {
        let mut subscriptions_to_remove = Vec::new();

        // Find subscriptions for this speaker
        {
            let subscriptions = self.subscriptions.read().unwrap();
            for (id, subscription) in subscriptions.iter() {
                if subscription.speaker_id() == speaker_id {
                    subscriptions_to_remove.push(*id);
                }
            }
        }

        // Remove each subscription
        for subscription_id in subscriptions_to_remove {
            self.remove_subscription(subscription_id)?;
        }

        Ok(())
    }

    /// Remove a specific subscription
    fn remove_subscription(&self, subscription_id: SubscriptionId) -> SubscriptionResult<()> {
        let mut subscription = {
            let mut subscriptions = self.subscriptions.write().unwrap();
            subscriptions.remove(&subscription_id)
        };

        if let Some(ref mut sub) = subscription {
            // Unsubscribe from the device
            if let Err(e) = sub.unsubscribe() {
                log::warn!("Failed to unsubscribe from device: {}", e);
            }

            // Unregister from callback server
            if let Some(callback_server) = self.callback_server.read().unwrap().as_ref() {
                if let Err(e) = callback_server.unregister_subscription(subscription_id) {
                    log::warn!("Failed to unregister from callback server: {}", e);
                }
            }

            log::info!("Removed subscription {}", subscription_id);
        }

        Ok(())
    }

    /// Validate subscription registry consistency (public interface)
    /// 
    /// This method can be called externally to check for registry corruption
    /// and ensure proper error isolation between service types.
    pub fn validate_registry(&self) -> SubscriptionResult<()> {
        self.validate_subscription_registry()
    }

    /// Add a speaker to the subscription manager
    ///
    /// This method will create subscriptions for all enabled services for the given speaker.
    /// If the speaker already exists, it will only update the speaker information without
    /// recreating subscriptions, preserving per-speaker subscriptions during group changes.
    ///
    /// # Arguments
    ///
    /// * `speaker` - The speaker to add subscriptions for
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if subscriptions were created successfully, or an error if the operation failed.
    pub fn add_speaker(&self, speaker: &Speaker) -> SubscriptionResult<()> {
        let speaker_id = speaker.get_id();

        // Check if speaker already exists
        let speaker_already_exists = {
            let speakers = self.speakers.read().unwrap();
            speakers.contains_key(&speaker_id)
        };

        if speaker_already_exists {
            log::debug!("Speaker {} already exists, updating speaker info only (keeping existing subscriptions)", speaker.name);
            
            // Update speaker info but don't recreate subscriptions
            {
                let mut speakers = self.speakers.write().unwrap();
                speakers.insert(speaker_id.clone(), speaker.clone());
            }
            
            return Ok(());
        }

        // Add new speaker to storage
        {
            let mut speakers = self.speakers.write().unwrap();
            speakers.insert(speaker_id.clone(), speaker.clone());
        }

        // Create subscriptions for all enabled services (only for new speakers)
        println!("🔧 Creating subscriptions for speaker: {}", speaker.name);
        let subscription_ids = self.create_subscriptions_for_speaker(&speaker)?;
        println!("🎯 Created {} subscriptions for speaker: {}", subscription_ids.len(), speaker.name);

        log::info!(
            "Added new speaker {} with {} subscriptions",
            speaker.name,
            subscription_ids.len()
        );

        Ok(())
    }

    /// Remove a speaker from the subscription manager
    ///
    /// This method will unsubscribe from all services for the given speaker.
    ///
    /// # Arguments
    ///
    /// * `speaker_id` - The ID of the speaker to remove
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if the speaker was removed successfully, or an error if the operation failed.
    pub fn remove_speaker(&self, speaker_id: &SpeakerId) -> SubscriptionResult<()> {
        // Remove speaker from storage
        let speaker_name = {
            let mut speakers = self.speakers.write().unwrap();
            speakers.remove(&speaker_id).map(|s| s.name)
        };

        if speaker_name.is_none() {
            log::debug!("Speaker {:?} not found for removal", speaker_id);
            return Ok(());
        }



        // Check if this speaker was used for any network-wide subscriptions
        self.handle_network_wide_speaker_removal(&speaker_id)?;

        // Remove all per-speaker subscriptions for this speaker
        self.remove_subscriptions_for_speaker(&speaker_id)?;



        log::info!(
            "Removed speaker {} and all its subscriptions",
            speaker_name.unwrap_or_else(|| format!("{:?}", speaker_id))
        );

        Ok(())
    }

    /// Handle the removal of a speaker that might be used for network-wide subscriptions
    /// 
    /// Simplified approach: only clean up network-wide subscriptions, don't try to recreate them.
    /// They will be recreated naturally when the next speaker is added.
    fn handle_network_wide_speaker_removal(&self, removed_speaker_id: &SpeakerId) -> SubscriptionResult<()> {
        // Check if any network-wide subscriptions are using this speaker
        let network_subscriptions_to_check: Vec<(ServiceType, SubscriptionId)> = {
            let network_subscriptions = self.network_subscriptions.read().unwrap();
            network_subscriptions.iter().map(|(&service_type, &sub_id)| (service_type, sub_id)).collect()
        };

        for (service_type, subscription_id) in network_subscriptions_to_check {
            // Check if this subscription is using the removed speaker
            let subscription_uses_removed_speaker = {
                let subscriptions = self.subscriptions.read().unwrap();
                subscriptions.get(&subscription_id)
                    .map(|sub| sub.speaker_id() == removed_speaker_id)
                    .unwrap_or(false)
            };

            if subscription_uses_removed_speaker {
                log::info!(
                    "Network-wide subscription {} for {:?} was using removed speaker {:?}, cleaning up",
                    subscription_id,
                    service_type,
                    removed_speaker_id
                );

                // Remove the old subscription
                self.remove_subscription(subscription_id)?;
                
                // Clean up from network registry
                self.cleanup_inactive_network_subscription(service_type);

                log::info!(
                    "Cleaned up {:?} network-wide subscription. Will be recreated when next speaker is added.",
                    service_type
                );
            }
        }

        Ok(())
    }

    // Removed complex failover and recovery logic to prevent subscription recreation issues

    /// Refresh all subscriptions
    ///
    /// This method checks all active subscriptions and renews any that are approaching expiry.
    pub fn refresh_subscriptions(&self) -> SubscriptionResult<()> {
        Self::check_subscription_renewals(&self.subscriptions, &self.config);
        Ok(())
    }

    /// Shutdown the subscription manager
    ///
    /// This method cleanly shuts down all subscriptions and releases resources.
    pub fn shutdown(&mut self) -> SubscriptionResult<()> {
        log::info!("Shutting down subscription manager");

        // Send shutdown signal to background thread
        if let Some(shutdown_sender) = self.shutdown_sender.take() {
            let _ = shutdown_sender.send(());
        }

        // Wait for management thread to finish
        if let Some(handle) = self.management_thread.take() {
            if let Err(_) = handle.join() {
                log::warn!("Management thread did not shut down cleanly");
            }
        }

        // Unsubscribe from all active subscriptions
        let subscription_ids: Vec<SubscriptionId> = {
            let subscriptions = self.subscriptions.read().unwrap();
            subscriptions.keys().copied().collect()
        };

        for subscription_id in subscription_ids {
            if let Err(e) = self.remove_subscription(subscription_id) {
                log::warn!("Error removing subscription during shutdown: {}", e);
            }
        }

        // Shutdown callback server
        {
            let mut callback_server = self.callback_server.write().unwrap();
            if let Some(mut server) = callback_server.take() {
                if let Err(e) = server.shutdown() {
                    log::warn!("Error shutting down callback server: {}", e);
                }
            }
        }

        // Clear all data
        {
            let mut speakers = self.speakers.write().unwrap();
            speakers.clear();
        }
        {
            let mut network_subscriptions = self.network_subscriptions.write().unwrap();
            network_subscriptions.clear();
        }

        log::info!("Subscription manager shutdown complete");
        Ok(())
    }

    /// Get the number of active subscriptions
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.read().unwrap().len()
    }

    /// Get the number of managed speakers
    pub fn speaker_count(&self) -> usize {
        self.speakers.read().unwrap().len()
    }

    /// Get information about all active subscriptions
    pub fn get_subscription_info(&self) -> Vec<SubscriptionInfo> {
        let subscriptions = self.subscriptions.read().unwrap();
        let speakers = self.speakers.read().unwrap();

        subscriptions
            .iter()
            .map(|(id, subscription)| {
                let speaker_name = speakers
                    .get(&subscription.speaker_id())
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                SubscriptionInfo {
                    id: *id,
                    speaker_id: subscription.speaker_id().clone(),
                    speaker_name,
                    service_type: subscription.service_type(),
                    is_active: subscription.is_active(),
                    last_renewal: subscription.last_renewal(),
                    needs_renewal: subscription.needs_renewal(),
                }
            })
            .collect()
    }

    /// Force renewal of all active subscriptions
    pub fn force_renewal_all(&self) -> SubscriptionResult<()> {
        let mut subscriptions_guard = self.subscriptions.write().unwrap();
        let mut renewal_results = Vec::new();

        for (id, subscription) in subscriptions_guard.iter_mut() {
            if subscription.is_active() {
                match subscription.renew() {
                    Ok(()) => {
                        log::debug!("Successfully force-renewed subscription {}", id);
                    }
                    Err(e) => {
                        log::warn!("Failed to force-renew subscription {}: {}", id, e);
                        renewal_results.push((*id, e));
                        // Mark subscription as inactive
                        let _ = subscription.on_subscription_state_changed(false);
                    }
                }
            }
        }

        if !renewal_results.is_empty() {
            let error_msg = format!("Failed to renew {} subscriptions", renewal_results.len());
            return Err(SubscriptionError::SubscriptionFailed(error_msg));
        }

        Ok(())
    }



    /// Recreate subscriptions for a specific speaker
    ///
    /// This method removes all existing subscriptions for the speaker and creates new ones.
    /// Useful for recovering from persistent subscription failures.
    pub fn recreate_subscriptions_for_speaker(
        &self,
        speaker_id: SpeakerId,
    ) -> SubscriptionResult<()> {
        // Get the speaker info
        let speaker = {
            let speakers = self.speakers.read().unwrap();
            speakers.get(&speaker_id).cloned()
        };

        let speaker = match speaker {
            Some(s) => s,
            None => {
                return Err(SubscriptionError::SubscriptionNotFound {
                    subscription_id: SubscriptionId::new(), // Placeholder
                });
            }
        };

        // Remove existing subscriptions
        self.remove_subscriptions_for_speaker(&speaker_id)?;

        // Create new subscriptions
        let subscription_ids = self.create_subscriptions_for_speaker(&speaker)?;

        log::info!(
            "Recreated {} subscriptions for speaker {}",
            subscription_ids.len(),
            speaker.name
        );

        Ok(())
    }

    /// Get subscription statistics
    pub fn get_statistics(&self) -> SubscriptionStatistics {
        let subscriptions = self.subscriptions.read().unwrap();
        let speakers = self.speakers.read().unwrap();

        let total_subscriptions = subscriptions.len();
        let active_subscriptions = subscriptions.values().filter(|s| s.is_active()).count();
        let subscriptions_needing_renewal =
            subscriptions.values().filter(|s| s.needs_renewal()).count();

        let mut service_counts = HashMap::new();
        for subscription in subscriptions.values() {
            *service_counts
                .entry(subscription.service_type())
                .or_insert(0) += 1;
        }

        SubscriptionStatistics {
            total_speakers: speakers.len(),
            total_subscriptions,
            active_subscriptions,
            subscriptions_needing_renewal,
            service_counts,
        }
    }

    /// Clean up inactive subscriptions
    ///
    /// This method removes subscriptions that are no longer active and cannot be renewed.
    pub fn cleanup_inactive_subscriptions(&self) -> SubscriptionResult<usize> {
        let mut subscriptions_to_remove = Vec::new();

        // Find inactive subscriptions
        {
            let subscriptions = self.subscriptions.read().unwrap();
            for (id, subscription) in subscriptions.iter() {
                if !subscription.is_active() {
                    subscriptions_to_remove.push(*id);
                }
            }
        }

        // Remove inactive subscriptions
        for subscription_id in &subscriptions_to_remove {
            self.remove_subscription(*subscription_id)?;
        }

        log::info!(
            "Cleaned up {} inactive subscriptions",
            subscriptions_to_remove.len()
        );
        Ok(subscriptions_to_remove.len())
    }

    /// Check if a speaker has active subscriptions
    pub fn has_active_subscriptions(&self, speaker_id: &SpeakerId) -> bool {
        let subscriptions = self.subscriptions.read().unwrap();
        subscriptions
            .values()
            .any(|s| s.speaker_id() == speaker_id && s.is_active())
    }

    /// Get the callback server port
    pub fn callback_server_port(&self) -> Option<u16> {
        let callback_server = self.callback_server.read().unwrap();
        callback_server.as_ref().map(|s| s.port())
    }

    /// Attempt to recover failed subscriptions (simplified to prevent subscription recreation)
    ///
    /// This method is now simplified to prevent automatic subscription recreation
    /// that can cause callback registration conflicts.
    pub fn recover_failed_subscriptions(&self) -> SubscriptionResult<RecoveryReport> {
        let recovery_report = RecoveryReport {
            total_attempts: 0,
            successful_recoveries: 0,
            failed_recoveries: Vec::new(),
        };

        log::debug!("Subscription recovery disabled to prevent callback registration conflicts");
        log::debug!("Subscriptions will be created naturally when speakers are added");

        Ok(recovery_report)
    }

    /// Check network connectivity to a speaker
    pub fn check_speaker_connectivity(&self, speaker_id: &SpeakerId) -> bool {
        let speaker = {
            let speakers = self.speakers.read().unwrap();
            speakers.get(&speaker_id).cloned()
        };

        if let Some(speaker) = speaker {
            Self::is_speaker_reachable(&speaker)
        } else {
            false
        }
    }

    /// Check if a speaker is reachable via network
    fn is_speaker_reachable(speaker: &Speaker) -> bool {
        use std::net::{SocketAddr, TcpStream};
        use std::time::Duration;

        let addr = format!("{}:{}", speaker.ip_address, speaker.port);
        if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
            TcpStream::connect_timeout(&socket_addr, Duration::from_secs(3)).is_ok()
        } else {
            false
        }
    }

    /// Perform health check on all subscriptions (simplified - no automatic recovery)
    pub fn health_check_and_recover(&self) -> SubscriptionResult<HealthCheckReport> {
        let mut report = HealthCheckReport {
            total_speakers: 0,
            reachable_speakers: 0,
            total_subscriptions: 0,
            active_subscriptions: 0,
            recovery_attempted: false,
            recovery_report: None,
        };

        // Check speaker connectivity
        let speakers: Vec<Speaker> = {
            let speakers_guard = self.speakers.read().unwrap();
            speakers_guard.values().cloned().collect()
        };

        report.total_speakers = speakers.len();

        for speaker in &speakers {
            if Self::is_speaker_reachable(speaker) {
                report.reachable_speakers += 1;
            }
        }

        // Check subscription health
        let stats = self.get_statistics();
        report.total_subscriptions = stats.total_subscriptions;
        report.active_subscriptions = stats.active_subscriptions;

        // No automatic recovery to prevent subscription recreation issues
        log::debug!("Health check complete. Automatic recovery disabled to prevent callback conflicts.");

        Ok(report)
    }

    /// Enhanced renewal with retry logic
    fn renew_subscription_with_retry(
        subscription: &mut Box<dyn ServiceSubscription>,
        subscription_id: SubscriptionId,
        config: &StreamConfig,
    ) -> bool {
        let max_attempts = config.retry_attempts.max(1);
        let base_backoff = config.retry_backoff;

        for attempt in 0..max_attempts {
            match subscription.renew() {
                Ok(()) => {
                    if attempt > 0 {
                        log::info!(
                            "Successfully renewed subscription {} after {} attempts",
                            subscription_id,
                            attempt + 1
                        );
                    }
                    return true;
                }
                Err(e) => {
                    if attempt < max_attempts - 1 {
                        let backoff_duration =
                            Self::calculate_backoff_duration(attempt, base_backoff);
                        log::warn!(
                            "Failed to renew subscription {} (attempt {}/{}): {}. Retrying in {:?}",
                            subscription_id,
                            attempt + 1,
                            max_attempts,
                            e,
                            backoff_duration
                        );
                        thread::sleep(backoff_duration);
                    } else {
                        log::error!(
                            "Failed to renew subscription {} after {} attempts: {}",
                            subscription_id,
                            max_attempts,
                            e
                        );
                    }
                }
            }
        }

        false
    }
}

/// Information about a subscription for monitoring and debugging
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub id: SubscriptionId,
    pub speaker_id: SpeakerId,
    pub speaker_name: String,
    pub service_type: ServiceType,
    pub is_active: bool,
    pub last_renewal: Option<SystemTime>,
    pub needs_renewal: bool,
}

/// Statistics about the subscription manager state
#[derive(Debug, Clone)]
pub struct SubscriptionStatistics {
    pub total_speakers: usize,
    pub total_subscriptions: usize,
    pub active_subscriptions: usize,
    pub subscriptions_needing_renewal: usize,
    pub service_counts: HashMap<ServiceType, usize>,
}

/// Report from subscription recovery attempts
#[derive(Debug, Clone)]
pub struct RecoveryReport {
    pub total_attempts: usize,
    pub successful_recoveries: usize,
    pub failed_recoveries: Vec<FailedRecovery>,
}

/// Information about a failed recovery attempt
#[derive(Debug, Clone)]
pub struct FailedRecovery {
    pub speaker_id: SpeakerId,
    pub speaker_name: String,
    pub error: String,
}

/// Report from health check operations
#[derive(Debug, Clone)]
pub struct HealthCheckReport {
    pub total_speakers: usize,
    pub reachable_speakers: usize,
    pub total_subscriptions: usize,
    pub active_subscriptions: usize,
    pub recovery_attempted: bool,
    pub recovery_report: Option<RecoveryReport>,
}

/// Information about a representative speaker for network-wide services
#[derive(Debug, Clone)]
pub struct RepresentativeSpeakerInfo {
    pub speaker_id: SpeakerId,
    pub speaker_name: String,
    pub speaker_ip: String,
    pub subscription_id: SubscriptionId,
    pub is_active: bool,
    pub last_renewal: Option<SystemTime>,
}

impl Drop for SubscriptionManager {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SpeakerId;

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
    fn test_subscription_manager_creation() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();

        let manager = SubscriptionManager::new(config, sender);
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert_eq!(manager.subscription_count(), 0);
        assert_eq!(manager.speaker_count(), 0);
    }

    #[test]
    fn test_add_remove_speaker() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        let speaker = create_test_speaker("uuid:RINCON_123456789::1");
        let speaker_id = speaker.get_id();

        // Add speaker - this will fail because we can't actually connect to a real device
        // but we can test that the method doesn't panic
        let _add_result = manager.add_speaker(&speaker);
        // The result may be an error due to network issues, but that's expected in tests

        // Test speaker count (should be 1 even if subscription creation failed)
        assert_eq!(manager.speaker_count(), 1);

        // Test adding the same speaker again (simulating group change)
        // This should not create duplicate subscriptions
        let _add_again_result = manager.add_speaker(&speaker);
        assert_eq!(manager.speaker_count(), 1); // Should still be 1

        // Remove speaker
        assert!(manager.remove_speaker(speaker_id).is_ok());
        assert_eq!(manager.speaker_count(), 0);
    }

    #[test]
    fn test_refresh_and_shutdown() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let mut manager = SubscriptionManager::new(config, sender).unwrap();

        assert!(manager.refresh_subscriptions().is_ok());
        assert!(manager.shutdown().is_ok());
    }

    #[test]
    fn test_subscription_info() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        let info = manager.get_subscription_info();
        assert_eq!(info.len(), 0); // No subscriptions initially
    }

    #[test]
    fn test_callback_url_generation() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        let subscription_id = super::super::types::SubscriptionId::new();
        let callback_url = manager.get_callback_url(subscription_id);

        assert!(callback_url.starts_with("http://127.0.0.1:"));
        assert!(callback_url.contains(&format!("/callback/{}", subscription_id)));
    }

    #[test]
    fn test_subscription_statistics() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        let stats = manager.get_statistics();
        assert_eq!(stats.total_speakers, 0);
        assert_eq!(stats.total_subscriptions, 0);
        assert_eq!(stats.active_subscriptions, 0);
        assert_eq!(stats.subscriptions_needing_renewal, 0);
        assert!(stats.service_counts.is_empty());
    }

    #[test]
    fn test_has_active_subscriptions() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        let speaker = create_test_speaker("uuid:RINCON_123456789::1");
        let speaker_id = speaker.get_id();

        // Initially no active subscriptions
        assert!(!manager.has_active_subscriptions(speaker_id));

        // Add speaker (may fail due to network, but that's ok for this test)
        let _ = manager.add_speaker(&speaker);

        // The speaker should be tracked even if subscription creation failed
        assert_eq!(manager.speaker_count(), 1);
    }

    #[test]
    fn test_callback_server_port() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        let port = manager.callback_server_port();
        assert!(port.is_some());
        let port = port.unwrap();
        assert!(port >= 8080 && port <= 8090); // Default port range
    }

    #[test]
    fn test_cleanup_inactive_subscriptions() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        // Should not fail even with no subscriptions
        let cleaned = manager.cleanup_inactive_subscriptions();
        assert!(cleaned.is_ok());
        assert_eq!(cleaned.unwrap(), 0);
    }

    #[test]
    fn test_force_renewal_all() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        // Should not fail even with no subscriptions
        let result = manager.force_renewal_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_backoff_duration() {
        let base_duration = Duration::from_millis(100);

        // Test exponential backoff
        let backoff_0 = SubscriptionManager::calculate_backoff_duration(0, base_duration);
        assert_eq!(backoff_0, Duration::from_millis(100)); // 100 * 2^0 = 100

        let backoff_1 = SubscriptionManager::calculate_backoff_duration(1, base_duration);
        assert_eq!(backoff_1, Duration::from_millis(200)); // 100 * 2^1 = 200

        let backoff_2 = SubscriptionManager::calculate_backoff_duration(2, base_duration);
        assert_eq!(backoff_2, Duration::from_millis(400)); // 100 * 2^2 = 400

        // Test capping at 30 seconds
        let backoff_large = SubscriptionManager::calculate_backoff_duration(20, base_duration);
        assert_eq!(backoff_large, Duration::from_millis(30_000)); // Capped at 30 seconds
    }

    #[test]
    fn test_recover_failed_subscriptions() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        // Should not fail even with no speakers
        let result = manager.recover_failed_subscriptions();
        assert!(result.is_ok());

        let report = result.unwrap();
        assert_eq!(report.total_attempts, 0);
        assert_eq!(report.successful_recoveries, 0);
        assert!(report.failed_recoveries.is_empty());
    }

    #[test]
    fn test_check_speaker_connectivity() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        let speaker = create_test_speaker("uuid:RINCON_123456789::1");
        let speaker_id = speaker.get_id();

        // Should return false for non-existent speaker
        assert!(!manager.check_speaker_connectivity(speaker_id));

        // Add speaker
        let _ = manager.add_speaker(&speaker);

        // Should return false for unreachable speaker (192.168.1.100 is likely not reachable in test)
        assert!(!manager.check_speaker_connectivity(speaker_id));
    }

    #[test]
    fn test_health_check_and_recover() {
        let config = StreamConfig::default();
        let (sender, _receiver) = mpsc::channel();
        let manager = SubscriptionManager::new(config, sender).unwrap();

        let result = manager.health_check_and_recover();
        assert!(result.is_ok());

        let report = result.unwrap();
        assert_eq!(report.total_speakers, 0);
        assert_eq!(report.reachable_speakers, 0);
        assert_eq!(report.total_subscriptions, 0);
        assert_eq!(report.active_subscriptions, 0);
        assert!(!report.recovery_attempted);
        assert!(report.recovery_report.is_none());
    }
}



#[cfg(test)]
mod network_tests {
    use super::*;
    use crate::model::{Speaker, SpeakerId};
    use std::sync::mpsc;

    fn create_test_speaker(id: &str, ip: &str, name: &str) -> Speaker {
        Speaker {
            id: SpeakerId::new(id),
            name: name.to_string(),
            room_name: name.to_string(),
            ip_address: ip.to_string(),
            port: 1400,
            model_name: "Test Speaker".to_string(),
            satellites: vec![],
        }
    }

    fn create_test_manager() -> SubscriptionManager {
        let config = StreamConfig::minimal();
        let (event_sender, _) = mpsc::channel();
        SubscriptionManager::new(config, event_sender).unwrap()
    }

}
