**English** | [日本語](gameplay.ja.md)

# Playing rs-gomoku

## Rules

Two players take turns placing a stone on an intersection of an N × N board. The first player plays **Black**, the second plays **White**. The first to line up five stones horizontally, vertically or diagonally wins. If the board fills up first, the game is a draw.

| Rule | Who wins |
|---|---|
| Freestyle (default) | Five **or more** in a row wins. |
| Standard | **Exactly** five in a row wins; a line of six or more (an *overline*) does not count, and the game goes on. |

Renju restrictions (forbidden moves for Black) are not implemented.

## Board size and coordinates

The board can be anything from 7 × 7 to 19 × 19 (default 15 × 15).

- Choose it on the title screen with `z` (9, 11, 13, 15, 19), or start with `--size <N>` for any size from 7 to 19.
- A point is named by a column letter (`a`, `b`, … from the left) and a row number (`1`, `2`, … from the **top**), e.g. `h8` is the centre of a 15 × 15 board. Every letter is used, including `i`.

## Starting a game

Run `rs-gomoku` (or `cargo run --release`) to open the title screen.

| Key | Setting | Values |
|---|---|---|
| `d` | Difficulty | Easy / Normal / Strong |
| `f` | First player (plays Black) | You / CPU |
| `r` | Rule | Freestyle / Standard |
| `z` | Board size | 9 / 11 / 13 / 15 / 19 |
| `l` | Language | English / 日本語 |
| `Enter` / `Space` | Start | |
| `q` | Quit | |

The same settings are available as options:

```sh
rs-gomoku --size 19 --rule standard --lang ja
```

## Controls during a game

| Key | Action |
|---|---|
| `←` `↑` `↓` `→` | Move the cursor |
| `Enter` / `Space` | Place a stone at the cursor |
| `h` | Hint: highlight the move the search likes best |
| `e` | Toggle the evaluation panel |
| `s` | Save the current game as a record file (see [replay](replay.md)) |
| `n` | New game |
| `t` / `Esc` | Back to the title screen |
| `q` / `Ctrl-C` | Quit |

The last move is highlighted and new stones fade in. The CPU searches in a background thread, so the screen keeps animating while it thinks, and it briefly highlights its chosen point before playing. When someone wins, the winning line flashes; winning showers the screen with confetti and losing makes it rain. The side panel keeps a win / loss / draw tally for the session.

## The evaluation panel

Press `e` to analyse the position for the player to move. Candidate points are tinted on the board like a heat map, and the side panel lists the five best moves:

- **Win n** / **Loss n** — a forced result the search has *proved*, ending the game after `n` moves counting this one.
- **+120**, **−40**, … — a heuristic score when nothing is proved; higher is better for the player to move.

Gomoku is too large to solve here, so most moves show a score rather than a proven result. See [strategy](strategy.md) for how the score is computed.

## Language

The interface is in English by default. Switch to Japanese with `l` on the title screen, or start the game with `--lang ja` (this also works for `replay`).

## Terminal requirements

The required size grows with the board: a width of `3N + 42` cells and a height of `N + 12` (at least 24, or 27 with the evaluation panel open).

| Board | Terminal |
|---|---|
| 9 × 9 | 69 × 24 (69 × 27 with the panel) |
| 15 × 15 | 87 × 27 |
| 19 × 19 | 99 × 31 |

A smaller terminal shows a message asking you to enlarge it. A true-colour (24-bit) terminal is recommended. If box-drawing characters look misaligned in a CJK locale, set your terminal's *East Asian Ambiguous width* to 1 (narrow).
