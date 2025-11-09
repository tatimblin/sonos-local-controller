use crate::util::http;
use crate::util::ssdp::{send_ssdp_request, SsdpResponse};
use crate::{SonosError, SpeakerController, SpeakerInfo, Topology};
use log::{debug, error, info, warn};
use std::io::Error;
use std::io::Result as ioResult;
use std::net::UdpSocket;

pub fn discover_speakers() -> Result<Vec<SpeakerInfo>, SonosError> {
    Ok(discover_speakers_iter().collect())
}

pub fn discover_speakers_iter() -> Box<dyn Iterator<Item = SpeakerInfo>> {
    info!("Starting discovery process with iterator...");

    let responses = match setup_discovery() {
        Ok(responses) => responses,
        Err(e) => {
            error!("Failed to setup discovery: {}", e);
            return Box::new(std::iter::empty());
        }
    };

    info!("SSDP request sent, processing responses...");

    Box::new(
        responses
            .filter_map(|result| result.ok())
            .inspect(|response| {
                debug!(
                    "Processing SSDP response from location: {}",
                    response.location
                )
            })
            .filter_map(process_ssdp_response)
            .inspect(|info| info!("Found speaker: {} at {}", info.name, info.ip)),
    )
}

pub fn discover_topology() -> Result<Topology, SonosError> {
    info!("Starting topology discovery...");

    let responses = setup_discovery()
        .map_err(|e| SonosError::NetworkError(format!("Failed to setup discovery: {}", e)))?;

    // Find the first valid IP from SSDP responses
    for response_result in responses {
        if let Ok(response) = response_result {
            if let Some(ip) = http::get_ip_from_url(&response.location) {
                info!("Found speaker at {}, fetching topology...", ip);

                match Topology::from_ip(&ip) {
                    Ok(topology) => {
                        info!(
                            "Successfully retrieved topology with {} zone groups",
                            topology.zone_groups.len()
                        );
                        return Ok(topology);
                    }
                    Err(e) => {
                        warn!("Failed to get topology from {}: {}", ip, e);
                        continue;
                    }
                }
            }
        }
    }

    Err(SonosError::DeviceUnreachable)
}

/// Set up the discovery process by creating a UDP socket and sending an SSDP request
fn setup_discovery(
) -> ioResult<impl Iterator<Item = Result<crate::util::ssdp::SsdpResponse, Error>>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    let responses = send_ssdp_request(
        socket,
        "239.255.255.250:1900",
        "urn:schemas-upnp-org:device:ZonePlayer:1",
    )?;

    Ok(responses)
}

/// Process an SSDP response and extract speaker information
fn process_ssdp_response(response: SsdpResponse) -> Option<SpeakerInfo> {
  let ip = http::get_ip_from_url(&response.location)?;
  SpeakerInfo::from_location(&ip).ok()
}
