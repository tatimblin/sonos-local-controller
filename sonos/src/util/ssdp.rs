use std::net::UdpSocket;
use std::time::Duration;
use std::str;

pub trait UdpSocketTrait {
    fn send_to(&self, buf: &[u8], addr: &str) -> std::io::Result<usize>;
    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, String)>;
    fn set_multicast_loop_v4(&self, multicast_loop: bool) -> std::io::Result<()>;
    fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()>;
}

impl UdpSocketTrait for UdpSocket {
    fn send_to(&self, buf: &[u8], addr: &str) -> std::io::Result<usize> {
        UdpSocket::send_to(self, buf, addr)
    }

    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, String)> {
        let (size, src) = UdpSocket::recv_from(self, buf)?;
        Ok((size, src.to_string()))
    }

    fn set_multicast_loop_v4(&self, loop_v4: bool) -> std::io::Result<()> {
        UdpSocket::set_multicast_loop_v4(self, loop_v4)
    }
    
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        UdpSocket::set_read_timeout(self, timeout)
    }
}

#[derive(Clone, Copy)]
pub struct SsdpResponseIter<S: UdpSocketTrait> {
    socket: S,
    buf: [u8; 1024],
    finished: bool,
}

impl<S: UdpSocketTrait> SsdpResponseIter<S> {
    fn new(socket: S) -> Self {
        SsdpResponseIter {
            socket,
            buf: [0; 1024],
            finished: false,
        }
    }
}

impl<S: UdpSocketTrait> Iterator for SsdpResponseIter<S> {
    type Item = Result<SsdpResponse, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match self.socket.recv_from(&mut self.buf) {
            Ok((amt, _)) => {
                let response = str::from_utf8(&self.buf[..amt]).expect("Failed to parse response");
                Some(parse_ssdp_response(response).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to parse SSDP response")
                }))
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    self.finished = true;
                    None
                } else {
                    Some(Err(e))
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SsdpResponse {
    pub location: String,
    pub urn: String,
    pub usn: String,
    pub friendly_name: Option<String>,
}

impl SsdpResponse {
    pub fn new() -> Self {
        SsdpResponse {
            location: String::new(),
            urn: String::new(),
            usn: String::new(),
            friendly_name: None,
        }
    }
}

// Sends an SSDP M-SEARCH request and returns the responses as a vector.
pub fn send_ssdp_request<S: UdpSocketTrait>(socket: S, host: &str, urn: &str) -> std::io::Result<SsdpResponseIter<S>> {
    socket.set_multicast_loop_v4(true)?;
    socket.set_read_timeout(Some(Duration::from_millis(500)))?; // TODO: handle this better

    // SSDP M-SEARCH request
    let m_search = format!(
        "M-SEARCH * HTTP/1.1\r\n\
        HOST: {}\r\n\
        MAN: \"ssdp:discover\"\r\n\
        MX: 2\r\n\
        ST: {}\r\n\
        USER-AGENT: Rust/1.0 UPnP/1.0 MyClient/1.0\r\n\
        \r\n",
        host,
        urn
    );

    socket.send_to(m_search.as_bytes(), host)?;

    Ok(SsdpResponseIter::new(socket))
}

fn parse_ssdp_response(response: &str) -> Option<SsdpResponse> {
    let lines: Vec<&str> = response.split("\r\n").collect();
    let mut location = String::new();
    let mut urn = String::new();
    let mut usn = String::new();
    let mut friendly_name = None;

    for line in lines {
        if let Some(value) = get_value_from_key_value_line(line, "LOCATION: ") {
            location = value.to_string();
            continue;
        }
        if let Some(value) = get_value_from_key_value_line(line, "ST: ") {
            urn = value.to_string();
            continue;
        }
        if let Some(value) = get_value_from_key_value_line(line, "USN: ") {
            usn = value.to_string();
            continue;
        }
        if let Some(value) = get_value_from_key_value_line(line, "friendly-name: ") {
            friendly_name = Some(value.to_string());
            continue;
        }
    }

    if !location.is_empty() {
        Some(SsdpResponse { location, urn, usn, friendly_name })
    } else {
        None
    }
}

fn get_value_from_key_value_line<'a>(line: &'a str, prefix: &'a str) -> Option<&'a str> {
    if line.starts_with(prefix) {
        Some(line.trim_start_matches(prefix))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const MULTICAST_ADDRESS: &str = "239.255.255.250:1900";
    const SONOS_SEARCH_TARGET: &str = "urn:schemas-upnp-org:device:ZonePlayer:1";

    pub struct MockUdpSocket {
        responses: Vec<(usize, String)>,
        send_error: Option<Box<dyn std::error::Error>>,
        response_index: RefCell<usize>,
    }
    
    impl UdpSocketTrait for MockUdpSocket {
        fn send_to(&self, _buf: &[u8], _addr: &str) -> std::io::Result<usize> {
            if let Some(ref error) = self.send_error {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, error.to_string()));
            }
            Ok(0)
        }
    
        fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, String)> {
            let mut index = self.response_index.borrow_mut();

            if *index >= self.responses.len() {
                return Err(std::io::Error::from(std::io::ErrorKind::WouldBlock));
            }
    
            let (size, response) = self.responses[*index].clone();
            buf[..size].copy_from_slice(response.as_bytes());
    
            *index += 1;
    
            Ok((size, "mock_address".to_string()))
        }
    
        fn set_multicast_loop_v4(&self, _multicast_loop: bool) -> std::io::Result<()> {
            Ok(())
        }
    
        fn set_read_timeout(&self, _dur: Option<Duration>) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_send_ssdp_request_empty_response() {
        let mock_socket = MockUdpSocket {
            responses: vec![],
            send_error: None,
            response_index: RefCell::new(0),
        };

        let mut response_stream = send_ssdp_request(mock_socket, MULTICAST_ADDRESS, SONOS_SEARCH_TARGET).unwrap();

        let response = response_stream.next();
        
        assert!(response.is_none());
    }

    #[test]
    fn test_send_ssdp_request_send_error() {
        let mock_socket = MockUdpSocket {
            responses: vec![],
            send_error: Some(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Send failed"))),
            response_index: RefCell::new(0),
        };

        let response_stream = send_ssdp_request(mock_socket, MULTICAST_ADDRESS, SONOS_SEARCH_TARGET);

        assert!(response_stream.is_err());

        if let Err(err) = response_stream {
            assert_eq!(err.kind(), std::io::ErrorKind::Other);
        } else {
            panic!("Expected an error, but got Ok");
        }
    }

    #[test]
    fn test_send_ssdp_request_success() {
        let mock_socket = MockUdpSocket {
            responses: vec![
                (139, "HTTP/1.1 200 OK\r\nLOCATION: http://mock_device\r\nST: urn:schemas-upnp-org:device:ZonePlayer:1\r\nUSN: uuid:12345\r\nFriendlyName: Mock Device\r\n\r\n".to_string()),
            ],
            send_error: None,
            response_index: RefCell::new(0),
        };

        let mut response_stream = send_ssdp_request(mock_socket, MULTICAST_ADDRESS, SONOS_SEARCH_TARGET).unwrap();

        let response = response_stream.next();

        assert!(response.is_some());

        let response = response.unwrap();
    
        match response {
            Ok(ssdp_response) => {
                assert_eq!(ssdp_response, SsdpResponse {
                    location: "http://mock_device".to_string(),
                    urn: "urn:schemas-upnp-org:device:ZonePlayer:1".to_string(),
                    usn: "uuid:12345".to_string(),
                    friendly_name: None,
                });
            }
            Err(_) => panic!("Expected an Ok response, but got an error.")
        }
    }
}
