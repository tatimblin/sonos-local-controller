use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::{mpsc, RwLock};
use warp::Filter;

use crate::streaming::types::{SubscriptionId, RawEvent};
use crate::streaming::subscription::SubscriptionError;

/// Server status information
#[derive(Debug, Clone)]
pub struct ServerStatus {
    pub port: u16,
    pub is_running: bool,
    pub is_healthy: bool,
}

/// HTTP server for receiving UPnP event notifications from Sonos devices
pub struct CallbackServer {
    server_handle: Option<JoinHandle<()>>,
    port: u16,
    event_router: Arc<EventRouter>,
    shutdown_tx: Option<mpsc::UnboundedSender<()>>,
}

impl CallbackServer {
    /// Create a new CallbackServer that will bind to a port in the given range
    pub fn new(
        port_range: (u16, u16),
        event_sender: mpsc::UnboundedSender<RawEvent>,
    ) -> Result<Self, SubscriptionError> {
        let event_router = Arc::new(EventRouter::new(event_sender));
        
        // Try to bind to a port in the range
        let mut port = None;
        for p in port_range.0..=port_range.1 {
            if Self::is_port_available(p) {
                port = Some(p);
                break;
            }
        }
        
        let port = port.ok_or_else(|| {
            SubscriptionError::CallbackServerError(
                format!("No available ports in range {}..{}", port_range.0, port_range.1)
            )
        })?;

        Ok(Self {
            server_handle: None,
            port,
            event_router,
            shutdown_tx: None,
        })
    }

    /// Start the HTTP server
    pub fn start(&mut self) -> Result<(), SubscriptionError> {
        if self.server_handle.is_some() {
            return Err(SubscriptionError::CallbackServerError(
                "Server is already running".to_string()
            ));
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
        let event_router = Arc::clone(&self.event_router);
        let port = self.port;

        let server_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Create a filter to pass the event router to handlers
                let with_router = warp::any().map(move || Arc::clone(&event_router));

                // Create the NOTIFY route for UPnP event notifications
                let notify_route = warp::path::full()
                    .and(warp::header::headers_cloned())
                    .and(warp::body::bytes())
                    .and(with_router.clone())
                    .and_then(handle_notify_request);

                // Create a catch-all route for logging all requests
                let catch_all = warp::path::full()
                    .and(warp::method())
                    .and(warp::header::headers_cloned())
                    .and(warp::body::bytes())
                    .and(with_router)
                    .and_then(handle_any_request);

                // Combine routes - try notify route first, then catch-all
                let routes = notify_route.or(catch_all);

                // Create the server - bind to all interfaces so Sonos devices can reach it
                let (_addr, server) = warp::serve(routes)
                    .bind_with_graceful_shutdown(
                        SocketAddr::from(([0, 0, 0, 0], port)),
                        async move {
                            shutdown_rx.recv().await;
                        }
                    );

                server.await;
            });
        });

        self.server_handle = Some(server_handle);
        self.shutdown_tx = Some(shutdown_tx);

        Ok(())
    }

    /// Stop the HTTP server
    pub fn shutdown(&mut self) -> Result<(), SubscriptionError> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(handle) = self.server_handle.take() {
            handle.join().map_err(|_| {
                SubscriptionError::CallbackServerError(
                    "Failed to join server thread".to_string()
                )
            })?;
        }

        Ok(())
    }

    /// Register a subscription for event routing
    pub fn register_subscription(
        &self,
        subscription_id: SubscriptionId,
        callback_path: String,
    ) -> Result<(), SubscriptionError> {
        self.event_router.register_subscription(subscription_id, callback_path)
    }

    /// Unregister a subscription
    pub fn unregister_subscription(
        &self,
        subscription_id: SubscriptionId,
    ) -> Result<(), SubscriptionError> {
        self.event_router.unregister_subscription(subscription_id)
    }

    /// Get the port the server is bound to
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the base callback URL for this server
    pub fn base_url(&self) -> String {
        // Try to get the local IP address instead of using localhost
        match Self::get_local_ip() {
            Some(ip) => format!("http://{}:{}", ip, self.port),
            None => {
                log::warn!("Could not determine local IP address, using localhost (this may not work with Sonos devices)");
                format!("http://127.0.0.1:{}", self.port)
            }
        }
    }

    /// Get the local IP address of this machine
    fn get_local_ip() -> Option<String> {
        use std::net::{TcpStream, UdpSocket};
        use std::time::Duration;
        
        // First try: Use UDP socket to connect to a local address (doesn't actually send data)
        // This is faster and doesn't require internet connectivity
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
            // Try to connect to the router's typical address to determine our local IP
            for router_ip in &["192.168.1.1:80", "192.168.0.1:80", "10.0.0.1:80"] {
                if let Ok(()) = socket.connect(router_ip) {
                    if let Ok(local_addr) = socket.local_addr() {
                        let ip_str = local_addr.ip().to_string();
                        // Make sure it's not a loopback address
                        if !ip_str.starts_with("127.") {
                            return Some(ip_str);
                        }
                    }
                }
            }
        }
        
        // Second try: Use TCP with a short timeout to external address
        if let Ok(stream) = TcpStream::connect_timeout(
            &"8.8.8.8:80".parse().unwrap(), 
            Duration::from_millis(1000)
        ) {
            if let Ok(local_addr) = stream.local_addr() {
                return Some(local_addr.ip().to_string());
            }
        }
        
        // Fallback: Use localhost (this may not work with Sonos devices)
        None
    }

    /// Check if a port is available for binding
    fn is_port_available(port: u16) -> bool {
        std::net::TcpListener::bind(("0.0.0.0", port)).is_ok()
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }

    /// Perform a health check on the server
    pub fn health_check(&self) -> Result<bool, SubscriptionError> {
        if !self.is_running() {
            return Ok(false);
        }

        // Check if the server thread is still alive
        if let Some(handle) = &self.server_handle {
            if handle.is_finished() {
                return Ok(false);
            }
        }

        // Try to connect to the server port to verify it's accepting connections
        match std::net::TcpStream::connect(("127.0.0.1", self.port)) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Restart the server if it's not healthy
    pub fn restart_if_unhealthy(&mut self) -> Result<bool, SubscriptionError> {
        match self.health_check() {
            Ok(true) => Ok(false), // Server is healthy, no restart needed
            Ok(false) => {
                log::warn!("Server health check failed, attempting restart on port {}", self.port);
                
                // Shutdown the current server
                let _ = self.shutdown();
                
                // Start a new server
                self.start()?;
                
                log::info!("Server successfully restarted on port {}", self.port);
                Ok(true) // Server was restarted
            }
            Err(e) => {
                log::error!("Health check failed with error: {}", e);
                Err(e)
            }
        }
    }

    /// Get server status information
    pub fn status(&self) -> ServerStatus {
        ServerStatus {
            port: self.port,
            is_running: self.is_running(),
            is_healthy: self.health_check().unwrap_or(false),
        }
    }
}

impl Drop for CallbackServer {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Handler function for NOTIFY requests
async fn handle_notify_request(
    path: warp::path::FullPath,
    headers: warp::http::HeaderMap,
    body: bytes::Bytes,
    event_router: Arc<EventRouter>,
) -> Result<impl warp::Reply, warp::Rejection> {
    event_router.handle_notify_request(path, headers, body).await
}

/// Handler function for all other requests (for debugging)
async fn handle_any_request(
    path: warp::path::FullPath,
    method: warp::http::Method,
    headers: warp::http::HeaderMap,
    body: bytes::Bytes,
    _event_router: Arc<EventRouter>,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("\n🌐 === ANY HTTP REQUEST ===");
    println!("📡 Method: {}", method);
    println!("📡 Path: {}", path.as_str());
    println!("📏 Body size: {} bytes", body.len());
    
    println!("📋 Headers:");
    for (name, value) in headers.iter() {
        println!("   {}: {:?}", name, value);
    }
    
    if body.len() > 0 {
        match String::from_utf8(body.to_vec()) {
            Ok(body_str) => {
                println!("📄 Body content:");
                println!("   {}", body_str.chars().take(300).collect::<String>());
                if body_str.len() > 300 {
                    println!("   ... (truncated, total {} chars)", body_str.len());
                }
            }
            Err(_) => {
                println!("📄 Body: [Invalid UTF-8, {} bytes]", body.len());
            }
        }
    } else {
        println!("📄 Body: [Empty]");
    }
    println!("🌐 === END ANY REQUEST ===\n");

    // Return a simple response
    Ok(warp::reply::with_status(
        format!("Received {} request to {}", method, path.as_str()),
        warp::http::StatusCode::OK,
    ))
}

/// Routes UPnP events to the appropriate subscription handlers
pub struct EventRouter {
    /// Maps callback paths to subscription IDs
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionId>>>,
    /// Channel to send parsed events
    event_sender: mpsc::UnboundedSender<RawEvent>,
}

impl EventRouter {
    /// Create a new EventRouter
    pub fn new(event_sender: mpsc::UnboundedSender<RawEvent>) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        }
    }

    /// Register a subscription for a specific callback path
    pub async fn register_subscription_async(
        &self,
        subscription_id: SubscriptionId,
        callback_path: String,
    ) -> Result<(), SubscriptionError> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(callback_path, subscription_id);
        Ok(())
    }

    /// Register a subscription for a specific callback path (blocking version)
    pub fn register_subscription(
        &self,
        subscription_id: SubscriptionId,
        callback_path: String,
    ) -> Result<(), SubscriptionError> {
        println!("📝 Registering subscription: {} -> {}", callback_path, subscription_id);
        
        // Use a blocking approach with std::sync primitives for non-async contexts
        // For now, we'll use a simple approach that works in both contexts
        let subscriptions = Arc::clone(&self.subscriptions);
        let path_clone = callback_path.clone();
        
        // Try to get current runtime handle, if available use it, otherwise spawn a thread
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.block_on(async {
                    let mut subs = subscriptions.write().await;
                    subs.insert(callback_path, subscription_id);
                });
            }
            Err(_) => {
                // No runtime available, use a thread with its own runtime
                let handle = std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let mut subs = subscriptions.write().await;
                        subs.insert(callback_path, subscription_id);
                    });
                });
                handle.join().map_err(|_| {
                    SubscriptionError::CallbackServerError(
                        "Failed to register subscription".to_string()
                    )
                })?;
            }
        }
        
        println!("✅ Successfully registered subscription: {}", path_clone);
        Ok(())
    }

    /// Unregister a subscription (async version)
    pub async fn unregister_subscription_async(
        &self,
        subscription_id: SubscriptionId,
    ) -> Result<(), SubscriptionError> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.retain(|_, id| *id != subscription_id);
        Ok(())
    }

    /// Unregister a subscription (blocking version)
    pub fn unregister_subscription(
        &self,
        subscription_id: SubscriptionId,
    ) -> Result<(), SubscriptionError> {
        let subscriptions = Arc::clone(&self.subscriptions);
        
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.block_on(async {
                    let mut subs = subscriptions.write().await;
                    subs.retain(|_, id| *id != subscription_id);
                });
            }
            Err(_) => {
                let handle = std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let mut subs = subscriptions.write().await;
                        subs.retain(|_, id| *id != subscription_id);
                    });
                });
                handle.join().map_err(|_| {
                    SubscriptionError::CallbackServerError(
                        "Failed to unregister subscription".to_string()
                    )
                })?;
            }
        }
        
        Ok(())
    }

    /// Handle incoming NOTIFY requests from UPnP devices
    pub async fn handle_notify_request(
        &self,
        path: warp::path::FullPath,
        headers: warp::http::HeaderMap,
        body: bytes::Bytes,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // LOG: Incoming HTTP request details
        println!("\n🌐 === INCOMING HTTP REQUEST ===");
        println!("📡 Path: {}", path.as_str());
        println!("📏 Body size: {} bytes", body.len());
        
        println!("📋 Headers:");
        for (name, value) in headers.iter() {
            println!("   {}: {:?}", name, value);
        }
        
        if body.len() > 0 {
            match String::from_utf8(body.to_vec()) {
                Ok(body_str) => {
                    println!("📄 Body content:");
                    println!("   {}", body_str.chars().take(500).collect::<String>());
                    if body_str.len() > 500 {
                        println!("   ... (truncated, total {} chars)", body_str.len());
                    }
                }
                Err(_) => {
                    println!("📄 Body: [Invalid UTF-8, {} bytes]", body.len());
                }
            }
        } else {
            println!("📄 Body: [Empty]");
        }
        println!("🌐 === END HTTP REQUEST ===\n");

        // Extract the callback path
        let callback_path = path.as_str().to_string();

        // Find the subscription ID for this path
        let subscription_id = {
            let subscriptions = self.subscriptions.read().await;
            subscriptions.get(&callback_path).copied()
        };

        let subscription_id = match subscription_id {
            Some(id) => {
                println!("✅ Found subscription ID: {} for path: {}", id, callback_path);
                id
            }
            None => {
                println!("❌ No subscription found for path: {}", callback_path);
                
                // Show all registered paths for debugging
                let subscriptions = self.subscriptions.read().await;
                println!("📋 Currently registered paths:");
                for (path, id) in subscriptions.iter() {
                    println!("   {} -> {}", path, id);
                }
                if subscriptions.is_empty() {
                    println!("   (No subscriptions registered)");
                }
                
                log::warn!("Received event for unknown callback path: {}", callback_path);
                return Ok(warp::reply::with_status(
                    "Unknown subscription",
                    warp::http::StatusCode::NOT_FOUND,
                ));
            }
        };

        // Validate required headers
        if !Self::validate_notify_headers(&headers) {
            log::warn!("Invalid NOTIFY headers for subscription {}", subscription_id);
            return Ok(warp::reply::with_status(
                "Invalid headers",
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }

        // Convert body to string
        let event_xml = match String::from_utf8(body.to_vec()) {
            Ok(xml) => xml,
            Err(_) => {
                log::warn!("Invalid UTF-8 in event body for subscription {}", subscription_id);
                return Ok(warp::reply::with_status(
                    "Invalid body encoding",
                    warp::http::StatusCode::BAD_REQUEST,
                ));
            }
        };

        // Create and send the raw event
        let raw_event = RawEvent::new(subscription_id, event_xml);
        
        println!("📤 Sending raw event to subscription manager...");
        println!("   Subscription ID: {}", subscription_id);
        println!("   Event XML length: {} bytes", raw_event.event_xml.len());
        
        if let Err(_) = self.event_sender.send(raw_event) {
            println!("❌ Failed to send raw event to subscription manager!");
            log::error!("Failed to send event for subscription {}", subscription_id);
            return Ok(warp::reply::with_status(
                "Internal server error",
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }

        println!("✅ Raw event sent to subscription manager successfully");
        log::debug!("Successfully processed event for subscription {}", subscription_id);
        
        // Return success response
        Ok(warp::reply::with_status(
            "OK",
            warp::http::StatusCode::OK,
        ))
    }

    /// Validate that the NOTIFY request has required UPnP headers
    fn validate_notify_headers(headers: &warp::http::HeaderMap) -> bool {
        // Check for required UPnP headers
        headers.contains_key("nt") && 
        headers.contains_key("nts") &&
        headers.get("nt").and_then(|v| v.to_str().ok()) == Some("upnp:event") &&
        headers.get("nts").and_then(|v| v.to_str().ok()) == Some("upnp:propchange")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_event_router_registration() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let router = EventRouter::new(tx);
        
        let sub_id = SubscriptionId::new();
        let path = "/callback/test".to_string();
        
        assert!(router.register_subscription_async(sub_id, path.clone()).await.is_ok());
        
        // Check that the subscription was registered
        let subscriptions = router.subscriptions.read().await;
        assert_eq!(subscriptions.get(&path), Some(&sub_id));
    }

    #[tokio::test]
    async fn test_event_router_unregistration() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let router = EventRouter::new(tx);
        
        let sub_id = SubscriptionId::new();
        let path = "/callback/test".to_string();
        
        router.register_subscription_async(sub_id, path.clone()).await.unwrap();
        assert!(router.unregister_subscription_async(sub_id).await.is_ok());
        
        // Check that the subscription was removed
        let subscriptions = router.subscriptions.read().await;
        assert!(!subscriptions.contains_key(&path));
    }

    #[test]
    fn test_validate_notify_headers() {
        let mut headers = warp::http::HeaderMap::new();
        
        // Missing headers should fail
        assert!(!EventRouter::validate_notify_headers(&headers));
        
        // Add required headers
        headers.insert("nt", "upnp:event".parse().unwrap());
        headers.insert("nts", "upnp:propchange".parse().unwrap());
        
        // Should now pass
        assert!(EventRouter::validate_notify_headers(&headers));
        
        // Wrong values should fail
        headers.insert("nt", "wrong:value".parse().unwrap());
        assert!(!EventRouter::validate_notify_headers(&headers));
    }

    #[test]
    fn test_callback_server_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let server = CallbackServer::new((8080, 8090), tx);
        
        assert!(server.is_ok());
        let server = server.unwrap();
        assert!(!server.is_running());
        assert!(server.port() >= 8080 && server.port() <= 8090);
    }

    #[test]
    fn test_callback_server_base_url() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let server = CallbackServer::new((8080, 8090), tx).unwrap();
        
        let base_url = server.base_url();
        assert!(base_url.starts_with("http://"));
        assert!(base_url.contains(&server.port().to_string()));
    }

    #[test]
    fn test_server_status() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let server = CallbackServer::new((8080, 8090), tx).unwrap();
        
        let status = server.status();
        assert_eq!(status.port, server.port());
        assert!(!status.is_running); // Server not started yet
        assert!(!status.is_healthy); // Server not healthy when not running
    }

    #[test]
    fn test_health_check_not_running() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let server = CallbackServer::new((8080, 8090), tx).unwrap();
        
        // Server not started, should not be healthy
        assert_eq!(server.health_check().unwrap(), false);
    }
}