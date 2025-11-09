use std::time::Duration;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Callback Server Network Test 🔧");
    println!("═══════════════════════════════════════════════════════════════");

    // Test if we can reach our own callback server
    let local_ip = "10.0.4.29"; // From the debug output
    let port = 8080;
    let test_url = format!("http://{}:{}/test", local_ip, port);

    println!("🌐 Testing callback server reachability...");
    println!("   Local IP: {}", local_ip);
    println!("   Port: {}", port);
    println!("   Test URL: {}", test_url);

    // Test 1: Check if port is open locally
    println!("\n📡 Test 1: Checking if port {} is open locally...", port);
    match std::net::TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(_) => {
            println!("✅ Port {} is available for binding", port);
        }
        Err(e) => {
            println!("❌ Port {} is already in use: {}", port, e);
            println!("   This might be from a previous run or another service");
        }
    }

    // Test 2: Try to connect to the port from localhost
    println!("\n📡 Test 2: Testing localhost connection...");
    match std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse()?,
        Duration::from_secs(2)
    ) {
        Ok(_) => {
            println!("✅ Can connect to localhost:{}", port);
        }
        Err(e) => {
            println!("❌ Cannot connect to localhost:{} - {}", port, e);
        }
    }

    // Test 3: Try to connect to the local IP
    println!("\n📡 Test 3: Testing local IP connection...");
    match std::net::TcpStream::connect_timeout(
        &format!("{}:{}", local_ip, port).parse()?,
        Duration::from_secs(2)
    ) {
        Ok(_) => {
            println!("✅ Can connect to {}:{}", local_ip, port);
        }
        Err(e) => {
            println!("❌ Cannot connect to {}:{} - {}", local_ip, port, e);
        }
    }

    // Test 4: Check firewall suggestions
    println!("\n🔍 Firewall and Network Troubleshooting:");
    println!("   1. Check if macOS firewall is blocking incoming connections:");
    println!("      System Preferences → Security & Privacy → Firewall");
    println!("   2. Try temporarily disabling firewall to test");
    println!("   3. Check if any VPN or network security software is running");
    println!("   4. Verify Sonos and your computer are on the same network segment");
    
    println!("\n💡 Manual Test:");
    println!("   Try this command from another device on your network:");
    println!("   curl -v http://{}:{}/test", local_ip, port);
    println!("   (This should connect even if it returns 404)");

    println!("\n🔧 Next Steps:");
    println!("   1. If port is available, the callback server should start");
    println!("   2. If localhost works but local IP doesn't, it's a firewall issue");
    println!("   3. If nothing works, try a different port range");

    Ok(())
}