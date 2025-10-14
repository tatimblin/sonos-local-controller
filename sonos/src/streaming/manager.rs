use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc as tokio_mpsc;

use super::av_transport::AVTransportSubscription;
use super::callback_server::CallbackServer;
use super::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use super::types::{RawEvent, ServiceType, StreamConfig, SubscriptionConfig, SubscriptionId};
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
        let (raw_event_sender, raw_event_receiver) = tokio_mpsc::unbounded_channel();
        let mut callback_server =
            CallbackServer::new(config.callback_port_range, raw_event_sender.clone())
                .map_err(|e| SubscriptionError::CallbackServerError(e.to_string()))?;

        // Start the callback server
        callback_server
            .start()
            .map_err(|e| SubscriptionError::CallbackServerError(e.to_string()))?;

        log::info!("Callback server started on port {}", callback_server.port());

        let speakers = Arc::new(RwLock::new(HashMap::new()));
        let subscriptions = Arc::new(RwLock::new(HashMap::new()));
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
                            Self::process_raw_event(&subscriptions, &event_sender, raw_event);
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
        let subscriptions_guard = match subscriptions.read() {
            Ok(guard) => guard,
            Err(_) => {
                log::error!("Failed to acquire read lock on subscriptions");
                return;
            }
        };

        if let Some(subscription) = subscriptions_guard.get(&raw_event.subscription_id) {
            match subscription.parse_event(&raw_event.event_xml) {
                Ok(state_changes) => {
                    for change in state_changes {
                        if let Err(e) = event_sender.send(change) {
                            log::error!("Failed to send state change: {}", e);
                        }
                    }
                }
                Err(e) => {
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
                        log::error!("Failed to send error state change: {}", e);
                    }
                }
            }
        } else {
            log::warn!(
                "Received event for unknown subscription: {}",
                raw_event.subscription_id
            );
        }
    }

    /// Check subscriptions for renewal needs
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
        let subscription_config = SubscriptionConfig::from_stream_config(&self.config);

        for service_type in &self.config.enabled_services {
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

        Ok(subscription_ids)
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
        // Generate subscription ID and callback URL
        let subscription_id = SubscriptionId::new();
        let callback_url = self.get_callback_url(subscription_id);

        // Create the appropriate subscription based on service type
        let mut subscription: Box<dyn ServiceSubscription> = match service_type {
            ServiceType::AVTransport => Box::new(AVTransportSubscription::new(
                speaker.clone(),
                callback_url,
                config,
            )?),
            ServiceType::RenderingControl => {
                // TODO: Implement RenderingControlSubscription in future tasks
                return Err(SubscriptionError::ServiceNotSupported {
                    service: service_type,
                });
            }
            ServiceType::ContentDirectory => {
                // TODO: Implement ContentDirectorySubscription in future tasks
                return Err(SubscriptionError::ServiceNotSupported {
                    service: service_type,
                });
            }
        };

        // Establish the subscription with the device
        let actual_subscription_id = subscription.subscribe()?;

        // Register with callback server
        if let Some(callback_server) = self.callback_server.read().unwrap().as_ref() {
            let callback_path = format!("/callback/{}", subscription_id);
            callback_server.register_subscription(subscription_id, callback_path)?;
        }

        // Store the subscription
        {
            let mut subscriptions = self.subscriptions.write().unwrap();
            subscriptions.insert(actual_subscription_id, subscription);
        }

        Ok(actual_subscription_id)
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
        // Remove speaker from storage
        let speaker_name = {
            let mut speakers = self.speakers.write().unwrap();
            speakers.remove(&speaker_id).map(|s| s.name)
        };

        if speaker_name.is_none() {
            log::debug!("Speaker {:?} not found for removal", speaker_id);
            return Ok(());
        }

        // Remove all subscriptions for this speaker
        self.remove_subscriptions_for_speaker(speaker_id)?;

        log::info!(
            "Removed speaker {} and all its subscriptions",
            speaker_name.unwrap_or_else(|| format!("{:?}", speaker_id))
        );

        Ok(())
    }

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
