use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Manual NOTIFY Request Test 🧪");
    println!("═══════════════════════════════════════════════════════════════");

    let callback_url = "http://10.0.4.29:8080/callback/test-subscription-id";

    println!("📡 Sending manual NOTIFY request to: {}", callback_url);

    // Create a sample UPnP event XML (similar to what Sonos would send)
    let event_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0">
    <e:property>
        <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/AVT/"&gt;
            &lt;InstanceID val="0"&gt;
                &lt;TransportState val="PLAYING"/&gt;
                &lt;CurrentPlayMode val="NORMAL"/&gt;
            &lt;/InstanceID&gt;
        &lt;/Event&gt;</LastChange>
    </e:property>
</e:propertyset>"#;

    // Create HTTP client
    let client = reqwest::blocking::Client::new();

    println!("📤 Sending NOTIFY request with UPnP headers...");

    let response = client
        .request(reqwest::Method::from_bytes(b"NOTIFY")?, callback_url)
        .header("HOST", "10.0.4.29:8080")
        .header("CONTENT-TYPE", "text/xml; charset=\"utf-8\"")
        .header("NT", "upnp:event")
        .header("NTS", "upnp:propchange")
        .header("SID", "uuid:test-subscription-id")
        .header("SEQ", "0")
        .body(event_xml)
        .timeout(Duration::from_secs(5))
        .send();

    match response {
        Ok(resp) => {
            println!("✅ Response received:");
            let status = resp.status();
            println!("   Status: {}", status);
            println!("   Headers: {:?}", resp.headers());

            match resp.text() {
                Ok(body) => {
                    println!("   Body: {}", body);
                }
                Err(e) => {
                    println!("   ❌ Failed to read response body: {}", e);
                }
            }

            if status.is_success() {
                println!("🎉 Manual NOTIFY request successful!");
                println!("   This means the callback server is reachable and working");
            } else {
                println!("⚠️  NOTIFY request failed with status: {}", status);
            }
        }
        Err(e) => {
            println!("❌ Failed to send NOTIFY request: {}", e);
            println!("   This suggests the callback server is not reachable");
            println!("   Possible causes:");
            println!("   1. Callback server is not running");
            println!("   2. Port 8080 is blocked by firewall");
            println!("   3. Network connectivity issues");
        }
    }

    Ok(())
}
