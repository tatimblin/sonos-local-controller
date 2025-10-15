use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 Simple HTTP Server Test 🌐");
    println!("═══════════════════════════════════════════════════════════════");

    let port = 9080;
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    
    println!("✅ HTTP server started on port {}", port);
    println!("📡 Listening on all interfaces (0.0.0.0:{})", port);
    println!("🔗 Test URL: http://10.0.4.29:{}/test", port);
    println!("\n💡 Test this server by:");
    println!("   1. Opening http://10.0.4.29:{}/test in a browser", port);
    println!("   2. Running: curl -v http://10.0.4.29:{}/test", port);
    println!("   3. From your phone's browser on the same network");
    println!("\n⏳ Waiting for connections... (Press Ctrl+C to stop)");
    println!("═══════════════════════════════════════════════════════════════\n");

    let mut request_count = 0;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                request_count += 1;
                println!("📥 Request #{} from: {:?}", request_count, stream.peer_addr());
                
                // Read the request
                let mut buffer = [0; 1024];
                match stream.read(&mut buffer) {
                    Ok(size) => {
                        let request = String::from_utf8_lossy(&buffer[..size]);
                        println!("   Request details:");
                        for line in request.lines().take(10) {
                            println!("     {}", line);
                        }
                        
                        // Send a simple HTTP response
                        let response = "HTTP/1.1 200 OK\r\n\r\nHello from Rust HTTP server!\r\n";
                        if let Err(e) = stream.write_all(response.as_bytes()) {
                            println!("   ❌ Failed to send response: {}", e);
                        } else {
                            println!("   ✅ Response sent successfully");
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Failed to read request: {}", e);
                    }
                }
                
                println!("   ─────────────────────────────────────────────────");
            }
            Err(e) => {
                println!("❌ Connection failed: {}", e);
            }
        }
    }

    Ok(())
}