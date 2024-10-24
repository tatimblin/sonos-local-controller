use std::net::UdpSocket;
use regex::Regex;
use crate::util::ssdp::send_ssdp_request;

pub fn search() -> std::io::Result<()> {
  // Bind the UDP socket to a local address.
  let mut socket = UdpSocket::bind("0.0.0.0:0")?;
    
  // Call the SSDP request function, searching for Sonos devices.
  let responses = send_ssdp_request(
      &mut socket,
      "239.255.255.250:1900",
      "urn:schemas-upnp-org:device:ZonePlayer:1"
  )?;

  let location_regex = Regex::new(r"^https?://(.+?):1400/xml").unwrap();

  for response in responses {
      println!("Received response: {}", response);

      if let Some(captures) = location_regex.captures(&response) {
          let location = captures.get(1).map_or("", |m| m.as_str());
          println!("Device found at: {}", location);
      }
  }

  Ok(())
}
