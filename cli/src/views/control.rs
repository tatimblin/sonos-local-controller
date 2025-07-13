use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::text::Text;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::io;
use std::sync::Arc;

use crate::state::reducers::AppAction;
use crate::state::store::Store;
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
				SpeakerList::new_with(
					topology,
					|uuid| {
							format!("Group2: {uuid}")
					},
					|uuid| {
							format!("Speaker2: {uuid}")
					},
					|uuid| {
							format!("Satellite2: {uuid}")
					}
				)
			} else {
				// Create empty topology list for empty state
				let empty_topology = TopologyList { items: vec!() };
				SpeakerList::new(&empty_topology)
			}
		});

		Self { store, list_widget }
	}

	fn get_selected_list(&self) -> String {
    self.store.with_state(|state| {
      state.selected_speaker_uuids
				.iter()
				.map(|x| x.to_string() + ",")
				.collect::<String>()
    })
  }
}

impl View for ControlView {
	fn render(&mut self, frame: &mut Frame) {
		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints([
					Constraint::Length(1),
					Constraint::Min(0),
			])
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
			}
			KeyCode::Down => {
				self.list_widget.next();
			}
			KeyCode::Left => {
				store.dispatch(AppAction::AdjustVolume(-4));
			}
			KeyCode::Right => {
				store.dispatch(AppAction::AdjustVolume(4));
			}
			KeyCode::Char(' ') => {
				if let Some(uuid) = self.list_widget.selected_uuid() {
					store.dispatch(AppAction::ToggleSelect(uuid.to_owned()));
				}
			}
			_ => {}
		}
		Ok(())
	}
}
