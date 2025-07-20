use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::io;
use std::sync::Arc;

use crate::state::reducers::AppAction;
use crate::state::store::Store;
use crate::topology::topology_item::TopologyItem;
use crate::topology::topology_list::TopologyList;
use crate::widgets::speaker_list::SpeakerList;

use super::View;

pub struct ControlView {
    store: Arc<Store>,
    list_widget: SpeakerList,
}

impl ControlView {
    pub fn new(store: Arc<Store>) -> Self {
        let list_widget = store.with_state(|state| {
            if let Some(topology) = &state.topology {
                // Create topology list from actual topology data
                SpeakerList::new_with(topology, |item| match item {
                    TopologyItem::Group { uuid, name } => (format!("Group: {name}"), uuid.clone()),
                    TopologyItem::Speaker { uuid } => (format!("  Speaker: {uuid}"), uuid.clone()),
                    TopologyItem::Satellite { uuid } => {
                        (format!("  Satellite: {uuid}"), uuid.clone())
                    }
                })
            } else {
                // Create empty topology list for empty state
                let empty_topology = TopologyList { items: vec![] };
                SpeakerList::new(&empty_topology)
            }
        });

        Self { store, list_widget }
    }

    fn get_selected_list(&self) -> String {
        self.store.with_state(|state| {
            if let Some(locked_uuid) = &state.locked_speaker_uuid {
                locked_uuid.clone()
            } else {
                "No speaker locked".to_string()
            }
        })
    }
}

impl View for ControlView {
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(frame.area());
        let body = Text::from(self.get_selected_list());
        let body_paragraph = Paragraph::new(body).alignment(Alignment::Center);
        frame.render_widget(body_paragraph, chunks[0]);

        self.store.with_state(|state| {
            self.list_widget.draw(frame, chunks[1], state);
        });
    }

    fn handle_input(&mut self, key_event: KeyEvent, store: &Store) -> io::Result<()> {
        match key_event.code {
            KeyCode::Up => {
                self.list_widget.previous();
                // If we navigated to a speaker, set it as active
                if let Some(selected_item) = self.list_widget.selected() {
                    if let TopologyItem::Speaker { uuid } = selected_item {
                        store.dispatch(AppAction::SetActiveSpeaker(uuid.clone()));
                    }
                }
            }
            KeyCode::Down => {
                self.list_widget.next();
                // If we navigated to a speaker, set it as active
                if let Some(selected_item) = self.list_widget.selected() {
                    if let TopologyItem::Speaker { uuid } = selected_item {
                        store.dispatch(AppAction::SetActiveSpeaker(uuid.clone()));
                    }
                }
            }
            KeyCode::Left => {
                // Send volume down command to locked speaker if available
                if let Some(locked_uuid) = store.with_state(|state| state.locked_speaker_uuid.clone()) {
                    store.dispatch(AppAction::SendSpeakerCommand {
                        uuid: locked_uuid,
                        command: sonos::SpeakerCommand::AdjustVolume(-4),
                    });
                }
            }
            KeyCode::Right => {
                // Send volume up command to locked speaker if available
                if let Some(locked_uuid) = store.with_state(|state| state.locked_speaker_uuid.clone()) {
                    store.dispatch(AppAction::SendSpeakerCommand {
                        uuid: locked_uuid,
                        command: sonos::SpeakerCommand::AdjustVolume(4),
                    });
                }
            }
            KeyCode::Char(' ') => {
                // Toggle lock for the currently highlighted item if it's a speaker
                if let Some(selected_item) = self.list_widget.selected() {
                    if let TopologyItem::Speaker { uuid } = selected_item {
                        store.dispatch(AppAction::ToggleSpeakerLock(uuid.clone()));
                    }
                }
            }
            KeyCode::Char('p') => {
                // Send play command to locked speaker if available
                if let Some(locked_uuid) = store.with_state(|state| state.locked_speaker_uuid.clone()) {
                    store.dispatch(AppAction::SendSpeakerCommand {
                        uuid: locked_uuid,
                        command: sonos::SpeakerCommand::Play,
                    });
                }
            }
            KeyCode::Char('s') => {
                // Send pause command to locked speaker if available
                if let Some(locked_uuid) = store.with_state(|state| state.locked_speaker_uuid.clone()) {
                    store.dispatch(AppAction::SendSpeakerCommand {
                        uuid: locked_uuid,
                        command: sonos::SpeakerCommand::Pause,
                    });
                }
            }
            _ => {
                // Let unhandled keys (like 'q') pass through to the main app
            }
        }
        Ok(())
    }
}
