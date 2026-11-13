**English** | [日本語](strategy.ja.md)

# Strategy and search

## Shapes that matter

Gomoku is a game of threats. The shapes below are what both players watch for:

| Shape | Meaning |
|---|---|
| Five | Five in a row — the game is over. |
| Open four | Four in a row with both ends empty. It cannot be stopped: the opponent can block only one end. |
| Four | Four stones that need one more point to make five. The opponent **must** block it. |
| Open three | Three that can become an open four next move. It must usually be answered. |
| Closed three, two, … | Weaker shapes that build towards the ones above. |

The classic winning pattern is the **four-three**: a single move that makes a four and an open three at the same time. The opponent has to block the four, and the three then becomes an open four.

The sample record [`examples/records/black-wins-15.gmkrec`](../examples/records/black-wins-15.gmkrec) shows one. Black's 11th move, `i8`, makes the four `f8 g8 h8 i8` and the open three `i6 i7 i8` together. White blocks the four at `j8`, and Black extends the three to five.

It is known that the first player can force a win on a 15 × 15 board under freestyle rules (Allis, 1994), which is why tournament rules restrict Black. rs-gomoku does not implement those restrictions.

## How the CPU thinks

The search lives in `src/search.rs`.

1. **Candidates.** Only empty points within distance 2 of an existing stone are considered (the centre on an empty board). They are ordered by a quick pattern score.
2. **Pattern evaluation.** For each line through a point, the length of the run and whether its ends are open are classified into the shapes above. Attack and defence are both scored, so a move that blocks the opponent's four ranks high.
3. **Alpha-beta search.** Negamax with alpha-beta pruning and iterative deepening over the best candidates, until a depth or time limit is reached.
4. **VCF search** (Strong only). *Victory by continuous fours*: a search that plays only fours, so the opponent's reply is always forced. When it finds a sequence that ends in five, the move is a proven win.

| Difficulty | Depth | Candidates | Time | VCF |
|---|---|---|---|---|
| Easy | 1 | 8 | 30 ms | — |
| Normal | 4 | 12 | 300 ms | — |
| Strong | 6 | 16 | 1000 ms | ✓ |

On every level the CPU takes an immediate five; Easy adds randomness and sometimes misses a block.

A result is shown as *Win n* or *Loss n* only when it has been proved — by the VCF search or by reaching the end of the game inside the search. Otherwise the panel shows the heuristic score.
