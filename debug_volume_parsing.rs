use std::path::Path;

// Add the sonos crate to the path
fn main() {
    // Test the volume parsing with some sample XML
    let sample_xmls = vec![
        // Direct volume property
        r#"<property><Volume>50</Volume></property>"#,
        
        // Volume in LastChange
        r#"<property><LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume val="75"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></property>"#,
        
        // Namespaced property
        r#"<e:property xmlns:e="urn:schemas-upnp-org:event-1-0"><Volume>25</Volume></e:property>"#,
        
        // Real-world example with multiple properties
        r#"<e:property xmlns:e="urn:schemas-upnp-org:event-1-0">
            <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume channel="Master" val="30"/&gt;&lt;Mute channel="Master" val="0"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
        </e:property>"#,
    ];

    println!("Testing volume parsing with different XML formats...\n");
    
    for (i, xml) in sample_xmls.iter().enumerate() {
        println!("=== Test {} ===", i + 1);
        println!("XML: {}", xml);
        
        // We'll need to import the sonos crate to test this
        // For now, let's just print the XML to see what we're working with
        println!("Length: {} bytes", xml.len());
        println!();
    }
}