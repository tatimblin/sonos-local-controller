use sonos::services::zone_group_topology::parser::ZoneGroupTopologyParser;
use sonos::{
    discover_speakers_with_timeout, streaming::EventStreamBuilder, ServiceType, SonosError,
};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Sonos Parser Usage - Event Stream Demo");

    // First, test the new ZoneGroupTopologyParser with sample XML
    println!("🔍 Testing ZoneGroupTopologyParser with sample XML...");

    let sample_xml = r#"
        <e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0">
            <e:property>
                <ZoneGroupState>&lt;ZoneGroupState&gt;&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_C43875CA135801400" ID="RINCON_C43875CA135801400:2858411400"&gt;&lt;ZoneGroupMember UUID="RINCON_C43875CA135801400" Location="http://192.168.4.65:1400/xml/device_description.xml" ZoneName="Roam 2" Icon="" Configuration="1" SoftwareVersion="85.0-64200" /&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator="RINCON_804AF2AA2FA201400" ID="RINCON_804AF2AA2FA201400:1331296863"&gt;&lt;ZoneGroupMember UUID="RINCON_804AF2AA2FA201400" Location="http://192.168.4.69:1400/xml/device_description.xml" ZoneName="Living Room" Icon="" Configuration="1" SoftwareVersion="85.0-65020" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;&lt;/ZoneGroupState&gt;</ZoneGroupState>
            </e:property>
        </e:propertyset>
    "#;

    match ZoneGroupTopologyParser::from_xml(sample_xml) {
        Ok(parser) => match parser.zone_groups() {
            Some(zone_groups) => {
                println!("✅ Successfully parsed {} zone groups:", zone_groups.len());
                for (i, group) in zone_groups.iter().enumerate() {
                    println!(
                        "  Group {}: Coordinator={}, ID={}, Members={}",
                        i + 1,
                        group.coordinator,
                        group.id,
                        group.members.len()
                    );
                    for (j, member) in group.members.iter().enumerate() {
                        println!(
                            "    Member {}: UUID={}, ZoneName={}, Satellites={}",
                            j + 1,
                            member.uuid,
                            member.zone_name,
                            member.satellites.len()
                        );
                    }
                }
            }
            None => {
                println!("⚠️  Parser returned None for zone_groups()");
            }
        },
        Err(e) => {
            println!("❌ Parser failed: {:?}", e);
        }
    }

    println!("\nDiscovering speakers...");

    // Discover speakers
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(2)) {
        Ok(speakers) if !speakers.is_empty() => speakers,
        Ok(_) | Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on the network.");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    println!("Found {} speakers", speakers.len());

    // Setup event streaming
    match EventStreamBuilder::new(speakers) {
        Ok(builder) => {
            let start_time = Instant::now();

            println!("🚀 Starting event stream for 30 seconds...");
            println!("Will log all XML events and their parsed data\n");

            match builder
                .with_services(&[ServiceType::ZoneGroupTopology])
                .with_event_handler(move |event| {                    
                    println!("📡 StateChange Event received:");
                    println!("Event: {:?}", event);
                    println!();
                })
                .start()
            {
                Ok(_stream) => {
                    println!("✅ Event streaming active - waiting for zone group topology events...");
                    println!("💡 Try grouping/ungrouping speakers in the Sonos app to see events!");
                    println!("⏰ Running for 30 seconds...\n");

                    // Run for 30 seconds
                    while start_time.elapsed() < Duration::from_secs(30) {
                        std::thread::sleep(Duration::from_millis(100));
                    }

                    println!("⏰ 30 seconds elapsed!");
                    println!("🎯 Demo complete!");
                }
                Err(e) => {
                    println!("⚠️  Streaming failed: {:?}", e);
                    return Err(Box::new(e));
                }
            }
        }
        Err(e) => {
            println!("⚠️  Failed to create event stream: {:?}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}
