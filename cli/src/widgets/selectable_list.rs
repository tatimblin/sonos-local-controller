use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, List, ListItem, ListState},
    Frame,
};

#[derive(Clone)]
pub struct SelectableList {
    title: String,
    items: Vec<ListItem<'static>>,
    state: ListState,
}

impl SelectableList {
    pub fn new(title: &str, items: Vec<ListItem<'static>>) -> Self {
        let mut state = ListState::default();
        // Only select first item if list is not empty
        if !items.is_empty() {
            state.select(Some(0));
        }

        Self {
            title: title.to_string(),
            items,
            state,
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let list = List::new(self.items.clone())
            .block(Block::default().title(self.title.clone()))
            .highlight_style(Style::new().reversed())
            .highlight_symbol("≡ ");

        frame.render_stateful_widget(list, area, &mut self.state);
    }

    pub fn next(&mut self) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }

        let i = self.state.selected().unwrap_or(0);
        let next = (i + 1) % self.items.len();
        self.state.select(Some(next));
        Some(next)
    }

    pub fn previous(&mut self) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        Some(i)
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn update_items(&mut self, items: Vec<ListItem<'static>>) {
        self.items = items;
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;

    fn create_test_list() -> SelectableList {
        SelectableList::new(
            "Test List",
            vec![
                ListItem::new("Item 1"),
                ListItem::new("Item 2"),
                ListItem::new("Item 3"),
            ],
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

        terminal
            .draw(|frame| {
                list.draw(frame, frame.area());
                assert_eq!(list.items.len(), 3);
            })
            .unwrap();
    }

    #[test]
    fn test_list_item_creation() {
        let items = vec![
            ListItem::new("First Item"),
            ListItem::new("Second Item"),
            ListItem::new("Third Item"),
        ];
        let list = SelectableList::new("ListItem Test", items);

        assert_eq!(list.title, "ListItem Test");
        assert_eq!(list.items.len(), 3);
        assert_eq!(list.selected(), Some(0));
    }

    #[test]
    fn test_list_item_with_styled_content() {
        use ratatui::style::{Color, Style};
        use ratatui::text::{Line, Span};

        let styled_item = ListItem::new(Line::from(vec![
            Span::styled("Styled", Style::default().fg(Color::Red)),
            Span::raw(" Item"),
        ]));

        let items = vec![ListItem::new("Normal Item"), styled_item];

        let list = SelectableList::new("Styled Test", items);
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.selected(), Some(0));
    }

    #[test]
    fn test_navigation_with_list_items() {
        let items = vec![
            ListItem::new("Alpha"),
            ListItem::new("Beta"),
            ListItem::new("Gamma"),
            ListItem::new("Delta"),
        ];
        let mut list = SelectableList::new("Navigation Test", items);

        // Test forward navigation
        assert_eq!(list.selected(), Some(0));
        list.next();
        assert_eq!(list.selected(), Some(1));
        list.next();
        assert_eq!(list.selected(), Some(2));
        list.next();
        assert_eq!(list.selected(), Some(3));
        list.next(); // Should wrap to 0
        assert_eq!(list.selected(), Some(0));

        // Test backward navigation
        list.previous(); // Should wrap to 3
        assert_eq!(list.selected(), Some(3));
        list.previous();
        assert_eq!(list.selected(), Some(2));
    }

    #[test]
    fn test_single_item_list() {
        let items = vec![ListItem::new("Only Item")];
        let mut list = SelectableList::new("Single Item", items);

        assert_eq!(list.selected(), Some(0));

        // Navigation should stay on the same item
        list.next();
        assert_eq!(list.selected(), Some(0));

        list.previous();
        assert_eq!(list.selected(), Some(0));
    }
}
