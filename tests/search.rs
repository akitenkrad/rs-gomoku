use std::time::Duration;

use rs_gomoku::game::{Game, Rule, Side, Stone};
use rs_gomoku::search::{Evaluation, SearchLimits, candidate_moves, evaluate_moves, patterns_after_move};

fn cell(size: usize, column: usize, row: usize) -> usize {
    row * size + column
}

fn position(black: &[usize], white: &[usize]) -> Game {
    assert!(black.len() == white.len() || black.len() == white.len() + 1);
    let mut game = Game::new(15, Rule::Freestyle, Side::Human);
    for (turn, &move_) in black.iter().enumerate() {
        game.play(move_).unwrap();
        if let Some(&reply) = white.get(turn) {
            game.play(reply).unwrap();
        }
    }
    game
}

fn short_limits() -> SearchLimits {
    SearchLimits {
        max_depth: 3,
        candidate_limit: 12,
        time_limit: Duration::from_millis(120),
        use_vcf: false,
    }
}

#[test]
fn pattern_classifier_distinguishes_the_required_shapes() {
    let size = 15;
    let row = 7;

    let open_four = position(
        &[cell(size, 7, row), cell(size, 8, row), cell(size, 9, row)],
        &[cell(size, 0, 0), cell(size, 2, 0), cell(size, 4, 0)],
    );
    let counts = patterns_after_move(&open_four, cell(size, 10, row), Stone::Black);
    assert_eq!(counts.open_four, 1);

    let closed_four = position(
        &[cell(size, 7, row), cell(size, 8, row), cell(size, 9, row)],
        &[cell(size, 6, row), cell(size, 2, 0), cell(size, 4, 0)],
    );
    let counts = patterns_after_move(&closed_four, cell(size, 10, row), Stone::Black);
    assert_eq!(counts.four, 1);
    assert_eq!(counts.open_four, 0);

    let open_three = position(&[cell(size, 7, row), cell(size, 8, row)], &[cell(size, 0, 0), cell(size, 2, 0)]);
    assert_eq!(patterns_after_move(&open_three, cell(size, 9, row), Stone::Black).open_three, 1);

    let sleeping_three = position(&[cell(size, 7, row), cell(size, 8, row)], &[cell(size, 6, row), cell(size, 2, 0)]);
    assert_eq!(patterns_after_move(&sleeping_three, cell(size, 9, row), Stone::Black).sleeping_three, 1);

    let open_two = position(&[cell(size, 7, row)], &[cell(size, 0, 0)]);
    assert_eq!(patterns_after_move(&open_two, cell(size, 8, row), Stone::Black).open_two, 1);

    let five = position(
        &[cell(size, 6, row), cell(size, 7, row), cell(size, 8, row), cell(size, 9, row)],
        &[cell(size, 0, 0), cell(size, 2, 0), cell(size, 4, 0), cell(size, 6, 0)],
    );
    assert_eq!(patterns_after_move(&five, cell(size, 10, row), Stone::Black).five, 1);
}

#[test]
fn candidates_are_central_on_empty_board_and_within_chebyshev_distance_two() {
    let mut game = Game::new(15, Rule::Freestyle, Side::Human);
    assert_eq!(candidate_moves(&game), vec![cell(15, 7, 7)]);

    game.play(cell(15, 7, 7)).unwrap();
    let candidates = candidate_moves(&game);
    assert_eq!(candidates.len(), 24);
    assert!(candidates.iter().all(|&candidate| {
        let row = candidate / 15;
        let column = candidate % 15;
        row.abs_diff(7) <= 2 && column.abs_diff(7) <= 2
    }));
}

#[test]
fn corner_candidates_and_invalid_pattern_cells_are_bounded() {
    let mut game = Game::new(7, Rule::Freestyle, Side::Human);
    // 探索は棋譜ではなく盤面を正とし，復元・分析局面にも使えること．
    game.board[0] = Some(Stone::Black);
    let candidates = candidate_moves(&game);
    assert_eq!(candidates, vec![1, 2, 7, 8, 9, 14, 15, 16]);
    assert_eq!(patterns_after_move(&game, 49, Stone::White), Default::default());
    assert_eq!(patterns_after_move(&game, 0, Stone::White), Default::default());
}

#[test]
fn standard_does_not_classify_an_overline_as_five() {
    let mut standard = Game::new(15, Rule::Standard, Side::Human);
    let mut freestyle = Game::new(15, Rule::Freestyle, Side::Human);
    for column in 0..5 {
        standard.board[cell(15, column, 7)] = Some(Stone::Black);
        freestyle.board[cell(15, column, 7)] = Some(Stone::Black);
    }
    let sixth = cell(15, 5, 7);
    assert_eq!(patterns_after_move(&standard, sixth, Stone::Black).five, 0);
    assert_eq!(patterns_after_move(&freestyle, sixth, Stone::Black).five, 1);
}

#[test]
fn search_takes_an_immediate_win_and_marks_it_as_proven() {
    let game = position(
        &[cell(15, 7, 7), cell(15, 8, 7), cell(15, 9, 7), cell(15, 10, 7)],
        &[cell(15, 0, 0), cell(15, 2, 0), cell(15, 4, 0), cell(15, 6, 0)],
    );
    let evaluations = evaluate_moves(&game, short_limits());
    assert!(
        evaluations
            .iter()
            .any(|(candidate, evaluation)| { [cell(15, 6, 7), cell(15, 11, 7)].contains(candidate) && *evaluation == Evaluation::Win(1) })
    );
}

#[test]
fn search_blocks_the_opponents_only_winning_point() {
    let game = position(
        &[cell(15, 6, 7), cell(15, 0, 0), cell(15, 2, 0), cell(15, 4, 0)],
        &[cell(15, 7, 7), cell(15, 8, 7), cell(15, 9, 7), cell(15, 10, 7)],
    );
    let evaluations = evaluate_moves(&game, short_limits());
    assert_eq!(evaluations.first().map(|entry| entry.0), Some(cell(15, 11, 7)));
}

#[test]
fn normal_depth_search_defends_against_an_open_three() {
    let game = position(
        &[cell(15, 0, 0), cell(15, 2, 0), cell(15, 4, 0)],
        &[cell(15, 7, 7), cell(15, 8, 7), cell(15, 9, 7)],
    );
    let evaluations = evaluate_moves(&game, SearchLimits::normal());
    let selected = evaluations.first().unwrap().0;
    assert!([cell(15, 6, 7), cell(15, 10, 7)].contains(&selected), "{evaluations:?}");
}

#[test]
fn vcf_proves_a_three_ply_win_from_a_double_four() {
    let center = cell(15, 7, 7);
    let game = position(
        &[
            cell(15, 5, 7),
            cell(15, 6, 7),
            cell(15, 8, 7),
            cell(15, 7, 5),
            cell(15, 7, 6),
            cell(15, 7, 8),
        ],
        &[
            cell(15, 0, 0),
            cell(15, 2, 0),
            cell(15, 4, 0),
            cell(15, 9, 0),
            cell(15, 11, 0),
            cell(15, 13, 0),
        ],
    );
    let limits = SearchLimits {
        max_depth: 4,
        candidate_limit: 16,
        time_limit: Duration::from_millis(500),
        use_vcf: true,
    };
    let evaluations = evaluate_moves(&game, limits);
    assert_eq!(evaluations.first(), Some(&(center, Evaluation::Win(3))));
}

#[test]
fn heuristic_horizon_is_never_reported_as_a_proven_win_or_loss() {
    let game = Game::new(15, Rule::Freestyle, Side::Human);
    let evaluations = evaluate_moves(&game, short_limits());
    assert_eq!(evaluations.len(), 1);
    assert!(matches!(evaluations[0].1, Evaluation::Score(_)));
}

#[test]
fn terminal_position_has_no_candidates_or_evaluations() {
    let game = position(
        &[cell(15, 6, 7), cell(15, 7, 7), cell(15, 8, 7), cell(15, 9, 7), cell(15, 10, 7)],
        &[cell(15, 0, 0), cell(15, 2, 0), cell(15, 4, 0), cell(15, 6, 0)],
    );
    assert!(candidate_moves(&game).is_empty());
    assert!(evaluate_moves(&game, short_limits()).is_empty());
}
