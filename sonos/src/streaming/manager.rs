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
use crate::models::{Speaker, SpeakerId, StateChange};

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
    /// Mapping of speakers by network subnet (ip_subnet -> Vec<SpeakerId>)
    speaker_networks: Arc<RwLock<HashMap<String, Vec<SpeakerId>>>>,
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
        let speaker_networks = Arc::new(RwLock::new(HashMap::new()));
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
            speaker_networks,
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
    fn process_raw_event(
        subscriptions: &Arc<RwLock<HashMap<SubscriptionId, Box<dyn ServiceSubscription>>>>,
        event_sender: &mpsc::Sender<StateChange>,
        raw_event: RawEvent,
    ) {
        println!("🔄 Processing raw event in subscription manager...");
        println!("   Subscription ID: {}", raw_event.subscription_id);
        println!("   Event XML length: {} bytes", raw_event.event_xml.len());
        
        let subscriptions_guard = match subscriptions.read() {
            Ok(guard) => guard,
            Err(_) => {
                println!("❌ Failed to acquire read lock on subscriptions");
                log::error!("Failed to acquire read lock on subscriptions");
                return;
            }
        };

        println!("📋 Current subscriptions in manager:");
        for (id, subscription) in subscriptions_guard.iter() {
            println!("   {} -> Speaker: {:?}, Service: {:?}, Active: {}", 
                id, subscription.speaker_id(), subscription.service_type(), subscription.is_active());
        }
        if subscriptions_guard.is_empty() {
            println!("   (No subscriptions in manager)");
        }

        if let Some(subscription) = subscriptions_guard.get(&raw_event.subscription_id) {
            println!("✅ Found subscription in manager, parsing event...");
            
            match subscription.parse_event(&raw_event.event_xml) {
                Ok(state_changes) => {
                    println!("✅ Successfully parsed {} state changes", state_changes.len());
                    for (i, change) in state_changes.iter().enumerate() {
                        println!("   Change {}: {:?}", i + 1, change);
                        if let Err(e) = event_sender.send(change.clone()) {
                            println!("❌ Failed to send state change {}: {}", i + 1, e);
                            log::error!("Failed to send state change: {}", e);
                        } else {
                            println!("✅ Sent state change {} successfully", i + 1);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to parse event: {}", e);
                    log::warn!(
                        "Failed to parse event for subscription {}: {}",
                        raw_event.subscription_id,
                        e
                    );

                    // Send error event
                    let error_change = StateChange::SubscriptionError {
                        speaker_id: subscription.speaker_id(),
                        service: subscription.service_type(),
                        error: e.to_string(),
                    };

                    if let Err(e) = event_sender.send(error_change) {
                        println!("❌ Failed to send error state change: {}", e);
                        log::error!("Failed to send error state change: {}", e);
                    } else {
                        println!("✅ Sent error state change successfully");
                    }
                }
            }
        } else {
            println!("❌ No subscription found in manager for ID: {}", raw_event.subscription_id);
            log::warn!(
                "Received event for unknown subscription: {}",
                raw_event.subscription_id
            );
        }
        
        println!("🔄 Finished processing raw event\n");
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
                if Self::renew_subscription_with_retry(subscription, subscription_id, config) {
                    log::debug!("Successfully renewed subscription {}", subscription_id);
                } else {
                    log::warn!(
                        "Failed to renew subscription {} after all retry attempts",
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
                log::warn!("Subscription {} has expired", subscription_id);
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
        let mut subscription_ids = Vec::new();
        let mut satellite_errors = 0;
        let mut total_attempts = 0;
        let subscription_config = SubscriptionConfig::from_stream_config(&self.config);

        for service_type in &self.config.enabled_services {
            total_attempts += 1;
            
            // Check if this is a network-wide service
            if service_type.subscription_scope() == SubscriptionScope::NetworkWide {
                // Handle network-wide services differently
                match self.handle_network_wide_service(speaker, *service_type, subscription_config.clone()) {
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
                        log::warn!(
                            "Failed to create {:?} network-wide subscription for speaker {}: {}",
                            service_type,
                            speaker.name,
                            e
                        );

                        // Send error event
                        let error_change = StateChange::SubscriptionError {
                            speaker_id: speaker.id,
                            service: *service_type,
                            error: e.to_string(),
                        };

                        if let Err(send_err) = self.event_sender.send(error_change) {
                            log::error!("Failed to send subscription error event: {}", send_err);
                        }
                    }
                }
            } else {
                // Handle per-speaker services as before
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
                        log::warn!(
                            "Failed to create {:?} subscription for speaker {}: {}",
                            service_type,
                            speaker.name,
                            e
                        );

                        // Send error event
                        let error_change = StateChange::SubscriptionError {
                            speaker_id: speaker.id,
                            service: *service_type,
                            error: e.to_string(),
                        };

                        if let Err(send_err) = self.event_sender.send(error_change) {
                            log::error!("Failed to send subscription error event: {}", send_err);
                        }
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

    /// Handle network-wide service subscription creation and management
    /// 
    /// Returns Ok(Some(subscription_id)) if a new subscription was created,
    /// Ok(None) if an existing subscription is being reused,
    /// or Err if the operation failed.
    fn handle_network_wide_service(
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
            // Verify the subscription is still active
            let is_active = {
                let subscriptions = self.subscriptions.read().unwrap();
                subscriptions.get(&subscription_id)
                    .map(|sub| sub.is_active())
                    .unwrap_or(false)
            };

            if is_active {
                // Update the network speakers list for the existing subscription
                self.update_network_subscription_speakers(subscription_id, speaker)?;
                
                log::debug!(
                    "Reusing existing {:?} network-wide subscription {} for speaker {} (updated speaker list)",
                    service_type,
                    subscription_id,
                    speaker.name
                );
                return Ok(None);
            } else {
                // If we get here, the subscription exists in the registry but is not active
                // We'll clean it up and create a new one
                self.cleanup_inactive_network_subscription(service_type);
            }
        }

        // Get speakers in the same network to determine if we should create a subscription
        let network_speakers = self.get_speakers_in_same_network(speaker);
        
        // Select a representative speaker for the network-wide subscription
        let representative_speaker_id = self.select_representative_speaker(&network_speakers)
            .unwrap_or(speaker.id); // Fallback to current speaker if no representative found

        // Get the representative speaker details
        let representative_speaker = {
            let speakers = self.speakers.read().unwrap();
            speakers.get(&representative_speaker_id).cloned()
        };

        let representative_speaker = match representative_speaker {
            Some(speaker) => speaker,
            None => {
                // If representative speaker not found, use the current speaker
                speaker.clone()
            }
        };

        // Create the network-wide subscription using the representative speaker
        match self.create_subscription_for_service(&representative_speaker, service_type, config) {
            Ok(subscription_id) => {
                // Register this as a network-wide subscription
                {
                    let mut network_subscriptions = self.network_subscriptions.write().unwrap();
                    network_subscriptions.insert(service_type, subscription_id);
                }
                
                log::info!(
                    "Created new {:?} network-wide subscription {} using representative speaker {}",
                    service_type,
                    subscription_id,
                    representative_speaker.name
                );
                
                Ok(Some(subscription_id))
            }
            Err(e) => {
                log::warn!(
                    "Failed to create {:?} network-wide subscription using representative speaker {}: {}",
                    service_type,
                    representative_speaker.name,
                    e
                );
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

    /// Update the network speakers list for an existing network-wide subscription
    /// 
    /// This method is called when a new speaker is added to a network that already
    /// has a network-wide subscription, ensuring the subscription can distribute
    /// events to all speakers in the network.
    fn update_network_subscription_speakers(
        &self,
        subscription_id: SubscriptionId,
        new_speaker: &Speaker,
    ) -> SubscriptionResult<()> {
        // Get network speakers first to avoid nested lock acquisition
        let network_speakers = self.get_speakers_in_same_network(new_speaker);
        let network_speaker_count = {
            let speakers = self.speakers.read().unwrap();
            network_speakers
                .iter()
                .filter(|&&speaker_id| speakers.contains_key(&speaker_id))
                .count()
        };

        // Now acquire the subscriptions lock
        let subscriptions = self.subscriptions.read().unwrap();
        
        if let Some(subscription) = subscriptions.get(&subscription_id) {
            // Check if this is a ZoneGroupTopology subscription that supports network speaker updates
            if subscription.service_type() == ServiceType::ZoneGroupTopology {
                // Update the subscription's network speakers list
                // Note: This requires the subscription to support dynamic speaker list updates
                // For now, we'll log this operation as the ZoneGroupTopologySubscription
                // will handle event distribution based on the current network state
                log::debug!(
                    "Updated network speakers list for subscription {} (now {} speakers in network)",
                    subscription_id,
                    network_speaker_count
                );
            }
        }

        Ok(())
    }

    /// Check if a network has any remaining speakers and clean up network-wide subscriptions if empty
    /// 
    /// This method is called when speakers are removed to ensure network-wide subscriptions
    /// are properly cleaned up when no speakers remain in a network.
    fn cleanup_empty_network_subscriptions(&self, removed_speaker: &Speaker) -> SubscriptionResult<()> {
        let network_subnet = Self::extract_network_subnet(&removed_speaker.ip_address);
        
        // Check if there are any remaining speakers in this network
        let remaining_speakers_in_network = {
            let speaker_networks = self.speaker_networks.read().unwrap();
            speaker_networks
                .get(&network_subnet)
                .map(|speakers| speakers.len())
                .unwrap_or(0)
        };

        // If no speakers remain in the network, clean up network-wide subscriptions
        if remaining_speakers_in_network == 0 {
            log::info!(
                "No speakers remaining in network {}, cleaning up network-wide subscriptions",
                network_subnet
            );

            // Get all network-wide subscriptions that might need cleanup
            let network_subscriptions_to_check: Vec<(ServiceType, SubscriptionId)> = {
                let network_subscriptions = self.network_subscriptions.read().unwrap();
                network_subscriptions.iter().map(|(&service_type, &sub_id)| (service_type, sub_id)).collect()
            };

            for (service_type, subscription_id) in network_subscriptions_to_check {
                // Check if this subscription was using a speaker from the now-empty network
                let subscription_uses_empty_network = {
                    let subscriptions = self.subscriptions.read().unwrap();
                    if let Some(subscription) = subscriptions.get(&subscription_id) {
                        let subscription_speaker_network = Self::extract_network_subnet(
                            &self.get_speaker_ip_for_subscription(subscription.speaker_id()).unwrap_or_default()
                        );
                        subscription_speaker_network == network_subnet
                    } else {
                        false
                    }
                };

                if subscription_uses_empty_network {
                    log::info!(
                        "Removing {:?} network-wide subscription {} for empty network {}",
                        service_type,
                        subscription_id,
                        network_subnet
                    );

                    // Remove the subscription
                    self.remove_subscription(subscription_id)?;
                    
                    // Clean up from network registry
                    self.cleanup_inactive_network_subscription(service_type);
                }
            }
        }

        Ok(())
    }

    /// Get the IP address for a speaker by its ID
    fn get_speaker_ip_for_subscription(&self, speaker_id: SpeakerId) -> Option<String> {
        let speakers = self.speakers.read().unwrap();
        speakers.get(&speaker_id).map(|speaker| speaker.ip_address.clone())
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
                // For ZoneGroupTopology, we need to create a network-wide subscription
                // Get all speakers in the same network for event distribution
                let network_speakers = self.get_speakers_in_same_network(speaker);
                let network_speaker_details: Vec<Speaker> = {
                    let speakers = self.speakers.read().unwrap();
                    network_speakers
                        .iter()
                        .filter_map(|&speaker_id| speakers.get(&speaker_id).cloned())
                        .collect()
                };

                use super::zone_group_topology::ZoneGroupTopologySubscription;
                Box::new(ZoneGroupTopologySubscription::new(
                    speaker.clone(),
                    network_speaker_details,
                    callback_url,
                    config,
                )?)
            }
        };

        // Establish the subscription with the device
        let _actual_subscription_id = subscription.subscribe()?;

        // Register with callback server using the original subscription ID (from callback URL)
        if let Some(callback_server) = self.callback_server.read().unwrap().as_ref() {
            let callback_path = format!("/callback/{}", subscription_id);
            callback_server.register_subscription(subscription_id, callback_path)?;
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

    /// Extract network subnet from IP address for network-wide service grouping
    /// 
    /// This method extracts the first 3 octets of an IPv4 address to group speakers
    /// by network subnet. For example, "192.168.1.100" becomes "192.168.1".
    fn extract_network_subnet(ip_address: &str) -> String {
        let parts: Vec<&str> = ip_address.split('.').collect();
        if parts.len() >= 3 {
            format!("{}.{}.{}", parts[0], parts[1], parts[2])
        } else {
            // Fallback for invalid IP addresses
            ip_address.to_string()
        }
    }

    /// Update speaker network mapping when a speaker is added or updated
    fn update_speaker_network_mapping(&self, speaker: &Speaker) {
        let network_subnet = Self::extract_network_subnet(&speaker.ip_address);
        
        let mut speaker_networks = self.speaker_networks.write().unwrap();
        
        // Remove speaker from any existing networks first
        for (_, speakers_in_network) in speaker_networks.iter_mut() {
            speakers_in_network.retain(|&id| id != speaker.id);
        }
        
        // Add speaker to the correct network
        speaker_networks
            .entry(network_subnet)
            .or_insert_with(Vec::new)
            .push(speaker.id);
            
        // Clean up empty networks
        speaker_networks.retain(|_, speakers| !speakers.is_empty());
    }

    /// Remove speaker from network mapping
    fn remove_speaker_from_network_mapping(&self, speaker_id: SpeakerId) {
        let mut speaker_networks = self.speaker_networks.write().unwrap();
        
        // Remove speaker from all networks
        for (_, speakers_in_network) in speaker_networks.iter_mut() {
            speakers_in_network.retain(|&id| id != speaker_id);
        }
        
        // Clean up empty networks
        speaker_networks.retain(|_, speakers| !speakers.is_empty());
    }

    /// Get all speakers in the same network as the given speaker
    fn get_speakers_in_same_network(&self, speaker: &Speaker) -> Vec<SpeakerId> {
        let network_subnet = Self::extract_network_subnet(&speaker.ip_address);
        let speaker_networks = self.speaker_networks.read().unwrap();
        
        speaker_networks
            .get(&network_subnet)
            .cloned()
            .unwrap_or_else(Vec::new)
    }

    /// Select a representative speaker for network-wide services
    /// 
    /// Priority order:
    /// 1. Coordinator speakers (speakers that are group coordinators)
    /// 2. Non-satellite speakers (speakers that can accept subscriptions)
    /// 3. Most recently discovered (newest speakers in the network)
    /// 4. Fallback to any available speaker
    fn select_representative_speaker(&self, network_speakers: &[SpeakerId]) -> Option<SpeakerId> {
        if network_speakers.is_empty() {
            return None;
        }

        let speakers = self.speakers.read().unwrap();
        
        // Collect available speakers with their details
        let mut available_speakers: Vec<(SpeakerId, &Speaker)> = network_speakers
            .iter()
            .filter_map(|&speaker_id| {
                speakers.get(&speaker_id).map(|speaker| (speaker_id, speaker))
            })
            .collect();

        if available_speakers.is_empty() {
            return None;
        }

        // Priority 1: Prefer non-satellite speakers that are likely coordinators
        // Combine coordinator and non-satellite logic for better selection
        let preferred_candidates: Vec<_> = available_speakers
            .iter()
            .filter(|(_, speaker)| {
                // Prefer speakers that are not satellites or subs
                let is_not_satellite = !speaker.model_name.to_lowercase().contains("satellite") &&
                                      !speaker.model_name.to_lowercase().contains("sub");
                
                // Prefer speakers with no satellites (likely coordinators) or main speakers
                let is_likely_coordinator = speaker.satellites.is_empty();
                
                is_not_satellite && is_likely_coordinator
            })
            .collect();

        if !preferred_candidates.is_empty() {
            // Among preferred candidates, select the one with the lowest IP (most stable)
            if let Some((speaker_id, _)) = preferred_candidates
                .iter()
                .min_by_key(|(_, speaker)| &speaker.ip_address)
            {
                return Some(*speaker_id);
            }
        }

        // Priority 2: Non-satellite speakers (even if they have satellites)
        let non_satellite_candidates: Vec<_> = available_speakers
            .iter()
            .filter(|(_, speaker)| {
                !speaker.model_name.to_lowercase().contains("satellite") &&
                !speaker.model_name.to_lowercase().contains("sub")
            })
            .collect();

        if !non_satellite_candidates.is_empty() {
            // Among non-satellite candidates, prefer the one with the lowest IP (most stable)
            if let Some((speaker_id, _)) = non_satellite_candidates
                .iter()
                .min_by_key(|(_, speaker)| &speaker.ip_address)
            {
                return Some(*speaker_id);
            }
        }

        // Priority 3: Most recently discovered (newest speakers in the network)
        // Since we don't track discovery time, we'll use IP address as a proxy
        // Lower IP addresses are often assigned first and may be more stable
        available_speakers.sort_by_key(|(_, speaker)| &speaker.ip_address);
        
        // Priority 4: Fallback to any available speaker
        available_speakers.first().map(|(speaker_id, _)| *speaker_id)
    }

    /// Remove all subscriptions for a speaker
    fn remove_subscriptions_for_speaker(&self, speaker_id: SpeakerId) -> SubscriptionResult<()> {
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

    /// Add a speaker to the subscription manager
    ///
    /// This method will create subscriptions for all enabled services for the given speaker.
    ///
    /// # Arguments
    ///
    /// * `speaker` - The speaker to add subscriptions for
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if subscriptions were created successfully, or an error if the operation failed.
    pub fn add_speaker(&self, speaker: Speaker) -> SubscriptionResult<()> {
        let speaker_id = speaker.id;

        // Check if speaker already exists
        {
            let speakers = self.speakers.read().unwrap();
            if speakers.contains_key(&speaker_id) {
                log::debug!("Speaker {} already exists, updating", speaker.name);
            }
        }

        // Add speaker to storage
        {
            let mut speakers = self.speakers.write().unwrap();
            speakers.insert(speaker_id, speaker.clone());
        }

        // Update network mapping for network-wide services
        self.update_speaker_network_mapping(&speaker);

        // Create subscriptions for all enabled services
        let subscription_ids = self.create_subscriptions_for_speaker(&speaker)?;

        log::info!(
            "Added speaker {} with {} subscriptions",
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
    pub fn remove_speaker(&self, speaker_id: SpeakerId) -> SubscriptionResult<()> {
        // Get speaker details before removal for network cleanup
        let removed_speaker = {
            let speakers = self.speakers.read().unwrap();
            speakers.get(&speaker_id).cloned()
        };

        // Remove speaker from storage
        let speaker_name = {
            let mut speakers = self.speakers.write().unwrap();
            speakers.remove(&speaker_id).map(|s| s.name)
        };

        if speaker_name.is_none() {
            log::debug!("Speaker {:?} not found for removal", speaker_id);
            return Ok(());
        }

        // Remove speaker from network mapping
        self.remove_speaker_from_network_mapping(speaker_id);

        // Check if this speaker was used for any network-wide subscriptions
        self.handle_network_wide_speaker_removal(speaker_id)?;

        // Remove all per-speaker subscriptions for this speaker
        self.remove_subscriptions_for_speaker(speaker_id)?;

        // Clean up network-wide subscriptions if this was the last speaker in the network
        if let Some(speaker) = removed_speaker {
            self.cleanup_empty_network_subscriptions(&speaker)?;
        }

        log::info!(
            "Removed speaker {} and all its subscriptions",
            speaker_name.unwrap_or_else(|| format!("{:?}", speaker_id))
        );

        Ok(())
    }

    /// Handle the removal of a speaker that might be used for network-wide subscriptions
    fn handle_network_wide_speaker_removal(&self, removed_speaker_id: SpeakerId) -> SubscriptionResult<()> {
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
                    "Network-wide subscription {} for {:?} was using removed speaker {:?}, handling failover",
                    subscription_id,
                    service_type,
                    removed_speaker_id
                );

                // Remove the old subscription
                self.remove_subscription(subscription_id)?;
                
                // Clean up from network registry
                self.cleanup_inactive_network_subscription(service_type);

                // Try to create a new network-wide subscription with a different representative speaker
                // We'll do this by finding any remaining speaker in the network and triggering subscription creation
                if let Some(replacement_speaker) = self.find_replacement_speaker_for_network_service(service_type) {
                    let subscription_config = SubscriptionConfig::from_stream_config(&self.config);
                    
                    match self.handle_network_wide_service(&replacement_speaker, service_type, subscription_config) {
                        Ok(Some(new_subscription_id)) => {
                            log::info!(
                                "Successfully created replacement {:?} network-wide subscription {} using speaker {}",
                                service_type,
                                new_subscription_id,
                                replacement_speaker.name
                            );
                        }
                        Ok(None) => {
                            log::debug!("Replacement subscription already exists for {:?}", service_type);
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to create replacement {:?} network-wide subscription: {}",
                                service_type,
                                e
                            );
                        }
                    }
                } else {
                    log::info!(
                        "No replacement speaker available for {:?} network-wide subscription",
                        service_type
                    );
                }
            }
        }

        Ok(())
    }

    /// Find a replacement speaker for a network-wide service when the current representative is removed
    fn find_replacement_speaker_for_network_service(&self, service_type: ServiceType) -> Option<Speaker> {
        let speakers = self.speakers.read().unwrap();
        
        if speakers.is_empty() {
            return None;
        }

        // Get all available speakers
        let all_speaker_ids: Vec<SpeakerId> = speakers.keys().copied().collect();
        
        // Use the representative speaker selection algorithm to find the best replacement
        if let Some(replacement_id) = self.select_representative_speaker(&all_speaker_ids) {
            speakers.get(&replacement_id).cloned()
        } else {
            None
        }
    }

    /// Detect if a representative speaker for a network-wide service is unavailable
    /// 
    /// This method checks if the speaker used for a network-wide subscription is still
    /// reachable and active. If not, it triggers automatic failover.
    fn detect_representative_speaker_unavailability(&self) -> SubscriptionResult<()> {
        let network_subscriptions_to_check: Vec<(ServiceType, SubscriptionId)> = {
            let network_subscriptions = self.network_subscriptions.read().unwrap();
            network_subscriptions.iter().map(|(&service_type, &sub_id)| (service_type, sub_id)).collect()
        };

        for (service_type, subscription_id) in network_subscriptions_to_check {
            // Get the speaker used by this network-wide subscription
            let representative_speaker_id = {
                let subscriptions = self.subscriptions.read().unwrap();
                subscriptions.get(&subscription_id)
                    .map(|sub| sub.speaker_id())
            };

            if let Some(speaker_id) = representative_speaker_id {
                // Check if the representative speaker is still available and reachable
                let is_speaker_available = {
                    let speakers = self.speakers.read().unwrap();
                    speakers.contains_key(&speaker_id)
                };

                let is_subscription_active = {
                    let subscriptions = self.subscriptions.read().unwrap();
                    subscriptions.get(&subscription_id)
                        .map(|sub| sub.is_active())
                        .unwrap_or(false)
                };

                // If speaker is unavailable or subscription is inactive, trigger failover
                if !is_speaker_available || !is_subscription_active {
                    log::warn!(
                        "Representative speaker {:?} for {:?} service is unavailable or inactive, triggering failover",
                        speaker_id,
                        service_type
                    );

                    self.trigger_representative_speaker_failover(service_type, subscription_id)?;
                }
            }
        }

        Ok(())
    }

    /// Trigger automatic failover to a new representative speaker
    /// 
    /// This method is called when the current representative speaker becomes unavailable.
    /// It selects a new representative and recreates the network-wide subscription.
    fn trigger_representative_speaker_failover(
        &self, 
        service_type: ServiceType, 
        old_subscription_id: SubscriptionId
    ) -> SubscriptionResult<()> {
        log::info!(
            "Triggering failover for {:?} network-wide subscription {}",
            service_type,
            old_subscription_id
        );

        // Remove the old subscription
        self.remove_subscription(old_subscription_id)?;
        
        // Clean up from network registry
        self.cleanup_inactive_network_subscription(service_type);

        // Find a replacement speaker
        if let Some(replacement_speaker) = self.find_replacement_speaker_for_network_service(service_type) {
            let subscription_config = SubscriptionConfig::from_stream_config(&self.config);
            
            match self.handle_network_wide_service(&replacement_speaker, service_type, subscription_config) {
                Ok(Some(new_subscription_id)) => {
                    log::info!(
                        "Successfully failed over {:?} network-wide subscription to new representative speaker {} (subscription {})",
                        service_type,
                        replacement_speaker.name,
                        new_subscription_id
                    );

                    // Send a state change event to notify about the failover
                    let failover_event = StateChange::SubscriptionError {
                        speaker_id: replacement_speaker.id,
                        service: service_type,
                        error: format!("Representative speaker failover completed successfully"),
                    };

                    if let Err(e) = self.event_sender.send(failover_event) {
                        log::warn!("Failed to send failover notification event: {}", e);
                    }

                    Ok(())
                }
                Ok(None) => {
                    log::debug!("Replacement subscription already exists for {:?}", service_type);
                    Ok(())
                }
                Err(e) => {
                    log::error!(
                        "Failed to create replacement {:?} network-wide subscription during failover: {}",
                        service_type,
                        e
                    );

                    // Send error event
                    let error_event = StateChange::SubscriptionError {
                        speaker_id: replacement_speaker.id,
                        service: service_type,
                        error: format!("Failover failed: {}", e),
                    };

                    if let Err(send_err) = self.event_sender.send(error_event) {
                        log::error!("Failed to send failover error event: {}", send_err);
                    }

                    Err(e)
                }
            }
        } else {
            let error_msg = format!("No replacement speaker available for {:?} network-wide subscription", service_type);
            log::error!("{}", error_msg);

            Err(SubscriptionError::SubscriptionFailed(error_msg))
        }
    }

    /// Refresh all subscriptions
    ///
    /// This method checks all active subscriptions and renews any that are approaching expiry.
    pub fn refresh_subscriptions(&self) -> SubscriptionResult<()> {
        Self::check_subscription_renewals(&self.subscriptions, &self.config);
        Ok(())
    }

    /// Check representative speaker availability and trigger failover if needed
    ///
    /// This method should be called periodically to ensure network-wide subscriptions
    /// are using available representative speakers. If a representative speaker becomes
    /// unavailable, this method will automatically trigger failover to a new speaker.
    pub fn check_representative_speaker_availability(&self) -> SubscriptionResult<()> {
        self.detect_representative_speaker_unavailability()
    }

    /// Get information about current representative speakers for network-wide services
    ///
    /// Returns a mapping of service types to their representative speaker information.
    /// This is useful for monitoring and debugging network-wide subscription health.
    pub fn get_representative_speakers(&self) -> HashMap<ServiceType, RepresentativeSpeakerInfo> {
        let mut result = HashMap::new();
        
        let network_subscriptions = self.network_subscriptions.read().unwrap();
        let subscriptions = self.subscriptions.read().unwrap();
        let speakers = self.speakers.read().unwrap();

        for (&service_type, &subscription_id) in network_subscriptions.iter() {
            if let Some(subscription) = subscriptions.get(&subscription_id) {
                let speaker_id = subscription.speaker_id();
                if let Some(speaker) = speakers.get(&speaker_id) {
                    result.insert(service_type, RepresentativeSpeakerInfo {
                        speaker_id,
                        speaker_name: speaker.name.clone(),
                        speaker_ip: speaker.ip_address.clone(),
                        subscription_id,
                        is_active: subscription.is_active(),
                        last_renewal: subscription.last_renewal(),
                    });
                }
            }
        }

        result
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
        {
            let mut speaker_networks = self.speaker_networks.write().unwrap();
            speaker_networks.clear();
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
                    speaker_id: subscription.speaker_id(),
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
        self.remove_subscriptions_for_speaker(speaker_id)?;

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
    pub fn has_active_subscriptions(&self, speaker_id: SpeakerId) -> bool {
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

    /// Attempt to recover failed subscriptions
    ///
    /// This method checks for inactive subscriptions and attempts to recreate them.
    /// It's useful for recovering from network outages or device restarts.
    pub fn recover_failed_subscriptions(&self) -> SubscriptionResult<RecoveryReport> {
        let mut recovery_report = RecoveryReport {
            total_attempts: 0,
            successful_recoveries: 0,
            failed_recoveries: Vec::new(),
        };

        // Get all speakers that should have subscriptions but don't have active ones
        let speakers_needing_recovery: Vec<Speaker> = {
            let speakers = self.speakers.read().unwrap();
            let subscriptions = self.subscriptions.read().unwrap();

            speakers
                .values()
                .filter(|speaker| {
                    // Check if this speaker has any active subscriptions
                    !subscriptions
                        .values()
                        .any(|sub| sub.speaker_id() == speaker.id && sub.is_active())
                })
                .cloned()
                .collect()
        };

        for speaker in speakers_needing_recovery {
            recovery_report.total_attempts += 1;

            match self.create_subscriptions_for_speaker(&speaker) {
                Ok(subscription_ids) => {
                    recovery_report.successful_recoveries += 1;
                    log::info!(
                        "Successfully recovered {} subscriptions for speaker {}",
                        subscription_ids.len(),
                        speaker.name
                    );
                }
                Err(e) => {
                    recovery_report.failed_recoveries.push(FailedRecovery {
                        speaker_id: speaker.id,
                        speaker_name: speaker.name.clone(),
                        error: e.to_string(),
                    });
                    log::warn!(
                        "Failed to recover subscriptions for speaker {}: {}",
                        speaker.name,
                        e
                    );
                }
            }
        }

        log::info!(
            "Recovery complete: {}/{} speakers recovered",
            recovery_report.successful_recoveries,
            recovery_report.total_attempts
        );

        Ok(recovery_report)
    }

    /// Check network connectivity to a speaker
    pub fn check_speaker_connectivity(&self, speaker_id: SpeakerId) -> bool {
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

    /// Perform health check on all subscriptions and attempt recovery
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

        // Attempt recovery if we have reachable speakers but missing subscriptions
        if report.reachable_speakers > 0
            && report.active_subscriptions
                < report.reachable_speakers * self.config.enabled_services.len()
        {
            log::info!("Attempting subscription recovery due to health check findings");
            report.recovery_attempted = true;

            match self.recover_failed_subscriptions() {
                Ok(recovery_report) => {
                    report.recovery_report = Some(recovery_report);
                }
                Err(e) => {
                    log::error!("Recovery attempt failed: {}", e);
                }
            }
        }

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
    use crate::models::SpeakerId;

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
        let speaker_id = speaker.id;

        // Add speaker - this will fail because we can't actually connect to a real device
        // but we can test that the method doesn't panic
        let _add_result = manager.add_speaker(speaker);
        // The result may be an error due to network issues, but that's expected in tests

        // Test speaker count (should be 1 even if subscription creation failed)
        assert_eq!(manager.speaker_count(), 1);

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
        let speaker_id = speaker.id;

        // Initially no active subscriptions
        assert!(!manager.has_active_subscriptions(speaker_id));

        // Add speaker (may fail due to network, but that's ok for this test)
        let _ = manager.add_speaker(speaker);

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
        let speaker_id = speaker.id;

        // Should return false for non-existent speaker
        assert!(!manager.check_speaker_connectivity(speaker_id));

        // Add speaker
        let _ = manager.add_speaker(speaker);

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
    use crate::models::{Speaker, SpeakerId};
    use std::sync::mpsc;

    fn create_test_speaker(id: &str, ip: &str, name: &str) -> Speaker {
        Speaker {
            id: SpeakerId::from_udn(id),
            udn: id.to_string(),
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

    #[test]
    fn test_extract_network_subnet() {
        assert_eq!(
            SubscriptionManager::extract_network_subnet("192.168.1.100"),
            "192.168.1"
        );
        assert_eq!(
            SubscriptionManager::extract_network_subnet("10.0.0.50"),
            "10.0.0"
        );
        assert_eq!(
            SubscriptionManager::extract_network_subnet("172.16.5.200"),
            "172.16.5"
        );
        
        // Test edge cases
        assert_eq!(
            SubscriptionManager::extract_network_subnet("192.168"),
            "192.168"
        );
        assert_eq!(
            SubscriptionManager::extract_network_subnet("invalid"),
            "invalid"
        );
    }

    #[test]
    fn test_update_speaker_network_mapping() {
        let manager = create_test_manager();
        
        let speaker1 = create_test_speaker("uuid:RINCON_123456789::1", "192.168.1.100", "Living Room");
        let speaker2 = create_test_speaker("uuid:RINCON_987654321::1", "192.168.1.101", "Kitchen");
        let speaker3 = create_test_speaker("uuid:RINCON_111222333::1", "10.0.0.50", "Bedroom");

        // Add speakers to different networks
        manager.update_speaker_network_mapping(&speaker1);
        manager.update_speaker_network_mapping(&speaker2);
        manager.update_speaker_network_mapping(&speaker3);

        let speaker_networks = manager.speaker_networks.read().unwrap();
        
        // Check that speakers are grouped by network
        let network_192_168_1 = speaker_networks.get("192.168.1").unwrap();
        assert_eq!(network_192_168_1.len(), 2);
        assert!(network_192_168_1.contains(&speaker1.id));
        assert!(network_192_168_1.contains(&speaker2.id));

        let network_10_0_0 = speaker_networks.get("10.0.0").unwrap();
        assert_eq!(network_10_0_0.len(), 1);
        assert!(network_10_0_0.contains(&speaker3.id));
    }

    #[test]
    fn test_remove_speaker_from_network_mapping() {
        let manager = create_test_manager();
        
        let speaker1 = create_test_speaker("uuid:RINCON_123456789::1", "192.168.1.100", "Living Room");
        let speaker2 = create_test_speaker("uuid:RINCON_987654321::1", "192.168.1.101", "Kitchen");

        // Add speakers
        manager.update_speaker_network_mapping(&speaker1);
        manager.update_speaker_network_mapping(&speaker2);

        // Verify both speakers are in the network
        {
            let speaker_networks = manager.speaker_networks.read().unwrap();
            let network = speaker_networks.get("192.168.1").unwrap();
            assert_eq!(network.len(), 2);
        }

        // Remove one speaker
        manager.remove_speaker_from_network_mapping(speaker1.id);

        // Verify only one speaker remains
        {
            let speaker_networks = manager.speaker_networks.read().unwrap();
            let network = speaker_networks.get("192.168.1").unwrap();
            assert_eq!(network.len(), 1);
            assert!(network.contains(&speaker2.id));
            assert!(!network.contains(&speaker1.id));
        }

        // Remove the last speaker
        manager.remove_speaker_from_network_mapping(speaker2.id);

        // Verify the network is cleaned up
        {
            let speaker_networks = manager.speaker_networks.read().unwrap();
            assert!(!speaker_networks.contains_key("192.168.1"));
        }
    }

    #[test]
    fn test_get_speakers_in_same_network() {
        let manager = create_test_manager();
        
        let speaker1 = create_test_speaker("uuid:RINCON_123456789::1", "192.168.1.100", "Living Room");
        let speaker2 = create_test_speaker("uuid:RINCON_987654321::1", "192.168.1.101", "Kitchen");
        let speaker3 = create_test_speaker("uuid:RINCON_111222333::1", "10.0.0.50", "Bedroom");

        // Add speakers to network mapping
        manager.update_speaker_network_mapping(&speaker1);
        manager.update_speaker_network_mapping(&speaker2);
        manager.update_speaker_network_mapping(&speaker3);

        // Test getting speakers in same network
        let same_network_speakers = manager.get_speakers_in_same_network(&speaker1);
        assert_eq!(same_network_speakers.len(), 2);
        assert!(same_network_speakers.contains(&speaker1.id));
        assert!(same_network_speakers.contains(&speaker2.id));
        assert!(!same_network_speakers.contains(&speaker3.id));

        let different_network_speakers = manager.get_speakers_in_same_network(&speaker3);
        assert_eq!(different_network_speakers.len(), 1);
        assert!(different_network_speakers.contains(&speaker3.id));
    }

    #[test]
    fn test_select_representative_speaker() {
        let manager = create_test_manager();
        
        let speaker1 = create_test_speaker("uuid:RINCON_123456789::1", "192.168.1.100", "Living Room");
        let speaker2 = create_test_speaker("uuid:RINCON_987654321::1", "192.168.1.101", "Kitchen");

        // Add speakers to storage
        {
            let mut speakers = manager.speakers.write().unwrap();
            speakers.insert(speaker1.id, speaker1.clone());
            speakers.insert(speaker2.id, speaker2.clone());
        }

        let network_speakers = vec![speaker1.id, speaker2.id];
        let representative = manager.select_representative_speaker(&network_speakers);
        
        // Should select one of the speakers
        assert!(representative.is_some());
        let selected_id = representative.unwrap();
        assert!(selected_id == speaker1.id || selected_id == speaker2.id);

        // Test with empty list
        let empty_speakers = vec![];
        let no_representative = manager.select_representative_speaker(&empty_speakers);
        assert!(no_representative.is_none());
    }

    #[test]
    fn test_select_representative_speaker_priority() {
        let manager = create_test_manager();
        
        // Test satellite vs non-satellite preference
        let mut satellite_speaker = create_test_speaker("uuid:RINCON_987654321::1", "192.168.1.101", "Satellite");
        satellite_speaker.model_name = "Sonos Satellite".to_string(); // Mark as satellite
        
        let regular_speaker = create_test_speaker("uuid:RINCON_444555666::1", "192.168.1.102", "Regular");

        {
            let mut speakers = manager.speakers.write().unwrap();
            speakers.insert(satellite_speaker.id, satellite_speaker.clone());
            speakers.insert(regular_speaker.id, regular_speaker.clone());
        }

        // Test with satellite and regular speakers - should prefer regular over satellite
        let network_speakers = vec![satellite_speaker.id, regular_speaker.id];
        let representative = manager.select_representative_speaker(&network_speakers);
        
        assert!(representative.is_some());
        let selected_id = representative.unwrap();
        // Should prefer regular speaker over satellite
        assert_eq!(selected_id, regular_speaker.id);

        // Test that algorithm returns a valid speaker from the list
        let speaker1 = create_test_speaker("uuid:RINCON_111111111::1", "192.168.1.100", "Speaker1");
        let speaker2 = create_test_speaker("uuid:RINCON_222222222::1", "192.168.1.101", "Speaker2");
        
        {
            let mut speakers = manager.speakers.write().unwrap();
            speakers.clear(); // Clear previous speakers
            speakers.insert(speaker1.id, speaker1.clone());
            speakers.insert(speaker2.id, speaker2.clone());
        }

        let network_speakers = vec![speaker1.id, speaker2.id];
        let representative = manager.select_representative_speaker(&network_speakers);
        
        assert!(representative.is_some());
        let selected_id = representative.unwrap();
        // Should select one of the available speakers
        assert!(selected_id == speaker1.id || selected_id == speaker2.id);
    }

    #[test]
    fn test_speaker_network_update_on_ip_change() {
        let manager = create_test_manager();
        
        let mut speaker = create_test_speaker("uuid:RINCON_123456789::1", "192.168.1.100", "Living Room");

        // Add speaker to first network
        manager.update_speaker_network_mapping(&speaker);
        
        {
            let speaker_networks = manager.speaker_networks.read().unwrap();
            assert!(speaker_networks.get("192.168.1").unwrap().contains(&speaker.id));
            assert!(!speaker_networks.contains_key("10.0.0"));
        }

        // Update speaker IP to different network
        speaker.ip_address = "10.0.0.50".to_string();
        manager.update_speaker_network_mapping(&speaker);

        {
            let speaker_networks = manager.speaker_networks.read().unwrap();
            // Should be removed from old network and added to new network
            assert!(!speaker_networks.contains_key("192.168.1")); // Old network cleaned up
            assert!(speaker_networks.get("10.0.0").unwrap().contains(&speaker.id));
        }
    }
}
