use sonos::xml::parser::XmlParser;

fn main() {
    // Sample XML from the actual event logs
    let sample_xml = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><LastChange>&lt;Event xmlns=&quot;urn:schemas-upnp-org:metadata-1-0/AVT/&quot; xmlns:r=&quot;urn:schemas-rinconnetworks-com:metadata-1-0/&quot;&gt;&lt;InstanceID val=&quot;0&quot;&gt;&lt;TransportState val=&quot;PLAYING&quot;/&gt;&lt;CurrentPlayMode val=&quot;NORMAL&quot;/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></e:property></e:propertyset>"#;

    println!("🔍 Testing XML parsing...");
    println!("Original XML: {}", sample_xml);
    
    match XmlParser::parse_av_transport_serde(sample_xml) {
        Ok(data) => {
            println!("✅ Parsing successful!");
            println!("Transport State: {:?}", data.transport_state);
            println!("Track Duration: {:?}", data.current_track_duration);
            println!("Track URI: {:?}", data.current_track_uri);
            println!("Track Metadata: {:?}", data.current_track_metadata);
        }
        Err(e) => {
            println!("❌ Parsing failed: {}", e);
            
            // Let's try to debug step by step
            println!("\n🔍 Debugging step by step...");
            
            // First, let's parse the property
            match XmlParser::parse_property_serde(sample_xml) {
                Ok(property) => {
                    println!("✅ Property parsing successful!");
                    println!("Direct transport_state: {:?}", property.transport_state);
                    println!("LastChange: {:?}", property.last_change);
                    
                    if let Some(last_change) = property.last_change {
                        println!("\n🔍 Decoding LastChange...");
                        let decoded = XmlParser::decode_entities(&last_change);
                        println!("Decoded LastChange: {}", decoded);
                        
                        // Try to parse the LastChange event
                        match serde_xml_rs::from_str::<sonos::xml::types::XmlLastChangeEvent>(&decoded) {
                            Ok(event) => {
                                println!("✅ LastChange event parsing successful!");
                                println!("Event: {:?}", event);
                            }
                            Err(e) => {
                                println!("❌ LastChange event parsing failed: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Property parsing failed: {}", e);
                }
            }
        }
    }
}