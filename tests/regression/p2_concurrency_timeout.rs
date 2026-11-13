use std::time::{Duration, Instant};

use rs_gomoku::ai::{self, Difficulty};
use rs_gomoku::game::{Game, Rule, Side};
use rs_gomoku::search::{Evaluation, SearchLimits, evaluate_moves};

#[test]
fn expired_heuristic_search_does_not_claim_an_unproved_mate() {
    let mut game = Game::new(19, Rule::Freestyle, Side::Human);
    for cell in [180, 181, 161, 200, 162, 201] {
        game.play(cell).unwrap();
    }
    let values = evaluate_moves(
        &game,
        SearchLimits {
            max_depth: 8,
            candidate_limit: 20,
            time_limit: Duration::ZERO,
            use_vcf: false,
        },
    );
    assert!(!values.is_empty());
    assert!(values.iter().all(|(_, value)| matches!(value, Evaluation::Score(_))));
}

#[test]
fn abandoned_background_search_does_not_block_the_caller() {
    let game = Game::new(19, Rule::Freestyle, Side::Cpu);
    let start = Instant::now();
    let receiver = ai::spawn_choose(game, Difficulty::Strong, 7, 99);
    drop(receiver);
    assert!(start.elapsed() < Duration::from_millis(50));
}
