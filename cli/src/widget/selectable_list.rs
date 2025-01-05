use ratatui::{
  layout::Rect,
  style::{Style, Stylize},
  widgets::{Block, Borders, List, ListItem, ListState},
  Frame
};

pub struct SelectableList {
  title: String,
  items: Vec<String>,
  state: ListState,
}

impl SelectableList {
  pub fn new(title: &str, items: Vec<String>) -> Self {
      let mut state = ListState::default();
      state.select(Some(2));

      Self {
          items,
          title: title.to_string(),
          state,
      }
  }

  pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
      let list_items: Vec<ListItem> = self.items
          .iter()
          .map(|s| ListItem::new(s.as_str()))
          .collect();

      let list = List::new(list_items)
          .block(Block::default().title(self.title.clone()))
          .highlight_style(Style::new().reversed())
          .highlight_symbol(">> ");

      frame.render_stateful_widget(list, area, &mut self.state);
  }

  pub fn next(&mut self) {
    let i = self.state.selected().unwrap_or(0);
    let next = (i + 1) % self.items.len();
    self.state.select(None);
    self.state.select(Some(next));
  }

  pub fn previous(&mut self) {
      let i = match self.state.selected() {
          Some(i) => if i == 0 { self.items.len() - 1 } else { i - 1 },
          None => 0,
      };
      self.state.select(None);
      self.state.select(Some(i));
  }

  pub fn selected(&self) -> Option<usize> {
      self.state.selected()
  }
}
