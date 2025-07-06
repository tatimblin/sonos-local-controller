use ratatui::{
  layout::Rect,
  style::{Style, Stylize},
  widgets::{Block, List, ListItem, ListState},
  Frame
};

#[derive(Clone)]
pub struct SelectableList {
  title: String,
  items: Vec<String>,
  state: ListState,
}

impl SelectableList {
  pub fn new(title: &str, items: Vec<String>) -> Self {
    let mut state = ListState::default();
    // Only select first item if list is not empty
    if !items.is_empty() {
      state.select(Some(0));
    }

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
    if self.items.is_empty() {
      return;
    }
    
    let i = self.state.selected().unwrap_or(0);
    let next = (i + 1) % self.items.len();
    self.state.select(None);
    self.state.select(Some(next));
  }

  pub fn previous(&mut self) {
    if self.items.is_empty() {
      return;
    }
    
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

  pub fn len(&self) -> usize {
    self.items.len()
  }
}

#[cfg(test)]
mod tests {
  use ratatui::{
    backend::TestBackend,
    Terminal,
  };

  use super::*;

  fn create_test_list() -> SelectableList {
    SelectableList::new(
      "Test List",
      vec!["Item 1".to_string(), "Item 2".to_string(), "Item 3".to_string()]
    )
  }

  #[test]
  fn test_new_list_creation() {
    let list = create_test_list();
    assert_eq!(list.title, "Test List");
    assert_eq!(list.items.len(), 3);
    assert_eq!(list.selected(), Some(0));
  }

  #[test]
  fn test_next_selection() {
    let mut list = create_test_list();
    assert_eq!(list.selected(), Some(0));

    list.next();
    assert_eq!(list.selected(), Some(1));

    list.next();
    assert_eq!(list.selected(), Some(2));

    list.next();
    assert_eq!(list.selected(), Some(0));
  }

  #[test]
  fn test_previous_selection() {
    let mut list = create_test_list();
    assert_eq!(list.selected(), Some(0));

    list.previous();
    assert_eq!(list.selected(), Some(2));

    list.previous();
    assert_eq!(list.selected(), Some(1));

    list.previous();
    assert_eq!(list.selected(), Some(0));
  }

  #[test]
  fn test_empty_list() {
    let list = SelectableList::new("Empty List", vec![]);
    assert_eq!(list.items.len(), 0);
    assert_eq!(list.selected(), None);
  }

  #[test]
  fn test_empty_list_navigation() {
    let mut list = SelectableList::new("Empty List", vec![]);
    assert_eq!(list.selected(), None);
    
    // Navigation should not panic or change selection on empty list
    list.next();
    assert_eq!(list.selected(), None);
    
    list.previous();
    assert_eq!(list.selected(), None);
  }

  #[test]
  fn test_selection_bounds() {
    let mut list = create_test_list();

    for _ in 0..10 {
      list.next();
      assert!(list.selected().unwrap() < list.items.len());
    }

    for _ in 0..10 {
      list.previous();
      assert!(list.selected().unwrap() < list.items.len());
    }
  }

  #[test]
  fn test_draw_doesnt_panic() {
    let mut list = create_test_list();

    let backend = TestBackend::new(10, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        list.draw(frame, frame.area());
        assert_eq!(list.items.len(), 3);
    }).unwrap();
  }
}
