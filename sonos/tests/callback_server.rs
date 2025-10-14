use sonos::streaming::{CallbackServer, SubscriptionId};
use tokio::sync::mpsc;

#[test]
fn test_callback_server_integration() {
    // Test that we can create a callback server and get basic info
    let (tx, _rx) = mpsc::unbounded_channel();
    let server = CallbackServer::new((8080, 8090), tx).unwrap();

    // Verify basic properties
    assert!(server.port() >= 8080 && server.port() <= 8090);
    assert!(!server.is_running());

    let base_url = server.base_url();
    assert!(base_url.starts_with("http://127.0.0.1:"));

    // Test subscription registration
    let sub_id = SubscriptionId::new();
    let callback_path = format!("/callback/{}", sub_id.as_string());

    assert!(server.register_subscription(sub_id, callback_path).is_ok());
    assert!(server.unregister_subscription(sub_id).is_ok());
}

#[test]
fn test_server_status() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let server = CallbackServer::new((8080, 8090), tx).unwrap();

    let status = server.status();
    assert_eq!(status.port, server.port());
    assert!(!status.is_running);
    assert!(!status.is_healthy);
}
