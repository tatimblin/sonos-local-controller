// Temporary test file to verify SpeakerCommand export
use sonos::SpeakerCommand;

fn main() {
    let commands = vec![
        SpeakerCommand::Play,
        SpeakerCommand::Pause,
        SpeakerCommand::SetVolume(50),
        SpeakerCommand::AdjustVolume(-10),
    ];
    
    for cmd in commands {
        println!("Command: {:?}", cmd);
    }
    
    println!("SpeakerCommand enum is properly exported and working!");
}