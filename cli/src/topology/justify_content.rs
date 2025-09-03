use crossterm::terminal;
use ratatui::{
  text::{Line, Span}
};

pub fn space_between<'a>(
  left: Vec<Span<'a>>,
  right: Option<Span<'a>>
) -> Line<'a> {
  let width = terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
  space_between_with_width(left, right, width)
}

fn space_between_with_width<'a>(
  mut left: Vec<Span<'a>>,
  right: Option<Span<'a>>,
  width: usize
) -> Line<'a> {
  if let Some(right_span) = right {
    let left_len: usize = left.iter().map(|s| s.content.chars().count()).sum();
    let right_len: usize = right_span.content.chars().count();

    let min_gap = 1;
    let edge_buffer = 2;

    let total_needed = left_len + min_gap + right_len + edge_buffer;

    if width >= total_needed {
      let gap = width - left_len - right_len - edge_buffer;
      left.push(Span::raw(" ".repeat(gap)));
    } else {
      left.push(Span::raw(" "))
    }
    left.push(right_span);
  }

  Line::from(left)
}

#[cfg(test)]
mod tests {
  use super::*;
  use ratatui::text::Span;

  #[test]
  fn aligns_when_enough_space() {
    let line = space_between_with_width(vec![Span::raw("foo")], Some(Span::raw("bar")), 10);
    let result: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(result, "foo  bar");
  }

  #[test]
  fn overflows_when_not_enough_space() {
    let line = space_between_with_width(vec![Span::raw("foo")], Some(Span::raw("bar")), 5);
    let result: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    assert_eq!(result, "foo bar");
  }
}
