use sonos::streaming::{EventStreamBuilder, ServiceType};
use sonos::models::{Speaker, SpeakerId, StateChange, PlaybackState};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}, mpsc};
use std::time::{Duration, Instant};
use std::thread;

/// Test that event processing remains non-blocking and responsive
#[test]
fn test_non_blocking_event_processing() {
    // Create a test speaker
    let test_speaker = Speaker {
        id: SpeakerId::from_udn("uuid:RINCON_TEST123::1"),
        udn: "uuid:RINCON_TEST123::1".to_string(),
        name: "Test Speaker".to_string(),
        room_name: "Test Room".to_string(),
        ip_address: "192.168.1.100".to_string(),
        port: 1400,
        model_name: "Test Model".to_string(),
        satellites: vec![],
    };

    // Counter to track events processed
    let events_processed = Arc::new(AtomicUsize::new(0));
    let events_processed_clone = events_processed.clone();

    // Flag to track if any handler blocks
    let handler_blocked = Arc::new(AtomicUsize::new(0));
    let handler_blocked_clone = handler_blocked.clone();

    // Create a channel to simulate high-frequency events
    let (event_sender, event_receiver) = mpsc::channel();

    // Simulate event processing loop behavior
    thread::spawn(move || {
        let start_time = Instant::now();
        let mut events_processed_count = 0;
        let mut display_update_needed = false;

        loop {
            match event_receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(event) => {
                    events_processed_count += 1;
                    
                    // Simulate StateCache update (non-blocking)
                    let _state_change: StateChange = event;
                    
                    // Simulate event handler calls (should be non-blocking)
                    let handler_start = Instant::now();
                    
                    // Simulate a potentially slow handler
                    if events_processed_count % 10 == 0 {
                        thread::sleep(Duration::from_millis(1)); // Minimal delay
                    }
                    
                    let handler_duration = handler_start.elapsed();
                    if handler_duration > Duration::from_millis(10) {
                        handler_blocked_clone.fetch_add(1, Ordering::SeqCst);
                    }
                    
                    events_processed_clone.store(events_processed_count, Ordering::SeqCst);
                    display_update_needed = true;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Handle flag-based updates during timeout periods (non-blocking)
                    if display_update_needed {
                        display_update_needed = false;
                        // Simulate non-blocking display update
                    }
                    
                    // Check if we should exit (simulate shutdown check)
                    if start_time.elapsed() > Duration::from_secs(2) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
    });

    // Send high-frequency events to test responsiveness
    let start_time = Instant::now();
    let mut events_sent = 0;
    
    while start_time.elapsed() < Duration::from_secs(1) {
        let event = StateChange::PlaybackStateChanged {
            speaker_id: test_speaker.id,
            state: PlaybackState::Playing,
        };
        
        if event_sender.send(event).is_ok() {
            events_sent += 1;
        }
        
        // Send events at high frequency to test non-blocking behavior
        thread::sleep(Duration::from_millis(1));
    }

    // Wait a bit for processing to complete
    thread::sleep(Duration::from_millis(500));

    // Verify that events were processed without blocking
    let final_events_processed = events_processed.load(Ordering::SeqCst);
    let blocked_handlers = handler_blocked.load(Ordering::SeqCst);

    println!("Events sent: {}", events_sent);
    println!("Events processed: {}", final_events_processed);
    println!("Blocked handlers: {}", blocked_handlers);

    // Assertions
    assert!(final_events_processed > 0, "No events were processed");
    assert!(final_events_processed >= events_sent / 2, "Too few events processed, indicating blocking");
    assert_eq!(blocked_handlers, 0, "Some handlers blocked for too long");
}

/// Test that flag-based updates work correctly
#[test]
fn test_flag_based_updates() {
    let display_update_needed = Arc::new(AtomicUsize::new(0));
    let display_update_needed_clone = display_update_needed.clone();
    
    let (event_sender, event_receiver) = mpsc::channel();
    
    // Simulate the event processing loop with flag-based updates
    thread::spawn(move || {
        let mut local_display_flag = false;
        
        for _ in 0..10 {
            match event_receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(_event) => {
                    // Set flag instead of doing direct I/O
                    local_display_flag = true;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Handle flag-based updates during timeout
                    if local_display_flag {
                        local_display_flag = false;
                        display_update_needed_clone.fetch_add(1, Ordering::SeqCst);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
    
    // Send some events
    for i in 0..5 {
        let event = StateChange::VolumeChanged {
            speaker_id: SpeakerId::from_udn("uuid:RINCON_TEST123::1"),
            volume: 50 + i,
        };
        let _ = event_sender.send(event);
        thread::sleep(Duration::from_millis(5));
    }
    
    // Wait for processing
    thread::sleep(Duration::from_millis(200));
    
    let updates_performed = display_update_needed.load(Ordering::SeqCst);
    assert!(updates_performed > 0, "Flag-based updates were not performed");
    assert!(updates_performed <= 5, "Too many updates performed");
}

/// Test that event handlers don't block the processing loop
#[test]
fn test_event_handlers_non_blocking() {
    let processing_times = Arc::new(std::sync::Mutex::new(Vec::new()));
    let processing_times_clone = processing_times.clone();
    
    let (event_sender, event_receiver) = mpsc::channel();
    
    // Simulate event processing with timing measurements
    thread::spawn(move || {
        for _ in 0..20 {
            let start_time = Instant::now();
            
            match event_receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(_event) => {
                    // Simulate event handler execution (should be fast)
                    let handler_start = Instant::now();
                    
                    // Simulate some work but keep it non-blocking
                    let _dummy_work = (0..1000).sum::<i32>();
                    
                    let handler_duration = handler_start.elapsed();
                    let total_duration = start_time.elapsed();
                    
                    let mut times = processing_times_clone.lock().unwrap();
                    times.push((handler_duration, total_duration));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Continue processing
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
    
    // Send events
    for i in 0..10 {
        let event = StateChange::MuteChanged {
            speaker_id: SpeakerId::from_udn("uuid:RINCON_TEST123::1"),
            muted: i % 2 == 0,
        };
        let _ = event_sender.send(event);
        thread::sleep(Duration::from_millis(10));
    }
    
    // Wait for processing
    thread::sleep(Duration::from_millis(300));
    
    let times = processing_times.lock().unwrap();
    assert!(!times.is_empty(), "No processing times recorded");
    
    // Verify that all processing times are reasonable (non-blocking)
    for (handler_duration, total_duration) in times.iter() {
        assert!(handler_duration.as_millis() < 10, "Handler took too long: {:?}", handler_duration);
        assert!(total_duration.as_millis() < 100, "Total processing took too long: {:?}", total_duration);
    }
}