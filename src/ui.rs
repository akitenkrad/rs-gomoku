//! ratatui による五目盤，状態パネル，アニメーションの描画．

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
};

use crate::app::{App, Screen};
use crate::game::{Outcome, Side, Stone, cell_name};
use crate::i18n::{Lang, Msg};
use crate::search::Evaluation;

type Rgb = (u8, u8, u8);

const BG: Rgb = (15, 17, 24);
const BOARD_BG: Rgb = (61, 50, 39);
const GRID: Rgb = (151, 126, 91);
const DIM: Rgb = (120, 124, 142);
const TEXT: Rgb = (229, 229, 238);
const ACCENT: Rgb = (120, 190, 245);
const PURPLE: Rgb = (146, 132, 245);
const TEAL: Rgb = (56, 190, 152);
const DANGER: Rgb = (245, 92, 111);

fn c(rgb: Rgb) -> Color {
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

fn fg(rgb: Rgb) -> Style {
    Style::new().fg(c(rgb))
}

fn shade(rgb: Rgb, amount: i16) -> Rgb {
    let shift = |value: u8| (i16::from(value) + amount).clamp(0, 255) as u8;
    (shift(rgb.0), shift(rgb.1), shift(rgb.2))
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(fg(DIM))
        .title(Span::styled(title, fg(TEXT).add_modifier(Modifier::BOLD)))
}

fn key(label: &str) -> Span<'static> {
    Span::styled(format!(" {label} "), Style::new().fg(c(BG)).bg(c(ACCENT)).add_modifier(Modifier::BOLD))
}

/// 盤サイズごとの描画に必要な最小端末サイズを返す．
pub fn minimum_terminal_size(size: usize, evaluation: bool) -> (u16, u16) {
    let content_height = size as u16 + 12;
    let panel_height = if evaluation { 27 } else { 24 };
    ((size as u16) * 3 + 42, content_height.max(panel_height))
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Block::new().style(Style::new().bg(c(BG)).fg(c(TEXT))), area);
    let (minimum_width, minimum_height) = minimum_terminal_size(app.game.size, app.show_evaluation);
    if area.width < minimum_width || area.height < minimum_height {
        let message = app.lang.resize(minimum_width, minimum_height, area.width, area.height);
        frame.render_widget(Paragraph::new(message).alignment(Alignment::Center), centered(area, area.width, 1));
        return;
    }

    match app.screen {
        Screen::Title => draw_title(frame, app, area),
        Screen::Playing | Screen::GameOver => draw_game(frame, app, area),
    }
    draw_particles(frame.buffer_mut(), app, area);
    if app.screen == Screen::GameOver {
        draw_result(frame, app, area);
    }
}

fn draw_title(frame: &mut Frame, app: &App, area: Rect) {
    let logo = vec![
        Line::styled("● ○ ● ○ ●", fg(TEAL).add_modifier(Modifier::BOLD)),
        Line::styled("rs-gomoku", fg(ACCENT).add_modifier(Modifier::BOLD)),
        Line::styled(Msg::TitleSubtitle.text(app.lang), fg(DIM)),
    ];
    let logo_area = Rect::new(area.x, area.y, area.width, 9.min(area.height));
    frame.render_widget(Paragraph::new(logo).alignment(Alignment::Center), centered(logo_area, 70, 5));

    let first = app.lang.first_player(app.first.label(app.lang), Stone::Black.label(app.lang));
    let rows = vec![
        setting_row("Enter", Msg::StartGame.text(app.lang), ""),
        setting_row("d", Msg::Difficulty.text(app.lang), app.difficulty.label(app.lang)),
        setting_row("f", Msg::First.text(app.lang), &first),
        setting_row("r", Msg::Rule.text(app.lang), app.selected_rule.label(app.lang)),
        setting_row("z", Msg::BoardSize.text(app.lang), &app.lang.board_size(app.selected_size)),
        setting_row("l", Msg::Language.text(app.lang), app.lang.label()),
        setting_row("q", Msg::Quit.text(app.lang), ""),
    ];
    let menu = Rect::new(area.x + area.width.saturating_sub(58) / 2, area.y + 8, 58, 11);
    frame.render_widget(
        Paragraph::new(rows).block(panel(&format!(" {} ", Msg::Settings.text(app.lang))).padding(ratatui::widgets::Padding::vertical(1))),
        menu,
    );
    if (app.frame / 18).is_multiple_of(2) {
        frame.render_widget(
            Paragraph::new(Span::styled(Msg::StartPrompt.text(app.lang), fg(ACCENT).add_modifier(Modifier::BOLD))).alignment(Alignment::Center),
            Rect::new(area.x, menu.bottom() + 1, area.width, 1),
        );
    }
}

fn setting_row(button: &str, label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        key(button),
        Span::styled(format!("  {label:<12}"), fg(DIM)),
        Span::styled(value.to_string(), fg(TEXT).add_modifier(Modifier::BOLD)),
    ])
}

fn draw_game(frame: &mut Frame, app: &App, area: Rect) {
    let [status, middle, help] = Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(4)]).areas(area);
    draw_status(frame, app, status);
    let [board, side] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(34)]).areas(middle);
    draw_board(frame, app, board);
    draw_side(frame, app, side);
    draw_help(frame, app, help);
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![Span::styled(" GOMOKU ", Style::new().fg(c(BG)).bg(c(TEAL)).add_modifier(Modifier::BOLD))];
    if let Some(replay) = app.replay() {
        spans.extend([
            Span::styled(
                format!(
                    "  {}",
                    if replay.paused {
                        Msg::Paused.text(app.lang)
                    } else {
                        Msg::Playing.text(app.lang)
                    }
                ),
                fg(ACCENT),
            ),
            Span::styled(
                format!("  {}", app.lang.replay_progress(replay.position, replay.record.moves.len())),
                fg(TEXT),
            ),
            Span::styled(format!("  {}ms", replay.interval.as_millis()), fg(DIM)),
        ]);
    } else {
        let stone = app.game.stone_for(app.game.to_move);
        spans.extend([
            Span::styled(format!("  {} ", Msg::Turn.text(app.lang)), fg(DIM)),
            Span::styled(
                format!("{} {}", app.side_name(app.game.to_move), stone.label(app.lang)),
                fg(stone_color(stone)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", app.lang.board_rule(app.game.size, app.game.rule.label(app.lang))),
                fg(TEXT),
            ),
            Span::styled(format!("  {}", app.lang.moves_count(app.game.history.len())), fg(DIM)),
        ]);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).block(panel("")), area);
}

fn draw_board(frame: &mut Frame, app: &App, area: Rect) {
    let board_width = app.game.size as u16 * 3 + 5;
    let board_height = app.game.size as u16 + 3;
    let board = centered(area, board_width, board_height);
    frame.render_widget(
        panel(&format!(" {} ", Msg::Board.text(app.lang))).style(Style::new().bg(c(BOARD_BG))),
        board,
    );
    let buffer = frame.buffer_mut();
    let origin_x = board.x + 4;
    let origin_y = board.y + 2;

    for column in 0..app.game.size {
        let label = char::from(b'a' + column as u8).to_string();
        buffer.set_string(origin_x + column as u16 * 3, board.y + 1, label, fg(DIM));
    }
    for row in 0..app.game.size {
        buffer.set_string(board.x + 1, origin_y + row as u16, format!("{:>2}", row + 1), fg(DIM));
        for column in 0..app.game.size {
            let x = origin_x + column as u16 * 3;
            let y = origin_y + row as u16;
            let grid = match (row == 0, row + 1 == app.game.size, column == 0, column + 1 == app.game.size) {
                (true, _, true, _) => '┌',
                (true, _, _, true) => '┐',
                (_, true, true, _) => '└',
                (_, true, _, true) => '┘',
                (true, _, _, _) => '┬',
                (_, true, _, _) => '┴',
                (_, _, true, _) => '├',
                (_, _, _, true) => '┤',
                _ => '┼',
            };
            buffer[(x, y)].set_char(grid).set_fg(c(GRID)).set_bg(c(BOARD_BG));
            if column + 1 < app.game.size {
                buffer[(x + 1, y)].set_char('─').set_fg(c(GRID)).set_bg(c(BOARD_BG));
                buffer[(x + 2, y)].set_char('─').set_fg(c(GRID)).set_bg(c(BOARD_BG));
            }
        }
    }

    let outcome = app.display_outcome.clone().or_else(|| app.game.outcome());
    let winning = match outcome.as_ref() {
        Some(Outcome::Win { cells, .. }) => Some(cells.as_slice()),
        _ => None,
    };
    for cell in 0..app.game.board.len() {
        let x = origin_x + (cell % app.game.size) as u16 * 3;
        let y = origin_y + (cell / app.game.size) as u16;
        let selected = cell == app.cursor && (app.human_can_act() || app.cpu.is_some());
        let hinted = app.hint_cells.contains(&cell);
        let flashing = winning.is_some_and(|cells| cells.contains(&cell)) && (app.frame / 8).is_multiple_of(2);
        if let Some(stone) = app.game.board[cell] {
            let progress = app
                .placement
                .as_ref()
                .filter(|placement| placement.cell == cell)
                .map_or(1.0, crate::app::Placement::progress);
            draw_stone(buffer, x, y, stone, progress, flashing || selected);
        } else if selected || hinted {
            let color = if hinted { TEAL } else { ACCENT };
            buffer[(x, y)].set_char('◇').set_fg(c(color)).set_bg(c(BOARD_BG));
        } else if app.show_evaluation
            && let Some((_, evaluation)) = app.evaluations.iter().find(|(candidate, _)| *candidate == cell)
        {
            buffer[(x, y)]
                .set_char('◆')
                .set_fg(c(evaluation_color(*evaluation)))
                .set_bg(c(shade(BOARD_BG, 14)));
        }
    }
}

fn draw_stone(buffer: &mut Buffer, x: u16, y: u16, stone: Stone, progress: f32, emphasized: bool) {
    if progress < 0.25 {
        buffer[(x, y)].set_char('·').set_fg(c(stone_color(stone)));
        return;
    }
    let base = stone_color(stone);
    let edge = if emphasized { ACCENT } else { shade(base, -55) };
    buffer[(x, y)].set_char(' ').set_bg(c(shade(base, 28)));
    if x > 0 {
        buffer[(x - 1, y)].set_char(' ').set_bg(c(base));
    }
    buffer[(x + 1, y)].set_char(' ').set_bg(c(edge));
}

fn stone_color(stone: Stone) -> Rgb {
    match stone {
        Stone::Black => (66, 70, 80),
        Stone::White => (224, 218, 198),
    }
}

fn evaluation_color(evaluation: Evaluation) -> Rgb {
    match evaluation {
        Evaluation::Win(_) => TEAL,
        Evaluation::Loss(_) => DANGER,
        Evaluation::Score(score) if score >= 0 => shade(TEAL, -20),
        Evaluation::Score(_) => shade(DANGER, -20),
    }
}

fn evaluation_text(evaluation: Evaluation, lang: Lang) -> String {
    match evaluation {
        Evaluation::Win(turns) => lang.evaluation(Msg::EvalWin, turns),
        Evaluation::Loss(turns) => lang.evaluation(Msg::EvalLoss, turns),
        Evaluation::Score(score) if score >= 0 => format!("+{score}"),
        Evaluation::Score(score) => score.to_string(),
    }
}

fn draw_side(frame: &mut Frame, app: &App, area: Rect) {
    let [state, evaluation, stats, log] = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(if app.show_evaluation { 8 } else { 0 }),
        Constraint::Length(5),
        Constraint::Fill(1),
    ])
    .areas(area);
    let state_lines = if app.replay().is_some() {
        let human_stone = app.game.stone_for(Side::Human);
        let cpu_stone = app.game.stone_for(Side::Cpu);
        vec![
            Line::from(vec![
                Span::styled(format!(" {} ", Msg::Rule.text(app.lang)), fg(DIM)),
                Span::styled(app.game.rule.label(app.lang), fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled(format!(" {} ", Msg::Board.text(app.lang)), fg(DIM)),
                Span::styled(app.lang.board_size(app.game.size), fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled(format!(" {} ", app.side_name(Side::Human)), fg(DIM)),
                Span::styled(human_stone.label(app.lang), fg(stone_color(human_stone))),
            ]),
            Line::from(vec![
                Span::styled(format!(" {} ", app.side_name(Side::Cpu)), fg(DIM)),
                Span::styled(cpu_stone.label(app.lang), fg(stone_color(cpu_stone))),
            ]),
            Line::styled(format!(" {}", app.message), fg(ACCENT)),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled(format!(" {} ", Msg::Difficulty.text(app.lang)), fg(DIM)),
                Span::styled(app.difficulty.label(app.lang), fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled(format!(" {} ", Msg::Rule.text(app.lang)), fg(DIM)),
                Span::styled(app.game.rule.label(app.lang), fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled(format!(" {} ", Msg::Board.text(app.lang)), fg(DIM)),
                Span::styled(app.lang.board_size(app.game.size), fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled(format!(" {} ", app.lang.default_human_name()), fg(DIM)),
                Span::styled(app.human_stone().label(app.lang), fg(stone_color(app.human_stone()))),
            ]),
            Line::styled(format!(" {}", app.message), fg(ACCENT)),
        ]
    };
    frame.render_widget(
        Paragraph::new(state_lines).block(panel(&format!(" {} ", Msg::Status.text(app.lang)))),
        state,
    );

    if app.show_evaluation {
        let lines = if app.evaluation_pending {
            vec![Line::styled(format!(" {}", Msg::Calculating.text(app.lang)), fg(ACCENT))]
        } else {
            app.evaluations
                .iter()
                .take(5)
                .enumerate()
                .map(|(index, &(cell, value))| {
                    Line::from(vec![
                        Span::styled(format!(" {}. ", index + 1), fg(DIM)),
                        Span::styled(format!("{:<4}", cell_name(cell, app.game.size)), fg(TEXT)),
                        Span::styled(evaluation_text(value, app.lang), fg(evaluation_color(value))),
                    ])
                })
                .collect()
        };
        frame.render_widget(
            Paragraph::new(lines).block(panel(&format!(" {} ", Msg::TopMoves.text(app.lang)))),
            evaluation,
        );
    }

    frame.render_widget(
        Paragraph::new(vec![
            Line::raw(""),
            Line::styled(
                app.lang.stats(app.stats.wins, app.stats.losses, app.stats.draws),
                fg(TEXT).add_modifier(Modifier::BOLD),
            ),
        ])
        .block(panel(&format!(" {} ", Msg::SessionScore.text(app.lang)))),
        stats,
    );

    let rows = log.height.saturating_sub(2) as usize;
    let start = app.log.len().saturating_sub(rows);
    let lines = app.log[start..]
        .iter()
        .enumerate()
        .map(|(offset, &(side, stone, cell))| {
            Line::from(vec![
                Span::styled(format!(" {:>2}. ", start + offset + 1), fg(DIM)),
                Span::styled(format!("{} {} ", app.side_name(side), stone.label(app.lang)), fg(stone_color(stone))),
                Span::styled(cell_name(cell, app.game.size), fg(TEXT)),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines).block(panel(&format!(" {} ", Msg::MoveLog.text(app.lang)))), log);
}

fn draw_help(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if app.replay().is_some() {
        vec![
            help_line(
                &[("Space", Msg::PlayPause), ("←→", Msg::PreviousNext), ("Home/End", Msg::BeginningEnd)],
                app.lang,
            ),
            help_line(&[("+/-", Msg::Speed), ("e", Msg::Evaluation), ("q", Msg::Quit)], app.lang),
        ]
    } else if app.screen == Screen::GameOver {
        vec![help_line(
            &[("Enter", Msg::New), ("s", Msg::Save), ("t", Msg::Title), ("q", Msg::Quit)],
            app.lang,
        )]
    } else {
        vec![
            help_line(
                &[
                    (Msg::ArrowKeys.text(app.lang), Msg::Select),
                    ("Enter", Msg::Place),
                    ("h", Msg::Hint),
                    ("e", Msg::Evaluation),
                ],
                app.lang,
            ),
            help_line(&[("s", Msg::Save), ("n", Msg::New), ("t", Msg::Title), ("q", Msg::Quit)], app.lang),
        ]
    };

    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center).block(panel("")), area);
}

fn help_line(items: &[(&str, Msg)], lang: Lang) -> Line<'static> {
    let mut spans = Vec::new();
    for &(button, message) in items {
        spans.push(key(button));
        spans.push(Span::styled(format!(" {} ", message.text(lang)), fg(DIM)));
    }
    Line::from(spans)
}

fn draw_particles(buffer: &mut Buffer, app: &App, area: Rect) {
    for particle in &app.particles {
        if !(0.0..1.0).contains(&particle.x) || !(0.0..1.0).contains(&particle.y) {
            continue;
        }
        let x = area.x + (particle.x * f32::from(area.width)) as u16;
        let y = area.y + (particle.y * f32::from(area.height)) as u16;
        if x < area.right() && y < area.bottom() {
            buffer[(x, y)].set_char(particle.glyph).set_fg(c(particle.color));
        }
    }
}

fn draw_result(frame: &mut Frame, app: &App, area: Rect) {
    let rect = centered(area, 50, 9);
    let outcome = app.display_outcome.clone().or_else(|| app.game.outcome());
    let color = match outcome.as_ref() {
        Some(Outcome::Win { side: Side::Human, .. }) => TEAL,
        Some(Outcome::Win { side: Side::Cpu, .. }) => DANGER,
        Some(Outcome::Draw) => PURPLE,
        None => ACCENT,
    };
    let headline = if app.replay().is_some() {
        app.message.clone()
    } else {
        match outcome.as_ref() {
            Some(Outcome::Win { side: Side::Human, .. }) => Msg::HumanWins.text(app.lang),
            Some(Outcome::Win { side: Side::Cpu, .. }) => Msg::CpuWins.text(app.lang),
            Some(Outcome::Draw) => Msg::Draw.text(app.lang),
            None => Msg::EndOfRecord.text(app.lang),
        }
        .into()
    };
    let pulse = if (app.frame / 8).is_multiple_of(2) { color } else { shade(color, -65) };
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(vec![
            Line::raw(""),
            Line::styled(headline, fg(color).add_modifier(Modifier::BOLD)),
            Line::raw(""),
            Line::styled(app.lang.moves_count(app.game.history.len()), fg(TEXT)),
            Line::raw(""),
            Line::styled(
                if app.replay().is_some() {
                    Msg::ReplayBackQuit.text(app.lang)
                } else {
                    Msg::NewGameSave.text(app.lang)
                },
                fg(DIM),
            ),
        ])
        .alignment(Alignment::Center)
        .block(panel(&format!(" {} ", Msg::Result.text(app.lang))).border_style(fg(pulse))),
        rect,
    );
}
