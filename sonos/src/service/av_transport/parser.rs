use crate::{PlaybackState, models::TrackInfo, xml_decode::{NestedAttribute, ValueAttribute}};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename = "propertyset")]
pub struct AVTransportParser {
    #[serde(rename = "property")]
    pub property: Property,
}

#[derive(Debug, Deserialize)]
pub struct Property {
    #[serde(
        rename = "LastChange",
        deserialize_with = "crate::xml_decode::xml_decode::deserialize_nested"
    )]
    pub last_change: LastChangeEvent,
}

// The root element for decoded LastChange content
#[derive(Debug, Deserialize)]
#[serde(rename = "Event")]
pub struct LastChangeEvent {
    #[serde(rename = "InstanceID")]
    pub instance: InstanceID,
}

#[derive(Debug, Deserialize)]
pub struct InstanceID {
    #[serde(rename = "@val")]
    pub id: String,

    #[serde(rename = "TransportState")]
    pub transport_state: ValueAttribute,

    #[serde(rename = "CurrentPlayMode")]
    pub current_play_mode: ValueAttribute,

    #[serde(rename = "CurrentCrossfadeMode", default)]
    pub current_crossfade_mode: Option<ValueAttribute>,

    #[serde(rename = "NumberOfTracks")]
    pub number_of_tracks: ValueAttribute,

    #[serde(rename = "CurrentTrack")]
    pub current_track: ValueAttribute,

    #[serde(rename = "CurrentSection", default)]
    pub current_section: Option<ValueAttribute>,

    #[serde(rename = "CurrentTrackURI")]
    pub current_track_uri: ValueAttribute,

    #[serde(rename = "CurrentTrackDuration")]
    pub current_track_duration: ValueAttribute,

    #[serde(rename = "CurrentTrackMetaData")]
    pub current_track_metadata: NestedAttribute<DidlLite>,

    #[serde(rename = "NextTrackURI", default)]
    pub next_track_uri: Option<ValueAttribute>,

    #[serde(rename = "NextTrackMetaData", default)]
    pub next_track_metadata: Option<ValueAttribute>,

    #[serde(rename = "EnqueuedTransportURI", default)]
    pub enqueued_transport_uri: Option<ValueAttribute>,

    #[serde(rename = "EnqueuedTransportURIMetaData", default)]
    pub enqueued_transport_uri_metadata: Option<ValueAttribute>,

    #[serde(rename = "TransportStatus", default)]
    pub transport_status: Option<ValueAttribute>,

    #[serde(rename = "CurrentTransportActions", default)]
    pub current_transport_actions: Option<ValueAttribute>,

    #[serde(rename = "PlaybackStorageMedium", default)]
    pub playback_storage_medium: Option<ValueAttribute>,

    #[serde(rename = "RecordStorageMedium", default)]
    pub record_storage_medium: Option<ValueAttribute>,

    #[serde(rename = "PossiblePlaybackStorageMedia", default)]
    pub possible_playback_storage_media: Option<ValueAttribute>,

    #[serde(rename = "PossibleRecordStorageMedia", default)]
    pub possible_record_storage_media: Option<ValueAttribute>,

    #[serde(rename = "RecordMediumWriteStatus", default)]
    pub record_medium_write_status: Option<ValueAttribute>,

    #[serde(rename = "CurrentRecordQualityMode", default)]
    pub current_record_quality_mode: Option<ValueAttribute>,

    #[serde(rename = "PossibleRecordQualityModes", default)]
    pub possible_record_quality_modes: Option<ValueAttribute>,

    #[serde(rename = "AVTransportURI", default)]
    pub av_transport_uri: Option<ValueAttribute>,

    #[serde(rename = "AVTransportURIMetaData", default)]
    pub av_transport_uri_metadata: Option<ValueAttribute>,

    #[serde(rename = "RelativeTimePosition", default)]
    pub relative_time_position: Option<ValueAttribute>,

    #[serde(rename = "AbsoluteTimePosition", default)]
    pub absolute_time_position: Option<ValueAttribute>,

    #[serde(rename = "RelativeCounterPosition", default)]
    pub relative_counter_position: Option<ValueAttribute>,

    #[serde(rename = "AbsoluteCounterPosition", default)]
    pub absolute_counter_position: Option<ValueAttribute>,
}

// DIDL-Lite metadata structure
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "DIDL-Lite")]
pub struct DidlLite {
    #[serde(rename = "item")]
    pub item: DidlItem,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DidlItem {
    #[serde(rename = "@id")]
    pub id: String,

    #[serde(rename = "@parentID")]
    pub parent_id: String,

    #[serde(rename = "@restricted", default)]
    pub restricted: Option<String>,

    #[serde(rename = "res")]
    pub res: DidlResource,

    #[serde(rename = "albumArtURI")]
    pub album_art_uri: Option<String>,

    #[serde(rename = "class")]
    pub class: String,

    #[serde(rename = "title")]
    pub title: String,

    #[serde(rename = "creator")]
    pub creator: Option<String>,

    #[serde(rename = "album")]
    pub album: Option<String>,

    #[serde(rename = "streamInfo", default)]
    pub stream_info: Option<String>,

    #[serde(rename = "desc", default)]
    pub desc: Option<DidlDesc>,

    #[serde(rename = "contentService", default)]
    pub content_service: Option<DidlContentService>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DidlResource {
    #[serde(rename = "@duration")]
    pub duration: Option<String>,

    #[serde(rename = "$value")]
    pub uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DidlDesc {
    #[serde(rename = "@id")]
    pub id: String,

    #[serde(rename = "@nameSpace")]
    pub name_space: String,

    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DidlContentService {
    #[serde(rename = "@id")]
    pub id: String,

    #[serde(rename = "@name")]
    pub name: String,
}

impl AVTransportParser {
    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::DeError> {
        // Use the automatic namespace stripping from xml_decode
        crate::xml_decode::xml_decode::parse(xml)
    }

    pub fn get_playback_state(&self) -> Option<PlaybackState> {
      println!("");
      println!("");
      println!("TRISTAN");
      println!("");
      println!("");
      println!("get_playback_state() --> {}", self
            .property
            .last_change
            .instance
            .transport_state
            .val
            .as_str());
        match self
            .property
            .last_change
            .instance
            .transport_state
            .val
            .as_str()
        {
            "PLAYING" => Some(PlaybackState::Playing),
            "PAUSED_PLAYBACK" => Some(PlaybackState::Paused),
            "STOPPED" => Some(PlaybackState::Stopped),
            "TRANSITIONING" => Some(PlaybackState::Transitioning),
            _ => None,
        }
    }

    pub fn get_track_info(&self) -> Option<TrackInfo> {
        let didl = self
            .property
            .last_change
            .instance
            .current_track_metadata
            .val
            .as_ref()?;
        let duration_ms = self.parse_duration(
            &self
                .property
                .last_change
                .instance
                .current_track_duration
                .val,
        );
        let uri = &self.property.last_change.instance.current_track_uri.val;
        Some(TrackInfo {
            title: Some(didl.item.title.clone()),
            artist: didl.item.creator.clone(),
            album: didl.item.album.clone(),
            duration_ms,
            uri: Some(uri.clone()),
        })
    }

    fn parse_duration(&self, duration_str: &str) -> Option<u64> {
        let parts: Vec<&str> = duration_str.split(':').collect();
        if parts.len() >= 3 {
            let hours: u64 = parts[0].parse().ok()?;
            let minutes: u64 = parts[1].parse().ok()?;

            // Handle seconds with optional milliseconds
            let seconds_part = parts[2];
            let seconds: f64 = seconds_part.parse().ok()?;

            let total_ms = (hours * 3600 + minutes * 60) * 1000 + (seconds * 1000.0) as u64;
            Some(total_ms)
        } else {
            None
        }
    }
}

impl LastChangeEvent {
    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::DeError> {
        crate::xml_decode::xml_decode::parse(xml)
    }
}

impl DidlLite {
    pub fn from_xml(xml: &str) -> Result<Self, quick_xml::DeError> {
        crate::xml_decode::xml_decode::parse(xml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use the full XML for each test only
    const SAMPLE_XML: &str = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><LastChange>&lt;Event xmlns=&quot;urn:schemas-upnp-org:metadata-1-0/AVT/&quot; xmlns:r=&quot;urn:schemas-rinconnetworks-com:metadata-1-0/&quot;&gt;&lt;InstanceID val=&quot;0&quot;&gt;&lt;TransportState val=&quot;PAUSED_PLAYBACK&quot;/&gt;&lt;CurrentPlayMode val=&quot;REPEAT_ALL&quot;/&gt;&lt;CurrentCrossfadeMode val=&quot;0&quot;/&gt;&lt;NumberOfTracks val=&quot;1&quot;/&gt;&lt;CurrentTrack val=&quot;1&quot;/&gt;&lt;CurrentSection val=&quot;0&quot;/&gt;&lt;CurrentTrackURI val=&quot;x-sonos-spotify:spotify:track:5hM5arv9KDbCHS0k9uqwjr?sid=12&amp;amp;flags=0&amp;amp;sn=2&quot;/&gt;&lt;CurrentTrackDuration val=&quot;0:03:57&quot;/&gt;&lt;CurrentTrackMetaData val=&quot;&amp;lt;DIDL-Lite xmlns:dc=&amp;quot;http://purl.org/dc/elements/1.1/&amp;quot; xmlns:upnp=&amp;quot;urn:schemas-upnp-org:metadata-1-0/upnp/&amp;quot; xmlns:r=&amp;quot;urn:schemas-rinconnetworks-com:metadata-1-0/&amp;quot; xmlns=&amp;quot;urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/&amp;quot;&amp;gt;&amp;lt;item id=&amp;quot;-1&amp;quot; parentID=&amp;quot;-1&amp;quot;&gt;&amp;lt;res duration=&amp;quot;0:03:58&amp;quot;&gt;x-sonos-spotify:spotify:track:5hM5arv9KDbCHS0k9uqwjr?sid=12&amp;amp;amp;flags=0&amp;amp;amp;sn=2&amp;lt;/res&amp;gt;&amp;lt;upnp:albumArtURI&amp;gt;https://i.scdn.co/image/ab67616d0000b27358267bd34420a00d5cf83a49&amp;lt;/upnp:albumArtURI&amp;gt;&amp;lt;upnp:class&amp;gt;object.item.audioItem.musicTrack&amp;lt;/upnp:class&amp;gt;&amp;lt;dc:title&amp;gt;Borderline&amp;lt;/dc:title&amp;gt;&amp;lt;dc:creator&amp;gt;Tame Impala&amp;lt;/dc:creator&amp;gt;&amp;lt;upnp:album&amp;gt;The Slow Rush&amp;lt;/upnp:album&amp;gt;&amp;lt;r:streamInfo&amp;gt;bd:16,sr:44100,c:0,l:0,d:0&amp;lt;/r:streamInfo&amp;gt;&amp;lt;/item&amp;gt;&amp;lt;/DIDL-Lite&amp;gt;&quot;/&gt;&lt;r:NextTrackURI val=&quot;&quot;/&gt;&lt;r:NextTrackMetaData val=&quot;&amp;lt;DIDL-Lite xmlns:dc=&amp;quot;http://purl.org/dc/elements/1.1/&amp;quot; xmlns:upnp=&amp;quot;urn:schemas-upnp-org:metadata-1-0/upnp/&amp;quot; xmlns:r=&amp;quot;urn:schemas-rinconnetworks-com:metadata-1-0/&amp;quot; xmlns=&amp;quot;urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/&amp;quot;&amp;gt;&amp;lt;item id=&amp;quot;-1&amp;quot; parentID=&amp;quot;-1&quot;&gt;&amp;lt;res&amp;gt;&amp;lt;/res&amp;gt;&amp;lt;upnp:albumArtURI&amp;gt;&amp;lt;/upnp:albumArtURI&amp;gt;&amp;lt;upnp:class&amp;gt;object.item.audioItem.musicTrack&amp;lt;/upnp:class&amp;gt;&amp;lt;dc:title&amp;gt;Pink + White&amp;lt;/dc:title&amp;gt;&amp;lt;dc:creator&amp;gt;Frank Ocean&amp;lt;/dc:creator&amp;gt;&amp;lt;upnp:album&amp;gt;Blonde&amp;lt;/upnp:album&amp;gt;&amp;lt;/item&amp;gt;&amp;lt;/DIDL-Lite&amp;gt;&quot;/&gt;&lt;r:EnqueuedTransportURI val=&quot;&quot;/&gt;&lt;r:EnqueuedTransportURIMetaData val=&quot;&amp;lt;DIDL-Lite xmlns:dc=&amp;quot;http://purl.org/dc/elements/1.1/&amp;quot; xmlns:upnp=&amp;quot;urn:schemas-upnp-org:metadata-1-0/upnp/&amp;quot; xmlns:r=&amp;quot;urn:schemas-rinconnetworks-com:metadata-1-0/&amp;quot; xmlns=&amp;quot;urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/&amp;quot;&amp;gt;&amp;lt;item id=&amp;quot;&quot; parentID=&quot;-1&quot; restricted=&quot;true&quot;&gt;&amp;lt;dc:title&amp;gt;Daily Mix 2&amp;lt;/dc:title&amp;gt;&amp;lt;upnp:class&amp;gt;object.container.playlistContainer&amp;lt;/upnp:class&amp;gt;&amp;lt;desc id=&quot;cdudn&quot; nameSpace=&quot;urn:schemas-rinconnetworks-com:metadata-1-0/&quot;&gt;SA_RINCON3079_X_#Svc3079-14ddbab7-Token&amp;lt;/desc&amp;gt;&amp;lt;upnp:albumArtURI&amp;gt;&amp;lt;/upnp:albumArtURI&amp;gt;&amp;lt;r:contentService id=&quot;12&quot; name=&quot;Spotify&quot;/&gt;&amp;lt;/item&amp;gt;&amp;lt;/DIDL-Lite&amp;gt;&quot;/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></e:property></e:propertyset>"#;

    #[test]
    fn test_parse_av_transport_xml() {
        // Parse the entire complex XML structure in one go
        let result = AVTransportParser::from_xml(SAMPLE_XML);
        assert!(
            result.is_ok(),
            "Failed to parse XML: {:?}",
            result.as_ref().err()
        );

        let parsed = result.unwrap();

        println!("✅ Successfully parsed AVTransport XML in one go");

        // LastChange is now automatically parsed into LastChangeEvent
        let last_change = &parsed.property.last_change;

        // Assert on the parsed LastChange event structure
        assert_eq!(last_change.instance.id, "0");
        assert_eq!(last_change.instance.transport_state.val, "PAUSED_PLAYBACK");
        assert_eq!(last_change.instance.current_play_mode.val, "REPEAT_ALL");
        assert_eq!(last_change.instance.current_track_duration.val, "0:03:57");
        assert_eq!(last_change.instance.number_of_tracks.val, "1");
        assert_eq!(last_change.instance.current_track.val, "1");

        // Assert on optional fields
        assert!(last_change.instance.current_crossfade_mode.is_some());
        assert_eq!(
            last_change
                .instance
                .current_crossfade_mode
                .as_ref()
                .unwrap()
                .val,
            "0"
        );

        // Assert that the track URI contains expected content
        assert!(last_change
            .instance
            .current_track_uri
            .val
            .contains("spotify:track"));
    }

    #[test]
    fn test_parse_didl_metadata() {
        let didl_xml = r#"<DIDL-Lite xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns:r="urn:schemas-rinconnetworks-com:metadata-1-0/" xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/"><item id="-1" parentID="-1"><res duration="0:03:58">x-sonos-spotify:spotify:track:5hM5arv9KDbCHS0k9uqwjr?sid=12&amp;flags=0&amp;sn=2</res><upnp:albumArtURI>https://i.scdn.co/image/ab67616d0000b27358267bd34420a00d5cf83a49</upnp:albumArtURI><upnp:class>object.item.audioItem.musicTrack</upnp:class><dc:title>Borderline</dc:title><dc:creator>Tame Impala</dc:creator><upnp:album>The Slow Rush</upnp:album><r:streamInfo>bd:16,sr:44100,c:0,l:0,d:0</r:streamInfo></item></DIDL-Lite>"#;

        let result = DidlLite::from_xml(didl_xml);
        assert!(
            result.is_ok(),
            "Failed to parse DIDL-Lite XML: {:?}",
            result.err()
        );

        let didl = result.unwrap();
        assert_eq!(didl.item.title, "Borderline");
        assert_eq!(didl.item.creator, Some("Tame Impala".to_string()));
        assert_eq!(didl.item.album, Some("The Slow Rush".to_string()));

        // Test the new streamInfo field
        assert!(didl.item.stream_info.is_some());
        assert_eq!(
            didl.item.stream_info.as_ref().unwrap(),
            "bd:16,sr:44100,c:0,l:0,d:0"
        );

        println!("✅ Successfully parsed DIDL-Lite metadata");
        println!(
            "🎵 Track: {} by {}",
            didl.item.title,
            didl.item.creator.unwrap_or_default()
        );
        if let Some(stream_info) = &didl.item.stream_info {
            println!("🎧 Stream info: {}", stream_info);
        }
    }

    #[test]
    fn test_parse_raw_event_with_new_fields() {
        // Test with the actual raw event provided by the user
        let raw_event = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><LastChange>&lt;Event xmlns=&quot;urn:schemas-upnp-org:metadata-1-0/AVT/&quot; xmlns:r=&quot;urn:schemas-rinconnetworks-com:metadata-1-0/&quot;&gt;&lt;InstanceID val=&quot;0&quot;&gt;&lt;TransportState val=&quot;PAUSED_PLAYBACK&quot;/&gt;&lt;CurrentPlayMode val=&quot;REPEAT_ALL&quot;/&gt;&lt;CurrentCrossfadeMode val=&quot;0&quot;/&gt;&lt;NumberOfTracks val=&quot;1&quot;/&gt;&lt;CurrentTrack val=&quot;1&quot;/&gt;&lt;CurrentSection val=&quot;0&quot;/&gt;&lt;CurrentTrackURI val=&quot;x-sonos-spotify:spotify:track:5hM5arv9KDbCHS0k9uqwjr?sid=12&amp;amp;flags=0&amp;amp;sn=2&quot;/&gt;&lt;CurrentTrackDuration val=&quot;0:03:57&quot;/&gt;&lt;CurrentTrackMetaData val=&quot;&amp;lt;DIDL-Lite xmlns:dc=&amp;quot;http://purl.org/dc/elements/1.1/&amp;quot; xmlns:upnp=&amp;quot;urn:schemas-upnp-org:metadata-1-0/upnp/&amp;quot; xmlns:r=&amp;quot;urn:schemas-rinconnetworks-com:metadata-1-0/&amp;quot; xmlns=&amp;quot;urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/&amp;quot;&amp;gt;&amp;lt;item id=&amp;quot;-1&amp;quot; parentID=&amp;quot;-1&amp;quot;&gt;&amp;lt;res duration=&amp;quot;0:03:58&amp;quot;&gt;x-sonos-spotify:spotify:track:5hM5arv9KDbCHS0k9uqwjr?sid=12&amp;amp;amp;flags=0&amp;amp;amp;sn=2&amp;lt;/res&amp;gt;&amp;lt;upnp:albumArtURI&amp;gt;https://i.scdn.co/image/ab67616d0000b27358267bd34420a00d5cf83a49&amp;lt;/upnp:albumArtURI&amp;gt;&amp;lt;upnp:class&amp;gt;object.item.audioItem.musicTrack&amp;lt;/upnp:class&amp;gt;&amp;lt;dc:title&amp;gt;Borderline&amp;lt;/dc:title&amp;gt;&amp;lt;dc:creator&amp;gt;Tame Impala&amp;lt;/dc:creator&amp;gt;&amp;lt;upnp:album&amp;gt;The Slow Rush&amp;lt;/upnp:album&amp;gt;&amp;lt;r:streamInfo&amp;gt;bd:16,sr:44100,c:0,l:0,d:0&amp;lt;/r:streamInfo&amp;gt;&amp;lt;/item&amp;gt;&amp;lt;/DIDL-Lite&amp;gt;&quot;/&gt;&lt;r:NextTrackURI val=&quot;&quot;/&gt;&lt;r:NextTrackMetaData val=&quot;&amp;lt;DIDL-Lite xmlns:dc=&amp;quot;http://purl.org/dc/elements/1.1/&amp;quot; xmlns:upnp=&amp;quot;urn:schemas-upnp-org:metadata-1-0/upnp/&amp;quot; xmlns:r=&amp;quot;urn:schemas-rinconnetworks-com:metadata-1-0/&amp;quot; xmlns=&amp;quot;urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/&amp;quot;&amp;gt;&amp;lt;item id=&amp;quot;-1&amp;quot; parentID=&amp;quot;-1&quot;&gt;&amp;lt;res&amp;gt;&amp;lt;/res&amp;gt;&amp;lt;upnp:albumArtURI&amp;gt;&amp;lt;/upnp:albumArtURI&amp;gt;&amp;lt;upnp:class&amp;gt;object.item.audioItem.musicTrack&amp;lt;/upnp:class&amp;gt;&amp;lt;dc:title&amp;gt;Pink + White&amp;lt;/dc:title&amp;gt;&amp;lt;dc:creator&amp;gt;Frank Ocean&amp;lt;/dc:creator&amp;gt;&amp;lt;upnp:album&amp;gt;Blonde&amp;lt;/upnp:album&amp;gt;&amp;lt;/item&amp;gt;&amp;lt;/DIDL-Lite&amp;gt;&quot;/&gt;&lt;r:EnqueuedTransportURI val=&quot;&quot;/&gt;&lt;r:EnqueuedTransportURIMetaData val=&quot;&amp;lt;DIDL-Lite xmlns:dc=&amp;quot;http://purl.org/dc/elements/1.1/&amp;quot; xmlns:upnp=&amp;quot;urn:schemas-upnp-org:metadata-1-0/upnp/&amp;quot; xmlns:r=&amp;quot;urn:schemas-rinconnetworks-com:metadata-1-0/&amp;quot; xmlns=&amp;quot;urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/&amp;quot;&amp;gt;&amp;lt;item id=&amp;quot;&quot; parentID=&quot;-1&quot; restricted=&quot;true&quot;&gt;&amp;lt;dc:title&amp;gt;Daily Mix 2&amp;lt;/dc:title&amp;gt;&amp;lt;upnp:class&amp;gt;object.container.playlistContainer&amp;lt;/upnp:class&amp;gt;&amp;lt;desc id=&quot;cdudn&quot; nameSpace=&quot;urn:schemas-rinconnetworks-com:metadata-1-0/&quot;&gt;SA_RINCON3079_X_#Svc3079-14ddbab7-Token&amp;lt;/desc&amp;gt;&amp;lt;upnp:albumArtURI&amp;gt;&amp;lt;/upnp:albumArtURI&amp;gt;&amp;lt;r:contentService id=&quot;12&quot; name=&quot;Spotify&quot;/&gt;&amp;lt;/item&amp;gt;&amp;lt;/DIDL-Lite&amp;gt;&quot;/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></e:property></e:propertyset>"#;

        let result = AVTransportParser::from_xml(raw_event);
        assert!(
            result.is_ok(),
            "Failed to parse raw event: {:?}",
            result.err()
        );

        let parsed = result.unwrap();
        let instance = &parsed.property.last_change.instance;

        // Verify all the core fields are parsed correctly
        assert_eq!(instance.id, "0");
        assert_eq!(instance.transport_state.val, "PAUSED_PLAYBACK");
        assert_eq!(instance.current_play_mode.val, "REPEAT_ALL");
        assert_eq!(instance.current_track_duration.val, "0:03:57");

        // Verify that the track URI contains the expected Spotify track ID
        assert!(instance
            .current_track_uri
            .val
            .contains("5hM5arv9KDbCHS0k9uqwjr"));

        // Verify that next track and enqueued transport URIs are empty (as expected)
        assert!(instance.next_track_uri.is_some());
        assert_eq!(instance.next_track_uri.as_ref().unwrap().val, "");

        assert!(instance.enqueued_transport_uri.is_some());
        assert_eq!(instance.enqueued_transport_uri.as_ref().unwrap().val, "");

        println!("✅ Successfully parsed complete raw event with all fields");
    }
}
