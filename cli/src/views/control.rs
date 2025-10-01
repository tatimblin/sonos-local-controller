use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use sonos::SpeakerController;
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
                SpeakerList::new(topology)
            } else {
                let empty_topology = TopologyList { items: vec![] };
                SpeakerList::new(&empty_topology)
            }
        });

        Self { store, list_widget }
    }

    fn get_selected_list(&self) -> String {
        self.store.with_state(|state| {
            if let Some(locked_uuid) = &state.selected_speaker_ip {
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

        // Get the current topology from the store and pass it to draw
        self.store.with_state(|state| {
            if let Some(topology) = &state.topology {
                self.list_widget.draw(frame, chunks[1], topology);
            } else {
                let empty_topology = TopologyList { items: vec![] };
                self.list_widget.draw(frame, chunks[1], &empty_topology);
            }
        });
    }

    fn handle_input(&mut self, key_event: KeyEvent, store: &Store) -> io::Result<()> {
        match key_event.code {
            KeyCode::Up => {
                self.list_widget.previous();
                let selected_item = store.with_state(|state| {
                    if let Some(topology) = &state.topology {
                        self.list_widget.selected(topology).cloned()
                    } else {
                        None
                    }
                });
                if let Some(item) = selected_item {
                    store.dispatch(AppAction::SetHighlight(item));
                }
            }
            KeyCode::Down => {
                self.list_widget.next();
                let selected_item = store.with_state(|state| {
                    if let Some(topology) = &state.topology {
                        self.list_widget.selected(topology).cloned()
                    } else {
                        None
                    }
                });
                if let Some(item) = selected_item {
                    store.dispatch(AppAction::SetHighlight(item));
                }
            }
            KeyCode::Left => {
                let highlighted_item = store.with_state(|state| state.highlight.clone());
                if let Some(topology_item) = highlighted_item {
                    let controller = SpeakerController::new();
                    match topology_item {
                        TopologyItem::Speaker { ip, coordinator_ip, uuid, group_uuid, .. } => {
                            if let Ok(new_volume) = controller.adjust_volume(&ip, -4) {
                                store.dispatch(AppAction::UpdateSpeakerVolume(uuid, new_volume));
                            }
                            if let Ok(new_group_volume) = controller.get_group_volume(&coordinator_ip) {
                                store.dispatch(AppAction::UpdateSpeakerVolume(group_uuid, new_group_volume));
                            }
                        }
                        TopologyItem::Group { ip, uuid, children, .. } => {
                            if let Ok(new_volume) = controller.adjust_volume(&ip, -4) {
                                store.dispatch(AppAction::UpdateSpeakerVolume(uuid, new_volume));
                            }
                            for (ip, uuid) in children {
                                if let Ok(volume) = controller.get_volume(&ip) {
                                    store.dispatch(AppAction::UpdateSpeakerVolume(uuid, volume));
                                }
                            }
                        }
                        TopologyItem::Satellite { .. } => {
                            // Satellites don't support direct volume control
                        }
                    }
                }
            }
            KeyCode::Right => {
                let highlighted_item = store.with_state(|state| state.highlight.clone());
                if let Some(topology_item) = highlighted_item {
                    let controller = SpeakerController::new();
                    match topology_item {
                        TopologyItem::Speaker { ip, coordinator_ip, uuid, group_uuid, .. } => {
                            if let Ok(new_volume) = controller.adjust_volume(&ip, 4) {
                                store.dispatch(AppAction::UpdateSpeakerVolume(uuid, new_volume));
                            }
                            if let Ok(new_group_volume) = controller.get_group_volume(&coordinator_ip) {
                                store.dispatch(AppAction::UpdateSpeakerVolume(group_uuid, new_group_volume));
                            }
                        }
                        TopologyItem::Group { ip, uuid, children, .. } => {
                            if let Ok(new_volume) = controller.adjust_volume(&ip, 4) {
                                store.dispatch(AppAction::UpdateSpeakerVolume(uuid, new_volume));
                            }
                            for (ip, uuid) in children {
                                log::debug!("Raise volume for {} {}", uuid, ip);
                                if let Ok(volume) = controller.get_volume(&ip) {
                                    store.dispatch(AppAction::UpdateSpeakerVolume(uuid, volume));
                                }
                            }
                        }
                        TopologyItem::Satellite { .. } => {
                            // Satellites don't support direct volume control
                        }
                    }
                }
            }
            KeyCode::Char(' ') => {
                // Toggle lock for the currently highlighted item if it's a speaker
                let selected_item = store.with_state(|state| {
                    if let Some(topology) = &state.topology {
                        self.list_widget.selected(topology).cloned()
                    } else {
                        None
                    }
                });
                if let Some(selected_item) = selected_item {
                    match selected_item {
                        TopologyItem::Speaker { ip, .. } => {
                            store.dispatch(AppAction::SetSelectSpeaker(ip));
                        }
                        TopologyItem::Group { .. } => {
                            // TODO: Handle group toggle
                        }
                        TopologyItem::Satellite { .. } => {
                            // TODO: Handle satellite toggle
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                let selected_item = store.with_state(|state| {
                    if let Some(topology) = &state.topology {
                        self.list_widget.selected(topology).cloned()
                    } else {
                        None
                    }
                });
                if let Some(selected_item) = selected_item {
                    match selected_item {
                        TopologyItem::Group { ip, .. } => {
                            let controller = SpeakerController::new();
                            let _ = controller.toggle_play_state(&ip);
                        }
                        TopologyItem::Speaker { .. } => todo!(),
                        TopologyItem::Satellite { .. } => todo!(),
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
