# animate-chars

![Demo](cham-dog.gif)

A CLI tool for animating Unicode characters and browsing the entire Unicode space. Written in Rust, ported from the original bash implementation.

## Features

- **Animate** any sequence of characters as a terminal spinner
- **Interactive browser** to explore and select Unicode characters with ratatui TUI
- **Navigate** with configurable step sizes (10, 50, 100, 500, 1000)
- **Quick jump** to popular ranges (Emoji, Box Drawing, Braille, Hiragana, Cham)
- **Save** character collections for reuse
- **Inspect** codepoints with hex and decimal values

## Installation

### From Source

```bash
cargo install --path .
```

Or build and run directly:

```bash
cargo build --release
./target/release/animate
```

## Quick Start

```bash
# Default animation
animate

# Interactive Unicode browser
animate -i

# Animate from saved file
animate -f selected_chars.txt

# Show codepoints
animate --show --range 0x1F600:10
```

## Interactive Mode

```bash
animate -i [START_ADDRESS]
```

**Controls:**
- `0-9` — Select character (allows duplicates for sequences)
- `n/p` — Next/previous by step size (default: 10)
- `j/k` — Jump ±100 codepoints
- `J/K` — Jump ±1000 codepoints
- `+/-` — Adjust step size (10 → 50 → 100 → 500 → 1000)
- `g` — Goto specific address (shows popular ranges)
- `s` — Save selection (prompts for filename)
- `q/Esc` — Quit

Always displays 10 characters per screen. Navigation is consistent and predictable.

## Options

```
Options:
      --range <START:LENGTH>    Unicode range (e.g., 0xAB20:7)
      --chars <CHARS>           Comma-separated character list
  -f, --file <FILE>             Load characters from saved file
      --speed <SPEED>           Animation speed in seconds [default: 0.1]
      --once                    Run once instead of looping
      --timer <SECONDS>         Run animation for specified duration in seconds
      --show                    Print characters with codepoints (no animation)
  -i, --interactive             Interactive Unicode browser
  <START>                       Starting address for interactive mode (e.g., 0x1F600)
  -h, --help                    Print help
```

## Examples

```bash
# Fast animation
animate --speed 0.05

# Custom spinner
animate --chars "⠋,⠙,⠹,⠸,⠼,⠴,⠦,⠧,⠇,⠏"

# Browse emoji
animate -i 0x1F600

# Timed animation
animate --timer 10 --speed 0.1

# Unicode range with single pass
animate --range 0xAB20:7 --once
```

## Technical Details

Built with:
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal manipulation
- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework

## License

MIT

## Related

- [abra-bail](https://github.com/phiat/abra-bail) - Original bash implementation
