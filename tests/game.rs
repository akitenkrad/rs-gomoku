use rs_gomoku::game::{Game, MoveError, Outcome, Rule, Side, Stone, cell_name, parse_cell};

fn cell(size: usize, col: usize, row: usize) -> usize {
    row * size + col
}

#[test]
fn board_size_boundaries_and_defaults_are_exact() {
    assert!(Game::try_new(6, Rule::Freestyle, Side::Human).is_err());
    assert_eq!(Game::try_new(7, Rule::Freestyle, Side::Human).unwrap().size, 7);
    assert_eq!(Game::try_new(19, Rule::Standard, Side::Cpu).unwrap().size, 19);
    assert!(Game::try_new(20, Rule::Freestyle, Side::Human).is_err());

    let game = Game::default();
    assert_eq!(game.size, 15);
    assert_eq!(game.rule, Rule::Freestyle);
    assert_eq!(game.first, Side::Human);
    assert_eq!(game.stone_for(Side::Human), Stone::Black);
    assert_eq!(game.stone_for(Side::Cpu), Stone::White);
}

#[test]
fn coordinates_include_i_and_round_trip_on_every_supported_size() {
    for size in 7..=19 {
        for index in 0..size * size {
            assert_eq!(parse_cell(&cell_name(index, size), size), Some(index));
        }
    }
    assert_eq!(parse_cell("A1", 15), Some(0));
    assert_eq!(parse_cell("h8", 15), Some(112));
    assert_eq!(parse_cell("I8", 15), Some(113));
    assert_eq!(parse_cell("s19", 19), Some(360));
    for invalid in ["", "a0", "a01", "a20", "t1", "aa1", "1a", " h8", "h8 "] {
        assert_eq!(parse_cell(invalid, 19), None, "{invalid:?}");
    }
    assert_eq!(cell_name(225, 15), "?");
}

#[test]
fn detects_five_in_four_directions_at_edges_and_corners() {
    let size = 15;
    let lines = [
        [cell(size, 0, 0), cell(size, 1, 0), cell(size, 2, 0), cell(size, 3, 0), cell(size, 4, 0)],
        [
            cell(size, 14, 10),
            cell(size, 14, 11),
            cell(size, 14, 12),
            cell(size, 14, 13),
            cell(size, 14, 14),
        ],
        [cell(size, 0, 0), cell(size, 1, 1), cell(size, 2, 2), cell(size, 3, 3), cell(size, 4, 4)],
        [cell(size, 4, 0), cell(size, 3, 1), cell(size, 2, 2), cell(size, 1, 3), cell(size, 0, 4)],
    ];
    for line in lines {
        let mut game = Game::new(size, Rule::Freestyle, Side::Human);
        for (turn, &black) in line.iter().enumerate() {
            game.play(black).unwrap();
            if turn < 4 {
                game.play(cell(size, 7 + turn, 14)).unwrap();
            }
        }
        let Some(Outcome::Win { side, mut cells }) = game.outcome() else {
            panic!("expected a win for {line:?}");
        };
        let mut expected = line.to_vec();
        cells.sort_unstable();
        expected.sort_unstable();
        assert_eq!(side, Side::Human);
        assert_eq!(cells, expected);
    }
}

fn make_line(rule: Rule, length: usize) -> Game {
    let size = 15;
    let mut game = Game::new(size, rule, Side::Human);
    let order: Vec<_> = if length == 6 { vec![0, 1, 2, 3, 5, 4] } else { (0..length).collect() };
    for (turn, col) in order.into_iter().enumerate() {
        game.play(cell(size, col, 7)).unwrap();
        if turn + 1 < length {
            game.play(cell(size, turn * 2, 14)).unwrap();
        }
    }
    game
}

#[test]
fn freestyle_accepts_overline_but_standard_only_exactly_five() {
    let freestyle = make_line(Rule::Freestyle, 6);
    assert!(matches!(freestyle.outcome(), Some(Outcome::Win { cells, .. }) if cells.len() == 6));

    let mut standard = make_line(Rule::Standard, 6);
    assert_eq!(standard.outcome(), None);
    let next = cell(15, 14, 13);
    assert_eq!(standard.play(next), Ok(()), "overline must not end a standard game");

    for rule in [Rule::Freestyle, Rule::Standard] {
        assert!(matches!(make_line(rule, 5).outcome(), Some(Outcome::Win { cells, .. }) if cells.len() == 5));
    }
}

#[test]
fn seven_by_seven_can_end_in_a_draw() {
    let size = 7;
    let mut game = Game::new(size, Rule::Freestyle, Side::Human);
    let mut black = Vec::new();
    let mut white = Vec::new();
    for row in 0..size {
        for col in 0..size {
            if (row + col / 2) % 2 == 0 {
                black.push(cell(size, col, row));
            } else {
                white.push(cell(size, col, row));
            }
        }
    }
    for turn in 0..white.len() {
        game.play(black[turn]).unwrap();
        game.play(white[turn]).unwrap();
    }
    game.play(*black.last().unwrap()).unwrap();
    assert_eq!(game.outcome(), Some(Outcome::Draw));
}

#[test]
fn illegal_moves_do_not_mutate_the_game() {
    let mut game = Game::new(15, Rule::Freestyle, Side::Cpu);
    assert_eq!(game.play(225), Err(MoveError::OutOfRange));
    assert_eq!(game.history.len(), 0);

    game.play(112).unwrap();
    let after_first = game.clone();
    assert_eq!(game.play(112), Err(MoveError::Occupied));
    assert_eq!(game, after_first);

    let mut finished = make_line(Rule::Freestyle, 5);
    let snapshot = finished.clone();
    assert_eq!(finished.play(200), Err(MoveError::GameOver));
    assert_eq!(finished, snapshot);
}

fn naive_outcome(game: &Game) -> Option<Outcome> {
    let directions = [(1isize, 0isize), (0, 1), (1, 1), (1, -1)];
    for row in 0..game.size {
        for col in 0..game.size {
            let start = row * game.size + col;
            let Some(stone) = game.board[start] else { continue };
            for (dx, dy) in directions {
                let previous_col = col as isize - dx;
                let previous_row = row as isize - dy;
                if previous_col >= 0
                    && previous_col < game.size as isize
                    && previous_row >= 0
                    && previous_row < game.size as isize
                    && game.board[previous_row as usize * game.size + previous_col as usize] == Some(stone)
                {
                    continue;
                }
                let mut cells = Vec::new();
                let mut x = col as isize;
                let mut y = row as isize;
                while x >= 0 && x < game.size as isize && y >= 0 && y < game.size as isize {
                    let index = y as usize * game.size + x as usize;
                    if game.board[index] != Some(stone) {
                        break;
                    }
                    cells.push(index);
                    x += dx;
                    y += dy;
                }
                let wins = match game.rule {
                    Rule::Freestyle => cells.len() >= 5,
                    Rule::Standard => cells.len() == 5,
                };
                if wins {
                    return Some(Outcome::Win {
                        side: game.side_for(stone),
                        cells,
                    });
                }
            }
        }
    }
    game.board.iter().all(Option::is_some).then_some(Outcome::Draw)
}

#[test]
fn optimized_outcome_matches_naive_scan_for_500_random_games() {
    let mut random = 0x7a91_d283_4c51_019bu64;
    for game_index in 0..500 {
        let size = 7 + game_index % 13;
        let rule = if game_index % 2 == 0 { Rule::Freestyle } else { Rule::Standard };
        let mut game = Game::new(size, rule, Side::Human);
        while game.outcome().is_none() {
            let legal = game.legal_moves();
            if legal.is_empty() {
                break;
            }
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            let chosen = legal[random as usize % legal.len()];
            game.play(chosen).unwrap();
            assert_eq!(game.outcome(), naive_outcome(&game), "game {game_index}");
        }
    }
}
