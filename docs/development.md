**English** | [日本語](development.ja.md)

# Development

## Building

```sh
cargo build --release
cargo run --release                      # play
cargo run --release -- replay FILE       # replay a record
```

rs-gomoku uses Rust edition 2024, [ratatui](https://ratatui.rs) for drawing and [clap](https://docs.rs/clap) for the command line. Its structure follows [rs-nim](https://github.com/akitenkrad/rs-nim) and [rs-tic-tac-toe](https://github.com/akitenkrad/rs-tic-tac-toe).

## Source layout

| File | Role |
|---|---|
| `src/main.rs` | Command-line parsing, the event loop (33 ms tick) and `replay --print` |
| `src/lib.rs` | Exposes the modules below as a library for the integration tests |
| `src/game.rs` | Board of any size from 7 to 19, rules (freestyle / standard), coordinates and win detection. Independent of the UI |
| `src/search.rs` | Pattern evaluation, candidate generation, alpha-beta search with iterative deepening, and VCF search |
| `src/ai.rs` | Difficulty levels, CPU move selection on a background thread, and a small xorshift RNG |
| `src/record.rs` | Reading and writing `.gmkrec` records, and rebuilding the position after any number of moves |
| `src/i18n.rs` | English and Japanese text for everything the user sees |
| `src/app.rs` | Screens, input, normal and replay modes, background analysis, animations, session tally, confetti and rain |
| `src/ui.rs` | Drawing, and the terminal size needed for a given board size |

The CPU, the hint and the evaluation panel run their searches on a background thread. Each request carries the current game's ID, so a result that arrives after you start a new game or go back to the title is discarded.

## Tests

```sh
cargo test                              # unit and integration tests
cargo test -- --ignored --nocapture     # text dump of the screens and a full game against the CPU
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The integration tests live in `tests/`:

| File | What it checks |
|---|---|
| `game.rs` | Fives in all four directions, at edges and corners, overlines under both rules, draws, illegal moves, and the win check against a plain implementation over random games |
| `search.rs` | Shape classification, taking a five, blocking the opponent's five, answering an open three, finding a VCF, and never reporting an unproved win or loss |
| `ai.rs` | Every difficulty plays only legal moves, and Strong answers within its time limit |
| `record.rs`, `record_i18n.rs` | Parsing, every error kind with its line number, the round trip and every sample record; default names per language |
| `app_ui.rs`, `app_i18n.rs`, `i18n.rs` | Replay navigation, saving, background thinking, and rendering every screen in both languages at 9, 15 and 19 at the minimum size |
| `cli.rs` | `replay --print` output and errors |
| `regression/` | Boundary cases, state transitions and background-thread timing found while testing |
