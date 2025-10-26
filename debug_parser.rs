use sonos::services::zone_group_topology::parser::ZoneGroupTopologyParser;

fn main() {
    let xml = r#"
        <propertyset>
            <property>
                <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
            </property>
        </propertyset>
    "#;

    println!("Testing parser with XML:");
    println!("{}", xml);
    
    match ZoneGroupTopologyParser::from_xml(xml) {
        Ok(parser) => {
            println!("Parser created successfully");
            match parser.zone_groups() {
                Some(zone_groups) => {
                    println!("Found {} zone groups", zone_groups.len());
                    for (i, group) in zone_groups.iter().enumerate() {
                        println!("Group {}: coordinator={}, id={}, members={}", 
                                i, group.coordinator, group.id, group.members.len());
                    }
                }
                None => {
                    println!("Parser returned None for zone_groups()");
                }
            }
        }
        Err(e) => {
            println!("Parser creation failed: {}", e);
        }
    }
}