use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::Line;
use rs_gomoku::ai::Difficulty;
use rs_gomoku::app::{App, Mode, Particle, Screen};
use rs_gomoku::game::{Outcome, Rule, Side, Stone};
use rs_gomoku::i18n::{Lang, Msg};
use rs_gomoku::record::{Record, parse};
use rs_gomoku::ui;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn sample_record() -> Record {
    Record {
        size: 9,
        rule: Rule::Freestyle,
        first: Side::Human,
        names: [Some("Alice".into()), Some("Bob".into())],
        moves: vec![0, 9, 1, 10, 2, 11, 3, 12, 4],
    }
}

#[test]
fn replay_navigation_matches_game_at_at_every_boundary() {
    let record = sample_record();
    let mut app = App::new_replay(record.clone(), Duration::from_millis(500), true, false, Lang::En);

    app.on_key(key(KeyCode::Right));
    assert_eq!(app.game, record.game_at(1).unwrap());
    app.on_key(key(KeyCode::Char('l')));
    assert_eq!(app.game, record.game_at(2).unwrap());
    app.on_key(key(KeyCode::Left));
    assert_eq!(app.game, record.game_at(1).unwrap());
    app.on_key(key(KeyCode::Char('g')));
    assert_eq!(app.game, record.game_at(0).unwrap());
    app.on_key(key(KeyCode::End));
    assert_eq!(app.game, record.game_at(record.moves.len()).unwrap());
    app.on_key(key(KeyCode::Home));
    assert_eq!(app.game, record.game_at(0).unwrap());
}

#[test]
fn replay_move_log_follows_the_replay_position() {
    let record = sample_record();
    let mut app = App::new_replay(record.clone(), Duration::from_millis(500), true, false, Lang::En);
    assert!(app.log.is_empty());

    app.on_key(key(KeyCode::Right));
    app.on_key(key(KeyCode::Right));
    app.on_key(key(KeyCode::Right));
    assert_eq!(
        app.log,
        vec![
            (Side::Human, Stone::Black, 0),
            (Side::Cpu, Stone::White, 9),
            (Side::Human, Stone::Black, 1)
        ]
    );

    app.on_key(key(KeyCode::Left));
    assert_eq!(app.log.len(), 2);
    app.on_key(key(KeyCode::End));
    assert_eq!(app.log.len(), record.moves.len());
    app.on_key(key(KeyCode::Home));
    assert!(app.log.is_empty());
}

#[test]
fn replay_autoplay_waits_for_interval() {
    let record = sample_record();
    let mut app = App::new_replay(record, Duration::from_millis(100), false, false, Lang::En);
    app.on_tick();
    assert_eq!(app.replay().unwrap().position, 0);
    std::thread::sleep(Duration::from_millis(110));
    app.on_tick();
    assert_eq!(app.replay().unwrap().position, 1);
}

#[test]
fn saving_writes_a_parseable_record() {
    let directory = std::path::PathBuf::from("target/app-save-test");
    std::fs::create_dir_all(&directory).unwrap();
    for entry in std::fs::read_dir(&directory).unwrap() {
        std::fs::remove_file(entry.unwrap().path()).unwrap();
    }

    let mut app = App::new(9, Rule::Standard);
    app.set_record_dir(&directory);
    app.start();
    app.on_key(key(KeyCode::Enter));
    app.on_key(key(KeyCode::Char('s')));

    let files: Vec<_> = std::fs::read_dir(&directory).unwrap().collect();
    assert_eq!(files.len(), 1);
    let text = std::fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
    assert!(!text.contains("human ="), "{text}");
    assert!(!text.contains("cpu ="), "{text}");
    let record = parse(&text).unwrap();
    assert_eq!(record.size, 9);
    assert_eq!(record.rule, Rule::Standard);
}

#[test]
fn cpu_search_never_blocks_tick_and_stale_results_are_discarded() {
    let mut app = App::new(15, Rule::Freestyle);
    app.first = Side::Cpu;
    app.difficulty = Difficulty::Strong;
    app.start();

    let before = Instant::now();
    app.on_tick();
    assert!(before.elapsed() < Duration::from_millis(100));
    assert!(app.cpu_searching());

    let old_id = app.game_id();
    app.on_key(key(KeyCode::Char('n')));
    assert!(app.game_id() > old_id);
    assert!(app.game.history.is_empty());
}

fn render_buffer(app: &App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::draw(frame, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn buffer_text(buffer: &Buffer) -> String {
    let mut output = String::new();
    for y in 0..buffer.area.height {
        let mut skip = 0;
        for x in 0..buffer.area.width {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let symbol = buffer[(x, y)].symbol();
            output.push_str(symbol);
            skip = symbol
                .chars()
                .map(|character| usize::from(!character.is_ascii() && character as u32 >= 0x3000) + 1)
                .sum::<usize>()
                .saturating_sub(1);
        }
        output.push('\n');
    }
    for line in output.lines() {
        assert!(
            Line::raw(line).width() <= usize::from(buffer.area.width),
            "line exceeds {} columns: {line:?}",
            buffer.area.width
        );
    }
    output
}

fn render(app: &App, width: u16, height: u16) -> String {
    buffer_text(&render_buffer(app, width, height))
}

fn board_rect(width: u16, height: u16, size: usize) -> Rect {
    let middle = Rect::new(0, 3, width, height - 7);
    let board_area = Rect::new(middle.x, middle.y, middle.width - 34, middle.height);
    let board_width = size as u16 * 3 + 5;
    let board_height = size as u16 + 3;
    Rect::new(
        board_area.x + (board_area.width - board_width) / 2,
        board_area.y + (board_area.height - board_height) / 2,
        board_width,
        board_height,
    )
}

fn find_text_x(buffer: &Buffer, text: &str) -> Option<u16> {
    find_text_position(buffer, text).map(|(x, _)| x)
}

fn find_text_position(buffer: &Buffer, text: &str) -> Option<(u16, u16)> {
    for y in 0..buffer.area.height {
        for start_x in 0..buffer.area.width {
            let mut x = start_x;
            let mut matches = true;
            for character in text.chars() {
                if x >= buffer.area.width || buffer[(x, y)].symbol() != character.to_string() {
                    matches = false;
                    break;
                }
                x += Line::raw(character.to_string()).width() as u16;
            }
            if matches {
                return Some((start_x, y));
            }
        }
    }
    None
}

#[test]
fn game_over_result_starts_at_side_top_and_move_log_keeps_last_move() {
    for lang in [Lang::En, Lang::Ja] {
        for replay in [false, true] {
            let mut app = if replay {
                let mut app = App::new_replay(sample_record(), Duration::from_secs(1), true, false, lang);
                app.on_key(key(KeyCode::End));
                app
            } else {
                let mut app = winning_app(lang);
                app.log = vec![
                    (Side::Human, Stone::Black, 0),
                    (Side::Cpu, Stone::White, 9),
                    (Side::Human, Stone::Black, 1),
                    (Side::Cpu, Stone::White, 10),
                    (Side::Human, Stone::Black, 2),
                    (Side::Cpu, Stone::White, 11),
                    (Side::Human, Stone::Black, 3),
                    (Side::Cpu, Stone::White, 12),
                    (Side::Human, Stone::Black, 4),
                ];
                app
            };
            app.frame = 0;

            let (width, height) = ui::minimum_terminal_size(9, false);
            let buffer = render_buffer(&app, width, height);
            let screen = buffer_text(&buffer);
            let (result_x, result_y) =
                find_text_position(&buffer, Msg::Result.text(lang)).unwrap_or_else(|| panic!("result title missing:\n{screen}"));
            let (log_x, log_y) = find_text_position(&buffer, Msg::MoveLog.text(lang)).unwrap_or_else(|| panic!("move log missing:\n{screen}"));

            assert!(result_x >= width - 34, "result panel overlaps board:\n{screen}");
            assert_eq!(result_y, 3, "result panel must start at the side-column top:\n{screen}");
            assert!(log_x >= width - 34 && log_y > result_y, "move log must fill below result:\n{screen}");
            assert!(
                screen.contains(&format!("{} {} e1", app.side_name(Side::Human), Stone::Black.label(lang))),
                "last move missing from game-over log:\n{screen}"
            );
        }
    }
}

fn winning_app(lang: Lang) -> App {
    let mut app = App::new(9, Rule::Freestyle).with_lang(lang);
    app.start();
    for cell in [0, 9, 1, 10, 2, 11, 3, 12, 4] {
        app.game.play(cell).unwrap();
    }
    app.display_outcome = app.game.outcome();
    app.screen = Screen::GameOver;
    app
}

#[test]
fn adjacent_stones_keep_grid_separators_board_background_and_last_move_marker() {
    let mut app = App::new(9, Rule::Freestyle);
    app.start();
    for cell in [0, 1, 9, 18] {
        app.game.play(cell).unwrap();
    }

    let (width, height) = ui::minimum_terminal_size(9, false);
    let buffer = render_buffer(&app, width, height);
    let board = board_rect(width, height, 9);
    let origin_x = board.x + 4;
    let origin_y = board.y + 2;
    let board_bg = buffer[(origin_x + 6, origin_y)].bg;

    for cell in [0, 1, 9, 18] {
        let x = origin_x + (cell % 9) as u16 * 3;
        let y = origin_y + (cell / 9) as u16;
        assert_eq!(buffer[(x, y)].symbol(), "●", "stone {cell} was not a separate glyph");
        for side_x in x - 1..=x + 1 {
            assert_eq!(buffer[(side_x, y)].bg, board_bg, "stone {cell} painted its background");
        }
    }
    assert_eq!(buffer[(origin_x + 1, origin_y)].symbol(), "─");
    assert_eq!(buffer[(origin_x + 2, origin_y)].symbol(), "─");

    let last_x = origin_x;
    let last_y = origin_y + 2;
    assert!(
        buffer[(last_x, last_y)].modifier.contains(Modifier::UNDERLINED),
        "last move needs a visible marker"
    );
}

#[test]
fn game_over_result_is_in_side_column_and_winning_line_remains_visible() {
    for lang in [Lang::En, Lang::Ja] {
        for replay in [false, true] {
            let mut app = if replay {
                let mut app = App::new_replay(sample_record(), Duration::from_secs(1), true, false, lang);
                app.on_key(key(KeyCode::End));
                app
            } else {
                winning_app(lang)
            };
            app.frame = 0;
            let (width, height) = ui::minimum_terminal_size(9, false);
            let buffer = render_buffer(&app, width, height);
            let screen = buffer_text(&buffer);
            let side_x = width - 34;
            let result_x = find_text_x(&buffer, Msg::Result.text(lang)).unwrap_or_else(|| panic!("result title missing:\n{screen}"));
            assert!(result_x >= side_x, "result panel overlaps the board:\n{screen}");

            let board = board_rect(width, height, 9);
            let origin_x = board.x + 4;
            let origin_y = board.y + 2;
            for cell in 0..5 {
                let x = origin_x + cell as u16 * 3;
                assert_eq!(buffer[(x, origin_y)].symbol(), "●", "winning cell {cell} hidden:\n{screen}");
            }

            let expected_headline = lang.winner("Alice");
            if replay {
                assert!(screen.contains(&expected_headline), "{screen}");
                assert!(screen.contains(Msg::ReplayBackQuit.text(lang)), "{screen}");
            } else {
                assert!(screen.contains(Msg::HumanWins.text(lang)), "{screen}");
                assert!(screen.contains(Msg::NewGameSave.text(lang)), "{screen}");
            }
            assert!(screen.contains(&lang.moves_count(9)), "{screen}");
        }
    }
}

#[test]
fn replay_hides_play_only_panels_while_normal_play_keeps_them() {
    for lang in [Lang::En, Lang::Ja] {
        let (width, height) = ui::minimum_terminal_size(9, false);
        let mut normal = App::new(9, Rule::Freestyle).with_lang(lang);
        normal.start();
        let normal = render(&normal, width, height);
        assert!(normal.contains(Msg::Difficulty.text(lang)), "{normal}");
        assert!(normal.contains(Msg::SessionScore.text(lang)), "{normal}");

        let replay = App::new_replay(sample_record(), Duration::from_secs(1), true, false, lang);
        let replay = render(&replay, width, height);
        assert!(!replay.contains(Msg::Difficulty.text(lang)), "{replay}");
        assert!(!replay.contains(Msg::SessionScore.text(lang)), "{replay}");
    }
}

#[test]
fn replay_header_marks_end_finished_and_restores_playing_after_step_back() {
    for lang in [Lang::En, Lang::Ja] {
        let record = sample_record();
        let (width, height) = ui::minimum_terminal_size(record.size, false);
        let mut app = App::new_replay(record, Duration::from_secs(1), false, false, lang);
        app.on_key(key(KeyCode::End));
        let finished = render(&app, width, height);
        let finished_label = match lang {
            Lang::En => "Finished",
            Lang::Ja => "再生終了",
        };
        assert!(finished.contains(finished_label), "{finished}");
        assert!(!finished.contains(Msg::Playing.text(lang)), "{finished}");

        app.on_key(key(KeyCode::Left));
        let stepped_back = render(&app, width, height);
        assert!(stepped_back.contains(Msg::Playing.text(lang)), "{stepped_back}");
        assert!(!stepped_back.contains(finished_label), "{stepped_back}");

        let mut paused = App::new_replay(sample_record(), Duration::from_secs(1), true, false, lang);
        paused.on_key(key(KeyCode::End));
        let paused_at_end = render(&paused, width, height);
        assert!(paused_at_end.contains(finished_label), "{paused_at_end}");
        assert!(!paused_at_end.contains(Msg::Paused.text(lang)), "{paused_at_end}");
    }
}

#[test]
fn result_panel_remains_above_game_over_particles() {
    let mut app = winning_app(Lang::En);
    app.particles.push(Particle {
        x: 50.0 / 69.0,
        y: 9.0 / 24.0,
        vx: 0.0,
        vy: 0.0,
        glyph: 'X',
        color: (255, 0, 0),
    });
    let screen = render(&app, 69, 24);
    assert!(screen.contains(Msg::HumanWins.text(Lang::En)), "{screen}");
}

#[test]
fn title_playing_evaluation_and_all_results_render_for_every_layout_size() {
    for size in [9, 15, 19] {
        let (width, height) = ui::minimum_terminal_size(size, false);
        for lang in [Lang::En, Lang::Ja] {
            let mut app = App::new(size, Rule::Freestyle).with_lang(lang);
            render(&app, width, height);

            app.start();
            render(&app, width, height);
            app.show_evaluation = true;
            let (eval_width, eval_height) = ui::minimum_terminal_size(size, true);
            render(&app, eval_width, eval_height);

            app.screen = Screen::GameOver;
            app.display_outcome = Some(Outcome::Draw);
            render(&app, width, height);
            app.display_outcome = Some(Outcome::Win {
                side: Side::Human,
                cells: (0..5).collect(),
            });
            render(&app, width, height);
            app.display_outcome = Some(Outcome::Win {
                side: Side::Cpu,
                cells: (size..size + 5).collect(),
            });
            render(&app, width, height);
        }
    }
}

#[test]
fn replay_midway_finished_and_unfinished_render() {
    let record = sample_record();
    let (width, height) = ui::minimum_terminal_size(record.size, false);
    let mut midway = App::new_replay(record.clone(), Duration::from_secs(1), true, false, Lang::En);
    midway.on_key(key(KeyCode::Right));
    render(&midway, width, height);
    midway.on_key(key(KeyCode::End));
    render(&midway, width, height);

    let mut unfinished = record;
    unfinished.moves.truncate(3);
    let unfinished = App::new_replay(unfinished, Duration::from_secs(1), true, false, Lang::Ja);
    assert!(matches!(unfinished.mode, Mode::Replay(_)));
    render(&unfinished, width, height);
}

#[test]
fn replay_result_panel_uses_record_names_and_neutral_end_messages() {
    let record = sample_record();
    let (width, height) = ui::minimum_terminal_size(record.size, false);
    for lang in [Lang::En, Lang::Ja] {
        let mut complete = App::new_replay(record.clone(), Duration::from_secs(1), true, false, lang);
        complete.on_key(key(KeyCode::End));
        let complete_screen = render(&complete, width, height);
        assert!(complete_screen.contains(&lang.winner("Alice")), "{complete_screen}");
        assert!(!complete_screen.contains(Msg::HumanWins.text(lang)), "{complete_screen}");
        assert!(!complete_screen.contains(Msg::RecordFinished.text(lang)), "{complete_screen}");

        let mut draw = App::new_replay(record.clone(), Duration::from_secs(1), true, false, lang);
        draw.screen = Screen::GameOver;
        draw.display_outcome = Some(Outcome::Draw);
        draw.message = Msg::Draw.text(lang).into();
        let draw_screen = render(&draw, width, height);
        assert!(draw_screen.contains(Msg::Draw.text(lang)), "{draw_screen}");
        assert!(!draw_screen.contains(Msg::HumanWins.text(lang)), "{draw_screen}");

        let mut unfinished_record = record.clone();
        unfinished_record.moves.truncate(3);
        let mut unfinished = App::new_replay(unfinished_record, Duration::from_secs(1), true, false, lang);
        unfinished.on_key(key(KeyCode::End));
        let unfinished_screen = render(&unfinished, width, height);
        assert!(unfinished_screen.contains(Msg::EndOfRecord.text(lang)), "{unfinished_screen}");
    }
}

#[test]
fn replay_status_uses_record_players_and_avoids_human_directed_text() {
    let record = sample_record();
    let (width, height) = ui::minimum_terminal_size(record.size, false);
    for lang in [Lang::En, Lang::Ja] {
        let mut replay = App::new_replay(record.clone(), Duration::from_secs(1), true, false, lang);
        let start = render(&replay, width, height);
        replay.on_key(key(KeyCode::Right));
        let midway = render(&replay, width, height);
        replay.on_key(key(KeyCode::End));
        let end = render(&replay, width, height);

        let alice = format!("Alice {}", Stone::Black.label(lang));
        let bob = format!("Bob {}", Stone::White.label(lang));
        for screen in [&start, &midway] {
            assert!(screen.contains(&alice), "{alice:?} missing:\n{screen}");
            assert!(screen.contains(&bob), "{bob:?} missing:\n{screen}");
        }
        for screen in [&start, &midway, &end] {
            for forbidden in [
                Msg::HumanTurn.text(lang),
                Msg::CpuThinking.text(lang),
                Msg::HumanWins.text(lang),
                Msg::CpuWins.text(lang),
                Msg::RecordFinished.text(lang),
                lang.default_human_name(),
                "Congratulations",
                "おめでとう",
            ] {
                assert!(!screen.contains(forbidden), "{forbidden:?} leaked into replay:\n{screen}");
            }
        }

        let unnamed = Record {
            names: [None, None],
            ..record.clone()
        };
        let unnamed = App::new_replay(unnamed, Duration::from_secs(1), true, false, lang);
        let unnamed_screen = render(&unnamed, width, height);
        assert!(unnamed_screen.contains(lang.default_human_name()), "{unnamed_screen}");
        assert!(unnamed_screen.contains(lang.default_cpu_name()), "{unnamed_screen}");
    }
}

#[test]
fn normal_header_and_replay_header_and_help_use_their_own_labels() {
    let record = sample_record();
    let (width, height) = ui::minimum_terminal_size(record.size, false);
    for lang in [Lang::En, Lang::Ja] {
        let mut normal = App::new(record.size, record.rule).with_lang(lang);
        normal.start();
        let normal_screen = render(&normal, width, height);
        let turn = format!("{} {} {}", Msg::Turn.text(lang), lang.default_human_name(), Stone::Black.label(lang));
        assert!(normal_screen.contains(&turn), "{normal_screen}");
        assert!(normal_screen.contains(Msg::Hint.text(lang)), "{normal_screen}");

        let mut replay = App::new_replay(record.clone(), Duration::from_secs(1), true, false, lang);
        replay.on_key(key(KeyCode::Right));
        let replay_screen = render(&replay, width, height);
        let progress = lang.replay_progress(1, record.moves.len());
        assert!(replay_screen.contains(&progress), "{replay_screen}");
        assert!(
            !replay_screen.contains(&format!("{} {progress}", Msg::Turn.text(lang))),
            "replay progress was rendered as a turn value:\n{replay_screen}"
        );
        assert!(!replay_screen.contains(&format!("h: {}", Msg::Hint.text(lang))), "{replay_screen}");
        assert!(!replay_screen.contains(Msg::Hint.text(lang)), "{replay_screen}");
    }
}

#[test]
fn undersized_terminal_renders_resize_guidance() {
    let app = App::new(19, Rule::Freestyle);
    render(&app, 40, 10);
}

#[test]
#[ignore = "manual-style screen dump for visual inspection"]
fn dump_screens() {
    for size in [9, 15, 19] {
        let app = App::new(size, Rule::Freestyle);
        let (width, height) = ui::minimum_terminal_size(size, false);
        println!("--- title {size}x{size} ---\n{}", render(&app, width, height));
    }
    for lang in [Lang::En, Lang::Ja] {
        let record = sample_record();
        let (width, height) = ui::minimum_terminal_size(record.size, false);
        let mut replay = App::new_replay(record, Duration::from_secs(1), true, false, lang);
        replay.on_key(key(KeyCode::End));
        println!("--- replay end {lang:?} ---\n{}", render(&replay, width, height));
    }
}

#[test]
#[ignore = "full CPU game is intentionally slower"]
fn plays_one_game_against_cpu() {
    let mut app = App::new(9, Rule::Freestyle);
    app.difficulty = Difficulty::Easy;
    app.start();
    for _ in 0..20_000 {
        if app.screen == Screen::GameOver {
            return;
        }
        if app.human_can_act() {
            let cell = app.game.legal_moves()[0];
            app.cursor = cell;
            app.on_key(key(KeyCode::Enter));
        }
        app.on_tick();
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("game did not finish");
}
