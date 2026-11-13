use std::time::{Duration, Instant};

use rs_gomoku::ai::{self, Difficulty, Rng};
use rs_gomoku::game::{Game, Rule, Side};
use rs_gomoku::i18n::Lang;

fn cell(column: usize, row: usize) -> usize {
    row * 15 + column
}

#[test]
fn difficulty_cycles_and_has_the_specified_labels() {
    assert_eq!(Difficulty::default(), Difficulty::Normal);
    assert_eq!(Difficulty::Easy.label(Lang::Ja), "やさしい");
    assert_eq!(Difficulty::Normal.label(Lang::En), "Normal");
    assert_eq!(Difficulty::Strong.label(Lang::En), "Strong");
    assert_eq!(Difficulty::Easy.next(), Difficulty::Normal);
    assert_eq!(Difficulty::Normal.next(), Difficulty::Strong);
    assert_eq!(Difficulty::Strong.next(), Difficulty::Easy);
}

#[test]
fn easy_blocks_an_immediate_loss_about_seventy_percent_of_the_time() {
    let mut game = Game::new(15, Rule::Freestyle, Side::Human);
    for move_ in [
        cell(6, 7),
        cell(7, 7),
        cell(0, 0),
        cell(8, 7),
        cell(2, 0),
        cell(9, 7),
        cell(4, 0),
        cell(10, 7),
    ] {
        game.play(move_).unwrap();
    }
    let blocks = (1..=100)
        .filter(|&seed| ai::choose(&game, Difficulty::Easy, &mut Rng::from_seed(seed)) == Some(cell(11, 7)))
        .count();
    assert!((60..=80).contains(&blocks), "blocked {blocks} times");
}

#[test]
fn xorshift_operations_respect_their_boundaries() {
    let mut rng = Rng::from_seed(0);
    for _ in 0..100 {
        assert_eq!(rng.range(4, 4), 4);
        assert!((3..=7).contains(&rng.range(3, 7)));
        assert!((0.0..1.0).contains(&rng.unit()));
        assert!(!rng.chance(0));
        assert!(rng.chance(100));
    }
}

#[test]
fn every_difficulty_returns_only_legal_moves() {
    let mut game = Game::new(15, Rule::Freestyle, Side::Cpu);
    for move_ in [cell(7, 7), cell(6, 7), cell(8, 8), cell(5, 6)] {
        game.play(move_).unwrap();
    }
    let legal = game.legal_moves();
    for difficulty in [Difficulty::Easy, Difficulty::Normal, Difficulty::Strong] {
        let mut rng = Rng::from_seed(71);
        let selected = ai::choose(&game, difficulty, &mut rng).unwrap();
        assert!(legal.contains(&selected), "{difficulty:?}: {selected}");
    }
}

#[test]
fn no_difficulty_moves_after_the_game_is_over() {
    let mut game = Game::new(15, Rule::Freestyle, Side::Human);
    for move_ in [
        cell(6, 7),
        cell(0, 0),
        cell(7, 7),
        cell(2, 0),
        cell(8, 7),
        cell(4, 0),
        cell(9, 7),
        cell(6, 0),
        cell(10, 7),
    ] {
        game.play(move_).unwrap();
    }
    for difficulty in [Difficulty::Easy, Difficulty::Normal, Difficulty::Strong] {
        assert_eq!(ai::choose(&game, difficulty, &mut Rng::from_seed(1)), None);
    }
}

#[test]
fn strong_returns_within_its_budget_plus_two_hundred_milliseconds() {
    let game = Game::new(15, Rule::Freestyle, Side::Cpu);
    let started = Instant::now();
    let selected = ai::choose(&game, Difficulty::Strong, &mut Rng::from_seed(9));
    assert!(selected.is_some());
    assert!(started.elapsed() <= Duration::from_millis(1_200), "{:?}", started.elapsed());
}

#[test]
fn background_search_preserves_the_game_id() {
    let game = Game::new(15, Rule::Freestyle, Side::Cpu);
    let receiver = ai::spawn_choose(game, Difficulty::Easy, 17, 42);
    let result = receiver.recv_timeout(Duration::from_millis(500)).unwrap();
    assert_eq!(result.game_id, 42);
    assert!(result.cell.is_some());
}
