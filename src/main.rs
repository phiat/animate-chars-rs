use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style,
    terminal::{self, ClearType},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

const DEFAULT_CHARS: &[char] = &['ꬠ', 'ꬡ', 'ꬢ', 'ꬣ', 'ꬤ', 'ꬥ', 'ꬦ'];
const DEFAULT_START: u32 = 0xAB20;
const MAX_UNICODE: u32 = 0x10FFFF;
const DISPLAY_SIZE: usize = 10;

#[derive(Parser, Debug)]
#[command(name = "animate-chars")]
#[command(about = "Animate Unicode characters in the terminal", long_about = None)]
struct Args {
    /// Unicode range (e.g., 0xAB20:7)
    #[arg(long, value_name = "START:LENGTH")]
    range: Option<String>,

    /// Comma-separated character list
    #[arg(long, value_name = "CHARS")]
    chars: Option<String>,

    /// Load characters from saved file
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

    /// Animation speed in seconds
    #[arg(long, default_value = "0.1")]
    speed: f64,

    /// Run once instead of looping
    #[arg(long)]
    once: bool,

    /// Run animation for specified duration in seconds
    #[arg(long, value_name = "SECONDS")]
    timer: Option<u64>,

    /// Print characters with codepoints (no animation)
    #[arg(long)]
    show: bool,

    /// Interactive Unicode browser
    #[arg(short, long)]
    interactive: bool,

    /// Starting address for interactive mode (e.g., 0x1F600)
    #[arg(value_name = "START")]
    start: Option<String>,
}

struct InteractiveState {
    position: u32,
    step_size: u32,
    selected: Vec<char>,
}

impl InteractiveState {
    fn new(start: u32) -> Self {
        Self {
            position: start,
            step_size: 10,
            selected: Vec::new(),
        }
    }

    fn get_page_chars(&self) -> Vec<(u32, char)> {
        (self.position..self.position + DISPLAY_SIZE as u32)
            .take_while(|&cp| cp <= MAX_UNICODE)
            .filter_map(|cp| char::from_u32(cp).map(|c| (cp, c)))
            .collect()
    }

    fn next_page(&mut self) {
        self.position = (self.position + self.step_size).min(MAX_UNICODE);
    }

    fn prev_page(&mut self) {
        self.position = self.position.saturating_sub(self.step_size);
    }

    fn jump_forward(&mut self, amount: u32) {
        self.position = (self.position + amount).min(MAX_UNICODE);
    }

    fn jump_back(&mut self, amount: u32) {
        self.position = self.position.saturating_sub(amount);
    }

    fn increase_step(&mut self) {
        self.step_size = match self.step_size {
            10 => 50,
            50 => 100,
            100 => 500,
            500 => 1000,
            _ => self.step_size,
        };
    }

    fn decrease_step(&mut self) {
        self.step_size = match self.step_size {
            1000 => 500,
            500 => 100,
            100 => 50,
            50 => 10,
            _ => self.step_size,
        };
    }

    fn goto(&mut self, address: u32) {
        self.position = address.min(MAX_UNICODE);
    }
}

fn parse_address(s: &str) -> Result<u32, String> {
    if let Some(stripped) = s.strip_prefix("0x") {
        u32::from_str_radix(stripped, 16)
            .map_err(|e| format!("Invalid hex address: {}", e))
    } else if s.chars().all(|c| c.is_ascii_digit()) {
        s.parse::<u32>()
            .map_err(|e| format!("Invalid decimal address: {}", e))
    } else {
        u32::from_str_radix(s, 16)
            .map_err(|e| format!("Invalid address: {}", e))
    }
}

fn parse_range(range_str: &str) -> Result<Vec<char>, String> {
    let parts: Vec<&str> = range_str.split(':').collect();
    if parts.len() != 2 {
        return Err("Range format should be START:LENGTH".to_string());
    }

    let start = parse_address(parts[0])?;
    let length: u32 = parts[1]
        .parse()
        .map_err(|e| format!("Invalid length: {}", e))?;

    Ok((start..start + length)
        .filter_map(char::from_u32)
        .collect())
}

fn parse_chars(chars_str: &str) -> Vec<char> {
    chars_str.split(',').filter_map(|s| s.chars().next()).collect()
}

fn load_chars_from_file(path: &PathBuf) -> Result<Vec<char>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let last_line = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .last()
        .ok_or_else(|| "No character data found in file".to_string())?;

    Ok(parse_chars(last_line))
}

fn save_chars_to_file(chars: &[char], path: &PathBuf) -> io::Result<()> {
    let char_list: String = chars.iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let content = format!(
        "# Selected Unicode characters - generated by animate-chars\n\
         # Usage: animate --chars \"{}\"\n\
         \n\
         {}\n",
        char_list, char_list
    );
    fs::write(path, content)
}

fn animate_once(chars: &[char], speed: Duration) -> io::Result<()> {
    let mut stdout = io::stdout();
    for ch in chars {
        queue!(stdout, cursor::MoveToColumn(0), style::Print(format!("{} ", ch)))?;
        stdout.flush()?;
        thread::sleep(speed);
    }
    Ok(())
}

fn run_animation(chars: &[char], speed: f64, once: bool, timer: Option<u64>) -> io::Result<()> {
    let duration = Duration::from_secs_f64(speed);
    let mut stdout = io::stdout();

    execute!(stdout, cursor::Hide)?;

    let result = if once {
        animate_once(chars, duration)
    } else if let Some(secs) = timer {
        let start = Instant::now();
        let total_duration = Duration::from_secs(secs);
        while start.elapsed() < total_duration {
            animate_once(chars, duration)?;
        }
        Ok(())
    } else {
        loop {
            animate_once(chars, duration)?;
        }
    };

    execute!(stdout, cursor::Show, style::Print("\n"))?;
    result
}

fn show_chars(chars: &[char]) {
    for ch in chars {
        let codepoint = *ch as u32;
        println!("U+{:X} ({}) {}", codepoint, codepoint, ch);
    }
}

fn draw_interactive(
    frame: &mut Frame,
    state: &InteractiveState,
) {
    let area = frame.area();
    let page_chars = state.get_page_chars();

    // Calculate layout
    let chunks = Layout::vertical([
        Constraint::Length(4), // Header
        Constraint::Min(10),    // Character list
        Constraint::Length(3),  // Help text
    ])
    .split(area);

    // Header
    let selected_text = if state.selected.is_empty() {
        "(none)".to_string()
    } else {
        state.selected.len().to_string()
    };

    let end_pos = state.position + DISPLAY_SIZE as u32 - 1;
    let header_text = vec![
        Line::from(vec![
            Span::raw("Range: "),
            Span::styled(
                format!("0x{:05X}-0x{:05X}", state.position, end_pos),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw(format!("Step: {}  ", state.step_size)),
            Span::raw("                    Selected: "),
            Span::styled(selected_text, Style::default().fg(Color::Green)),
        ]),
    ];

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title("Unicode Browser"));
    frame.render_widget(header, chunks[0]);

    // Character list
    let char_lines: Vec<Line> = page_chars
        .iter()
        .enumerate()
        .map(|(i, (codepoint, ch))| {
            Line::from(vec![
                Span::styled(format!("[{}] ", i), Style::default().fg(Color::Yellow)),
                Span::raw(format!("U+{:06X} ", codepoint)),
                Span::styled(format!("({:6}) ", codepoint), Style::default().fg(Color::Gray)),
                Span::styled(ch.to_string(), Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let char_list = Paragraph::new(char_lines);
    frame.render_widget(char_list, chunks[1]);

    // Help text
    let help = Paragraph::new(vec![
        Line::from("0-9:select | n:next p:prev | j:+100 k:-100 | J:+1000 K:-1000"),
        Line::from("+/-:step size | g:goto | s:save q:quit"),
    ])
    .alignment(Alignment::Left);
    frame.render_widget(help, chunks[2]);
}

fn handle_goto(state: &mut InteractiveState) -> io::Result<()> {
    let mut stdout = io::stdout();

    execute!(
        stdout,
        cursor::Show,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    println!("Popular ranges:");
    println!("  0x2500   │  Box Drawing (borders, lines)");
    println!("  0x2800   ⠀  Braille Patterns");
    println!("  0x3040   ぀  Hiragana (Japanese)");
    println!("  0xAA00   ꨀ  Cham");
    println!("  0x1F600  😀  Emoji (faces, hands)");
    println!();
    print!("Goto address: ");
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if let Ok(addr) = parse_address(input.trim()) {
        state.goto(addr);
    }

    execute!(stdout, cursor::Hide)?;
    Ok(())
}

fn handle_save(state: &InteractiveState) -> io::Result<bool> {
    if state.selected.is_empty() {
        println!("No characters selected.");
        return Ok(false);
    }

    let mut stdout = io::stdout();
    execute!(stdout, cursor::Show)?;

    print!("Save as (default: selected_chars.txt): ");
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let filename = input.trim();

    let path = if filename.is_empty() {
        PathBuf::from("selected_chars.txt")
    } else if !filename.contains('.') {
        PathBuf::from(format!("{}.txt", filename))
    } else {
        PathBuf::from(filename)
    };

    save_chars_to_file(&state.selected, &path)?;
    println!("Saved {} characters to: {:?}", state.selected.len(), path);

    Ok(true)
}

fn run_interactive(start: u32) -> io::Result<()> {
    let mut state = InteractiveState::new(start);

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = loop {
        terminal.draw(|f| draw_interactive(f, &state))?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Char(c @ '0'..='9') => {
                    let idx = (c as u8 - b'0') as usize;
                    let page = state.get_page_chars();
                    if idx < page.len() {
                        state.selected.push(page[idx].1);
                    }
                }
                KeyCode::Char('n') => state.next_page(),
                KeyCode::Char('p') => state.prev_page(),
                KeyCode::Char('j') => state.jump_forward(100),
                KeyCode::Char('k') => state.jump_back(100),
                KeyCode::Char('J') => state.jump_forward(1000),
                KeyCode::Char('K') => state.jump_back(1000),
                KeyCode::Char('+') => state.increase_step(),
                KeyCode::Char('-') => state.decrease_step(),
                KeyCode::Char('g') => {
                    terminal::disable_raw_mode()?;
                    execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen)?;
                    handle_goto(&mut state)?;
                    execute!(terminal.backend_mut(), terminal::EnterAlternateScreen)?;
                    terminal::enable_raw_mode()?;
                }
                KeyCode::Char('s') => {
                    terminal::disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        terminal::LeaveAlternateScreen,
                        cursor::Show
                    )?;
                    let saved = handle_save(&state)?;
                    if saved {
                        break Ok(());
                    }
                    execute!(terminal.backend_mut(), terminal::EnterAlternateScreen)?;
                    terminal::enable_raw_mode()?;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    break Ok(());
                }
                _ => {}
            }
        }
    };

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        terminal::LeaveAlternateScreen,
        cursor::Show
    )?;

    result
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Determine characters to use
    let chars = if let Some(ref range_str) = args.range {
        parse_range(range_str).unwrap_or_else(|e| {
            eprintln!("Error parsing range: {}", e);
            std::process::exit(1);
        })
    } else if let Some(ref chars_str) = args.chars {
        parse_chars(chars_str)
    } else if let Some(ref path) = args.file {
        load_chars_from_file(path).unwrap_or_else(|e| {
            eprintln!("Error loading file: {}", e);
            std::process::exit(1);
        })
    } else if !args.interactive {
        DEFAULT_CHARS.to_vec()
    } else {
        vec![]
    };

    // Handle interactive mode
    if args.interactive {
        let start = if let Some(ref start_str) = args.start {
            parse_address(start_str).unwrap_or_else(|e| {
                eprintln!("Error parsing start address: {}", e);
                std::process::exit(1);
            })
        } else {
            DEFAULT_START
        };
        return run_interactive(start);
    }

    // Handle show mode
    if args.show {
        show_chars(&chars);
        return Ok(());
    }

    // Run animation
    let result = run_animation(&chars, args.speed, args.once, args.timer);

    // Handle Ctrl+C gracefully
    if let Err(ref e) = result {
        if e.kind() == io::ErrorKind::Interrupted {
            return Ok(());
        }
    }

    result
}
