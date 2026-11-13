use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rs_gomoku::app::{App, Screen};
use rs_gomoku::game::{Rule, Side};
use rs_gomoku::i18n::Lang;
use rs_gomoku::record::Record;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn returning_from_replay_end_reconstructs_a_playable_midway_screen() {
    let record = Record {
        size: 9,
        rule: Rule::Freestyle,
        first: Side::Human,
        names: [Some("A".into()), Some("B".into())],
        moves: vec![0, 9, 1, 10, 2, 11, 3, 12, 4],
    };
    let mut app = App::new_replay(record.clone(), Duration::from_secs(1), true, false, Lang::En);
    app.on_key(key(KeyCode::End));
    assert_eq!(app.screen, Screen::GameOver);
    app.on_key(key(KeyCode::Left));
    assert_eq!(app.screen, Screen::Playing);
    assert_eq!(app.game, record.game_at(record.moves.len() - 1).unwrap());
}

#[test]
fn repeated_new_game_actions_advance_identity_and_reset_transient_state() {
    let mut app = App::new(15, Rule::Freestyle);
    app.first = Side::Cpu;
    app.start();
    app.on_tick();
    let first_id = app.game_id();
    app.on_key(key(KeyCode::Char('n')));
    let second_id = app.game_id();
    app.on_key(key(KeyCode::Esc));
    assert!(second_id > first_id);
    assert!(app.game_id() > second_id);
    assert_eq!(app.screen, Screen::Title);
    assert!(app.game.history.is_empty());
}
