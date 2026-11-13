use rs_gomoku::game::{Game, MoveError, Rule, Side, cell_name, parse_cell};
use rs_gomoku::record::parse;

#[test]
fn extreme_indices_and_numeric_tokens_never_panic_or_wrap() {
    let mut game = Game::new(19, Rule::Freestyle, Side::Human);
    let snapshot = game.clone();
    assert_eq!(game.play(usize::MAX), Err(MoveError::OutOfRange));
    assert_eq!(game, snapshot);
    assert_eq!(cell_name(usize::MAX, 19), "?");
    assert_eq!(parse_cell("a18446744073709551615", 19), None);
}

#[test]
fn parser_reports_the_real_coordinate_on_a_small_board() {
    let error = parse("size = 9\ni9\nI9\n").unwrap_err();
    assert_eq!(error.to_string(), "line 3: cell i9 is already occupied");
}
