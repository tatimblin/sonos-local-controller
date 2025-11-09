use std::fs::File;
use std::io::{ BufRead, BufReader, Lines, Result };
use std::path::Path;
use ratatui::text::{ Line, Text };

pub fn logo() -> Text<'static> {
  let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/assets/logo.txt");
  if let Ok(lines) = read_lines(path) {
    let text_lines: Vec<Line> = lines
      .filter_map(|line| line.ok())
      .map(|s| Line::raw(s))
      .collect();

    Text::from(text_lines)
  } else {
    Text::from("SONOS")
  }
}

fn read_lines<P>(filename: P) -> Result<Lines<BufReader<File>>>
where P: AsRef<Path>, {
  let file = File::open(filename)?;
  Ok(BufReader::new(file).lines())
}
