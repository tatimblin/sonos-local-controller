use super::subscription::SubscriptionError;
use crate::model::SpeakerId;
use std::time::Duration;

/// Simplified error type for the public streaming interface
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Failed to initialize streaming: {0}")]
    InitializationFailed(String),

    #[error("Speaker operation failed: {0}")]
    SpeakerOperationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error(
        "Network error: {0}. Check your network connection and ensure speakers are reachable."
    )]
    NetworkError(String),

    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    #[error("Shutdown failed")]
    ShutdownFailed,
}

impl From<SubscriptionError> for StreamError {
    fn from(err: SubscriptionError) -> Self {
        match err {
            SubscriptionError::InvalidConfiguration(msg) => StreamError::ConfigurationError(msg),
            SubscriptionError::SatelliteSpeaker => StreamError::SpeakerOperationFailed(
                "Cannot subscribe to satellite speaker. Try using the coordinator speaker instead."
                    .to_string(),
            ),
            SubscriptionError::NetworkError(msg) => StreamError::NetworkError(msg),
            SubscriptionError::ServiceNotSupported { service } => {
                StreamError::SpeakerOperationFailed(format!(
                    "Service {:?} is not supported by this speaker",
                    service
                ))
            }
            SubscriptionError::SubscriptionFailed(msg) => {
                StreamError::SubscriptionError(format!("Failed to establish subscription: {}", msg))
            }
            SubscriptionError::SubscriptionExpired => StreamError::SubscriptionError(
                "Subscription expired. The system will attempt to reconnect automatically."
                    .to_string(),
            ),
            SubscriptionError::EventParseError(msg) => StreamError::SubscriptionError(format!(
                "Failed to parse event from speaker: {}",
                msg
            )),
            SubscriptionError::CallbackServerError(msg) => {
                StreamError::InitializationFailed(format!(
                    "Failed to start callback server: {}. Try using a different port range.",
                    msg
                ))
            }
            SubscriptionError::HttpError(msg) => StreamError::NetworkError(msg),
            SubscriptionError::Timeout(msg) => StreamError::NetworkError(format!(
                "Operation timed out: {}. Check network connectivity.",
                msg
            )),
            _ => StreamError::InitializationFailed(err.to_string()),
        }
    }
}

/// Lifecycle event handlers for streaming events
#[derive(Default)]
pub struct LifecycleHandlers {
    /// Called when a speaker successfully connects and subscriptions are established
    pub on_speaker_connected: Option<Box<dyn Fn(SpeakerId) + Send + Sync>>,

    /// Called when a speaker disconnects or subscriptions fail
    pub on_speaker_disconnected: Option<Box<dyn Fn(SpeakerId) + Send + Sync>>,

    /// Called when an error occurs during streaming operations
    pub on_error: Option<Box<dyn Fn(StreamError) + Send + Sync>>,

    /// Called when the event stream starts successfully
    pub on_stream_started: Option<Box<dyn Fn() + Send + Sync>>,

    /// Called when the event stream stops
    pub on_stream_stopped: Option<Box<dyn Fn() + Send + Sync>>,
}

impl LifecycleHandlers {
    /// Create a new empty set of lifecycle handlers
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the speaker connected handler
    pub fn with_speaker_connected<F>(mut self, handler: F) -> Self
    where
        F: Fn(SpeakerId) + Send + Sync + 'static,
    {
        self.on_speaker_connected = Some(Box::new(handler));
        self
    }

    /// Set the speaker disconnected handler
    pub fn with_speaker_disconnected<F>(mut self, handler: F) -> Self
    where
        F: Fn(SpeakerId) + Send + Sync + 'static,
    {
        self.on_speaker_disconnected = Some(Box::new(handler));
        self
    }

    /// Set the error handler
    pub fn with_error<F>(mut self, handler: F) -> Self
    where
        F: Fn(StreamError) + Send + Sync + 'static,
    {
        self.on_error = Some(Box::new(handler));
        self
    }

    /// Set the stream started handler
    pub fn with_stream_started<F>(mut self, handler: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_stream_started = Some(Box::new(handler));
        self
    }

    /// Set the stream stopped handler
    pub fn with_stream_stopped<F>(mut self, handler: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_stream_stopped = Some(Box::new(handler));
        self
    }
}

/// Statistics about the current streaming session
#[derive(Debug, Clone)]
pub struct StreamStats {
    /// Number of active subscriptions across all speakers and services
    pub active_subscriptions: usize,

    /// Number of speakers currently being monitored
    pub active_speakers: usize,

    /// Total number of events received since streaming started
    pub total_events_received: u64,

    /// Number of subscription errors that have occurred
    pub subscription_errors: u64,

    /// Number of successful subscription renewals
    pub successful_renewals: u64,
}

impl StreamStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self {
            active_subscriptions: 0,
            active_speakers: 0,
            total_events_received: 0,
            subscription_errors: 0,
            successful_renewals: 0,
        }
    }

    /// Check if streaming is currently active
    pub fn is_active(&self) -> bool {
        self.active_subscriptions > 0
    }

    /// Get the average subscriptions per speaker
    pub fn avg_subscriptions_per_speaker(&self) -> f64 {
        if self.active_speakers == 0 {
            0.0
        } else {
            self.active_subscriptions as f64 / self.active_speakers as f64
        }
    }
}

impl Default for StreamStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Optional configuration overrides for advanced users
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    /// Override the default subscription timeout
    pub subscription_timeout: Option<Duration>,

    /// Override the default retry backoff duration
    pub retry_backoff: Option<Duration>,

    /// Override the default callback server port range (start, end)
    pub callback_port_range: Option<(u16, u16)>,

    /// Override the default buffer size for event processing
    pub buffer_size: Option<usize>,

    /// Override the default maximum retry attempts
    pub max_retry_attempts: Option<u32>,
}

impl ConfigOverrides {
    /// Create new empty configuration overrides
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the subscription timeout override
    pub fn with_subscription_timeout(mut self, timeout: Duration) -> Self {
        self.subscription_timeout = Some(timeout);
        self
    }

    /// Set the retry backoff override
    pub fn with_retry_backoff(mut self, backoff: Duration) -> Self {
        self.retry_backoff = Some(backoff);
        self
    }

    /// Set the callback port range override
    pub fn with_callback_port_range(mut self, start: u16, end: u16) -> Self {
        self.callback_port_range = Some((start, end));
        self
    }

    /// Set the buffer size override
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }

    /// Set the maximum retry attempts override
    pub fn with_max_retry_attempts(mut self, attempts: u32) -> Self {
        self.max_retry_attempts = Some(attempts);
        self
    }

    /// Validate the configuration overrides
    pub fn validate(&self) -> Result<(), StreamError> {
        if let Some(timeout) = self.subscription_timeout {
            if timeout.as_secs() < 60 {
                return Err(StreamError::ConfigurationError(
                    "Subscription timeout must be at least 60 seconds".to_string(),
                ));
            }
            if timeout.as_secs() > 86400 {
                return Err(StreamError::ConfigurationError(
                    "Subscription timeout too long (max 24 hours)".to_string(),
                ));
            }
        }

        if let Some(size) = self.buffer_size {
            if size == 0 {
                return Err(StreamError::ConfigurationError(
                    "Buffer size must be greater than 0".to_string(),
                ));
            }
            if size > 100_000 {
                return Err(StreamError::ConfigurationError(
                    "Buffer size too large (max 100,000)".to_string(),
                ));
            }
        }

        if let Some((start, end)) = self.callback_port_range {
            if start >= end {
                return Err(StreamError::ConfigurationError(
                    "Port range start must be less than end".to_string(),
                ));
            }
            if start < 1024 {
                return Err(StreamError::ConfigurationError(
                    "Port range start must be >= 1024".to_string(),
                ));
            }
        }

        if let Some(attempts) = self.max_retry_attempts {
            if attempts > 10 {
                return Err(StreamError::ConfigurationError(
                    "Too many retry attempts (max 10)".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::ServiceType;

    #[test]
    fn test_stream_error_from_subscription_error() {
        // Test satellite speaker error mapping
        let sub_err = SubscriptionError::SatelliteSpeaker;
        let stream_err = StreamError::from(sub_err);
        match stream_err {
            StreamError::SpeakerOperationFailed(msg) => {
                assert!(msg.contains("satellite speaker"));
                assert!(msg.contains("coordinator"));
            }
            _ => panic!("Expected SpeakerOperationFailed"),
        }

        // Test network error mapping
        let sub_err = SubscriptionError::NetworkError("Connection refused".to_string());
        let stream_err = StreamError::from(sub_err);
        match stream_err {
            StreamError::NetworkError(msg) => {
                assert!(msg.contains("Connection refused"));
            }
            _ => panic!("Expected NetworkError"),
        }

        // Test service not supported error mapping
        let sub_err = SubscriptionError::ServiceNotSupported {
            service: ServiceType::ContentDirectory,
        };
        let stream_err = StreamError::from(sub_err);
        match stream_err {
            StreamError::SpeakerOperationFailed(msg) => {
                assert!(msg.contains("ContentDirectory"));
                assert!(msg.contains("not supported"));
            }
            _ => panic!("Expected SpeakerOperationFailed"),
        }
    }

    #[test]
    fn test_lifecycle_handlers_builder() {
        let handlers = LifecycleHandlers::new()
            .with_speaker_connected(|id| println!("Connected: {:?}", id))
            .with_speaker_disconnected(|id| println!("Disconnected: {:?}", id))
            .with_error(|err| println!("Error: {:?}", err))
            .with_stream_started(|| println!("Stream started"))
            .with_stream_stopped(|| println!("Stream stopped"));

        assert!(handlers.on_speaker_connected.is_some());
        assert!(handlers.on_speaker_disconnected.is_some());
        assert!(handlers.on_error.is_some());
        assert!(handlers.on_stream_started.is_some());
        assert!(handlers.on_stream_stopped.is_some());
    }

    #[test]
    fn test_stream_stats() {
        let mut stats = StreamStats::new();
        assert_eq!(stats.active_subscriptions, 0);
        assert_eq!(stats.active_speakers, 0);
        assert!(!stats.is_active());
        assert_eq!(stats.avg_subscriptions_per_speaker(), 0.0);

        stats.active_subscriptions = 6;
        stats.active_speakers = 2;
        assert!(stats.is_active());
        assert_eq!(stats.avg_subscriptions_per_speaker(), 3.0);
    }

    #[test]
    fn test_config_overrides_builder() {
        let config = ConfigOverrides::new()
            .with_subscription_timeout(Duration::from_secs(3600))
            .with_retry_backoff(Duration::from_secs(2))
            .with_callback_port_range(9000, 9010)
            .with_buffer_size(2000)
            .with_max_retry_attempts(5);

        assert_eq!(config.subscription_timeout, Some(Duration::from_secs(3600)));
        assert_eq!(config.retry_backoff, Some(Duration::from_secs(2)));
        assert_eq!(config.callback_port_range, Some((9000, 9010)));
        assert_eq!(config.buffer_size, Some(2000));
        assert_eq!(config.max_retry_attempts, Some(5));
    }

    #[test]
    fn test_config_overrides_validation() {
        // Valid configuration should pass
        let valid_config = ConfigOverrides::new()
            .with_subscription_timeout(Duration::from_secs(1800))
            .with_buffer_size(1000)
            .with_callback_port_range(8080, 8090)
            .with_max_retry_attempts(3);
        assert!(valid_config.validate().is_ok());

        // Invalid timeout (too short)
        let invalid_config =
            ConfigOverrides::new().with_subscription_timeout(Duration::from_secs(30));
        assert!(invalid_config.validate().is_err());

        // Invalid timeout (too long)
        let invalid_config =
            ConfigOverrides::new().with_subscription_timeout(Duration::from_secs(90000));
        assert!(invalid_config.validate().is_err());

        // Invalid buffer size (zero)
        let invalid_config = ConfigOverrides::new().with_buffer_size(0);
        assert!(invalid_config.validate().is_err());

        // Invalid buffer size (too large)
        let invalid_config = ConfigOverrides::new().with_buffer_size(200_000);
        assert!(invalid_config.validate().is_err());

        // Invalid port range (start >= end)
        let invalid_config = ConfigOverrides::new().with_callback_port_range(8080, 8080);
        assert!(invalid_config.validate().is_err());

        // Invalid port range (start too low)
        let invalid_config = ConfigOverrides::new().with_callback_port_range(1000, 1010);
        assert!(invalid_config.validate().is_err());

        // Invalid retry attempts (too many)
        let invalid_config = ConfigOverrides::new().with_max_retry_attempts(15);
        assert!(invalid_config.validate().is_err());
    }
}
