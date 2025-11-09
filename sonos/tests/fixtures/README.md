# Test Fixtures

This directory contains XML fixtures used in unit tests for the Sonos discovery functionality.

## Files

- `sonos_one_device.xml` - Device description XML for a Sonos One speaker in the Living Room
- `sonos_play1_device.xml` - Device description XML for a Sonos Play:1 speaker in the Kitchen  
- `minimal_sonos_device.xml` - Minimal Sonos device XML with only required fields (missing optional roomName)
- `non_sonos_router_device.xml` - Non-Sonos device XML (router) used to test device filtering

## Usage

These fixtures are loaded in tests using `include_str!()` macro:

```rust
let device_xml = include_str!("../../tests/fixtures/sonos_one_device.xml");
let device = Device::from_xml(device_xml).unwrap();
```

## Adding New Fixtures

When adding new fixtures:
1. Use realistic Sonos device data when possible
2. Follow the UPnP device description XML format
3. Include both positive (Sonos) and negative (non-Sonos) test cases
4. Update this README with descriptions of new files