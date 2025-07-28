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
                SpeakerList::new_with(topology, |item| match item {
                    TopologyItem::Group { name, .. } => format!("▶ {name}"),
                    TopologyItem::Speaker { name, .. } => format!("  ├─ {name}"),
                    TopologyItem::Satellite { uuid } => format!("  Satellite: {uuid}"),
                })
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

        self.list_widget.draw(frame, chunks[1]);
    }

    fn handle_input(&mut self, key_event: KeyEvent, store: &Store) -> io::Result<()> {
        match key_event.code {
            KeyCode::Up => {
                self.list_widget.previous();
                if let Some(item) = self.list_widget.selected() {
                    store.dispatch(AppAction::SetHighlight(item.clone()));
                }
            }
            KeyCode::Down => {
                self.list_widget.next();
                if let Some(item) = self.list_widget.selected() {
                    store.dispatch(AppAction::SetHighlight(item.clone()));
                }
            }
            KeyCode::Left => {
                store.with_state(|state| {
                    if let Some(ref topology_item) = state.highlight {
                        match topology_item {
                            TopologyItem::Speaker { ip, .. } => {
                                let controller = SpeakerController::new();
                                let _ = controller.adjust_volume(ip, -4);
                            }
                            TopologyItem::Group { ip, .. } => {
                                let controller = SpeakerController::new();
                                let _ = controller.adjust_volume(ip, -4);
                            }
                            TopologyItem::Satellite { .. } => {
                                // Satellites don't support direct volume control
                            }
                        }
                    }
                });
            }
            KeyCode::Right => {
                store.with_state(|state| {
                    if let Some(ref topology_item) = state.highlight {
                        let controller = SpeakerController::new();
                        match topology_item {
                            TopologyItem::Speaker { ip, .. } => {
                                let _ = controller.adjust_volume(ip, 4);
                            }
                            TopologyItem::Group { ip, .. } => {
                                // Use coordinator IP for group volume control
                                let _ = controller.adjust_volume(ip, 4);
                            }
                            TopologyItem::Satellite { .. } => {
                                // Satellites don't support direct volume control
                            }
                        }
                    }
                });
            }
            KeyCode::Char(' ') => {
                // Toggle lock for the currently highlighted item if it's a speaker
                if let Some(selected_item) = self.list_widget.selected() {
                    match selected_item {
                        TopologyItem::Speaker { ip, .. } => {
                            store.dispatch(AppAction::SetSelectSpeaker(ip.clone()));
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
                if let Some(selected_item) = self.list_widget.selected() {
                    match selected_item {
                        TopologyItem::Group { ip, .. } => {
                            let controller = SpeakerController::new();
                            let _ = controller.toggle_play_state(ip);
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
