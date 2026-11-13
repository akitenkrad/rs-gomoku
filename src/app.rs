//! 画面遷移，入力，非同期 CPU 探索，アニメーションの状態管理．

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ai::{self, Difficulty, Rng};
use crate::game::{Game, Outcome, Rule, Side, Stone, cell_name};
use crate::i18n::{Lang, Msg};
use crate::record::{Record, to_text};
use crate::search::{self, Evaluation};

const CPU_THINK: Duration = Duration::from_millis(300);
const CPU_SHOW: Duration = Duration::from_millis(350);
pub const PLACEMENT_ANIM: Duration = Duration::from_millis(180);
const REPLAY_MIN_INTERVAL: Duration = Duration::from_millis(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Playing,
    GameOver,
}

pub enum Mode {
    Normal,
    Replay(ReplayState),
}

pub struct ReplayState {
    pub record: Record,
    pub position: usize,
    pub paused: bool,
    pub interval: Duration,
    pub exit_on_end: bool,
    next_advance_at: Instant,
}

pub struct Placement {
    pub cell: usize,
    pub stone: Stone,
    pub by: Side,
    pub started: Instant,
}

impl Placement {
    pub fn progress(&self) -> f32 {
        (self.started.elapsed().as_secs_f32() / PLACEMENT_ANIM.as_secs_f32()).min(1.0)
    }
}

pub struct CpuTurn {
    pub cell: Option<usize>,
    pub started: Instant,
    pub showing_since: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
}

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub glyph: char,
    pub color: (u8, u8, u8),
}

struct SearchReply {
    game_id: u64,
    payload: SearchPayload,
}

enum SearchPayload {
    Cpu(Option<usize>),
    Analysis {
        evaluations: Vec<(usize, Evaluation)>,
        move_cursor: bool,
    },
}

pub struct App {
    pub lang: Lang,
    pub mode: Mode,
    pub screen: Screen,
    pub game: Game,
    pub difficulty: Difficulty,
    pub first: Side,
    pub selected_rule: Rule,
    pub selected_size: usize,
    pub cursor: usize,
    pub show_evaluation: bool,
    pub evaluations: Vec<(usize, Evaluation)>,
    pub evaluation_pending: bool,
    pub hint_cells: Vec<usize>,
    pub log: Vec<(Side, Stone, usize)>,
    pub message: String,
    pub placement: Option<Placement>,
    pub cpu: Option<CpuTurn>,
    pub particles: Vec<Particle>,
    pub frame: u64,
    pub game_over_at: Option<Instant>,
    pub display_outcome: Option<Outcome>,
    pub stats: Stats,
    pub should_quit: bool,
    record_dir: PathBuf,
    rng: Rng,
    game_id: u64,
    result_tx: Sender<SearchReply>,
    result_rx: Receiver<SearchReply>,
}

impl Default for App {
    fn default() -> Self {
        Self::new(15, Rule::Freestyle)
    }
}

impl App {
    pub fn new(size: usize, rule: Rule) -> Self {
        let (result_tx, result_rx) = mpsc::channel();
        Self {
            lang: Lang::default(),
            mode: Mode::Normal,
            screen: Screen::Title,
            game: Game::new(size, rule, Side::Human),
            difficulty: Difficulty::Normal,
            first: Side::Human,
            selected_rule: rule,
            selected_size: size,
            cursor: (size / 2) * size + size / 2,
            show_evaluation: false,
            evaluations: Vec::new(),
            evaluation_pending: false,
            hint_cells: Vec::new(),
            log: Vec::new(),
            message: String::new(),
            placement: None,
            cpu: None,
            particles: Vec::new(),
            frame: 0,
            game_over_at: None,
            display_outcome: None,
            stats: Stats::default(),
            should_quit: false,
            record_dir: PathBuf::from("."),
            rng: Rng::seeded(),
            game_id: 0,
            result_tx,
            result_rx,
        }
    }

    pub fn with_lang(mut self, lang: Lang) -> Self {
        self.lang = lang;
        self
    }

    pub fn new_replay(record: Record, interval: Duration, paused: bool, exit_on_end: bool, lang: Lang) -> Self {
        let interval = interval.max(REPLAY_MIN_INTERVAL);
        let game = record.game_at(0).expect("parsed record has an initial position");
        let mut app = Self::new(record.size, record.rule).with_lang(lang);
        app.first = record.first;
        app.game = game;
        app.screen = Screen::Playing;
        app.message = Msg::RecordStart.text(lang).into();
        app.mode = Mode::Replay(ReplayState {
            record,
            position: 0,
            paused,
            interval,
            exit_on_end,
            next_advance_at: Instant::now() + interval,
        });
        app
    }

    pub fn set_record_dir(&mut self, path: impl Into<PathBuf>) {
        self.record_dir = path.into();
    }

    pub fn replay(&self) -> Option<&ReplayState> {
        match &self.mode {
            Mode::Normal => None,
            Mode::Replay(replay) => Some(replay),
        }
    }

    fn replay_mut(&mut self) -> Option<&mut ReplayState> {
        match &mut self.mode {
            Mode::Normal => None,
            Mode::Replay(replay) => Some(replay),
        }
    }

    pub fn game_id(&self) -> u64 {
        self.game_id
    }

    pub fn cpu_searching(&self) -> bool {
        self.cpu.as_ref().is_some_and(|cpu| cpu.cell.is_none())
    }

    pub fn side_name(&self, side: Side) -> &str {
        match &self.mode {
            Mode::Normal => side.label(self.lang),
            Mode::Replay(replay) => replay.record.name(side, self.lang),
        }
    }

    pub fn human_stone(&self) -> Stone {
        self.game.stone_for(Side::Human)
    }

    pub fn human_can_act(&self) -> bool {
        matches!(self.mode, Mode::Normal)
            && self.screen == Screen::Playing
            && self.game.to_move == Side::Human
            && self.placement.is_none()
            && self.cpu.is_none()
            && self.game.outcome().is_none()
    }

    pub fn start(&mut self) {
        self.invalidate_searches();
        self.mode = Mode::Normal;
        self.game = Game::new(self.selected_size, self.selected_rule, self.first);
        self.cursor = (self.selected_size / 2) * self.selected_size + self.selected_size / 2;
        self.log.clear();
        self.hint_cells.clear();
        self.evaluations.clear();
        self.evaluation_pending = false;
        self.placement = None;
        self.cpu = None;
        self.particles.clear();
        self.game_over_at = None;
        self.display_outcome = None;
        self.screen = Screen::Playing;
        self.update_message();
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c');
        if ctrl_c || key.code == KeyCode::Char('q') {
            self.should_quit = true;
            self.invalidate_searches();
            return;
        }
        if matches!(self.mode, Mode::Replay(_)) {
            if key.code == KeyCode::Esc {
                self.should_quit = true;
            } else {
                self.on_replay_key(key.code);
            }
            return;
        }
        match self.screen {
            Screen::Title => self.on_title_key(key.code),
            Screen::Playing => self.on_play_key(key.code),
            Screen::GameOver => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('n') => self.start(),
                KeyCode::Char('t') | KeyCode::Esc => self.return_to_title(),
                KeyCode::Char('s') => self.save_record(),
                _ => {}
            },
        }
    }

    fn on_title_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter | KeyCode::Char(' ') => self.start(),
            KeyCode::Char('d') => self.difficulty = self.difficulty.next(),
            KeyCode::Char('f') => self.first = self.first.other(),
            KeyCode::Char('l') => self.lang = self.lang.toggle(),
            KeyCode::Char('r') => {
                self.selected_rule = match self.selected_rule {
                    Rule::Freestyle => Rule::Standard,
                    Rule::Standard => Rule::Freestyle,
                };
            }
            KeyCode::Char('z') => {
                const SIZES: [usize; 5] = [9, 11, 13, 15, 19];
                let index = SIZES.iter().position(|&size| size == self.selected_size).unwrap_or(2);
                self.selected_size = SIZES[(index + 1) % SIZES.len()];
            }
            _ => {}
        }
    }

    fn on_play_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('e') => {
                self.show_evaluation = !self.show_evaluation;
                if self.show_evaluation {
                    self.request_analysis(false);
                }
            }
            KeyCode::Char('h') if self.human_can_act() => self.request_analysis(true),
            KeyCode::Char('s') => self.save_record(),
            KeyCode::Char('n') => self.start(),
            KeyCode::Char('t') | KeyCode::Esc => self.return_to_title(),
            KeyCode::Left if self.human_can_act() => self.move_cursor(-1, 0),
            KeyCode::Right if self.human_can_act() => self.move_cursor(1, 0),
            KeyCode::Up if self.human_can_act() => self.move_cursor(0, -1),
            KeyCode::Down if self.human_can_act() => self.move_cursor(0, 1),
            KeyCode::Enter | KeyCode::Char(' ') if self.human_can_act() => self.perform(self.cursor),
            _ => {}
        }
    }

    fn return_to_title(&mut self) {
        self.invalidate_searches();
        self.screen = Screen::Title;
        self.cpu = None;
        self.placement = None;
    }

    fn move_cursor(&mut self, dx: isize, dy: isize) {
        let size = self.game.size as isize;
        let column = (self.cursor % self.game.size) as isize;
        let row = (self.cursor / self.game.size) as isize;
        let column = (column + dx).rem_euclid(size);
        let row = (row + dy).rem_euclid(size);
        self.cursor = (row * size + column) as usize;
        self.hint_cells.clear();
    }

    fn perform(&mut self, cell: usize) {
        let by = self.game.to_move;
        let stone = self.game.stone_for(by);
        if self.game.play(cell).is_err() {
            self.message = Msg::CellUnavailable.text(self.lang).into();
            return;
        }
        self.log.push((by, stone, cell));
        self.cursor = cell;
        self.hint_cells.clear();
        self.evaluations.clear();
        self.cpu = None;
        self.placement = Some(Placement {
            cell,
            stone,
            by,
            started: Instant::now(),
        });
        self.message = self.lang.placed(self.side_name(by), &cell_name(cell, self.game.size));
    }

    fn update_message(&mut self) {
        self.message = match self.game.to_move {
            Side::Human => Msg::HumanTurn.text(self.lang).into(),
            Side::Cpu => Msg::CpuThinking.text(self.lang).into(),
        };
    }

    pub fn on_tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.update_particles();
        self.receive_search_results();

        if self.screen == Screen::GameOver
            && self.replay().is_some_and(|replay| replay.exit_on_end)
            && self.game.outcome().is_some()
            && self.game_over_at.is_some_and(|at| at.elapsed() >= Duration::from_secs(3))
        {
            self.should_quit = true;
        }
        if self.screen != Screen::Playing {
            return;
        }
        if matches!(self.mode, Mode::Replay(_)) {
            self.on_replay_tick();
            return;
        }
        if let Some(placement) = &self.placement {
            if placement.started.elapsed() >= PLACEMENT_ANIM {
                self.placement = None;
                if self.game.outcome().is_some() {
                    self.finish_normal();
                } else {
                    self.update_message();
                }
            }
            return;
        }
        if self.game.to_move != Side::Cpu || self.game.is_over() {
            return;
        }
        if self.cpu.is_none() {
            self.spawn_cpu_search();
            return;
        }
        let Some(cpu) = &mut self.cpu else { return };
        if let Some(cell) = cpu.cell {
            if cpu.started.elapsed() < CPU_THINK {
                return;
            }
            if cpu.showing_since.is_none() {
                cpu.showing_since = Some(Instant::now());
                self.cursor = cell;
                self.message = self.lang.cpu_selected(&cell_name(cell, self.game.size));
                return;
            }
            if cpu.showing_since.is_some_and(|at| at.elapsed() >= CPU_SHOW) {
                self.perform(cell);
            }
        }
    }

    fn spawn_cpu_search(&mut self) {
        let game = self.game.clone();
        let difficulty = self.difficulty;
        let game_id = self.game_id;
        let tx = self.result_tx.clone();
        let seed = self.rng.next_u64();
        self.cpu = Some(CpuTurn {
            cell: None,
            started: Instant::now(),
            showing_since: None,
        });
        std::thread::spawn(move || {
            let mut rng = Rng::from_seed(seed);
            let cell = ai::choose(&game, difficulty, &mut rng);
            let _ = tx.send(SearchReply {
                game_id,
                payload: SearchPayload::Cpu(cell),
            });
        });
    }

    fn receive_search_results(&mut self) {
        while let Ok(reply) = self.result_rx.try_recv() {
            if reply.game_id != self.game_id {
                continue;
            }
            match reply.payload {
                SearchPayload::Cpu(cell) => {
                    if let Some(cpu) = &mut self.cpu {
                        cpu.cell = cell;
                    }
                }
                SearchPayload::Analysis { evaluations, move_cursor } => {
                    self.evaluation_pending = false;
                    self.evaluations = evaluations;
                    if move_cursor && let Some(&(cell, _)) = self.evaluations.first() {
                        self.hint_cells = vec![cell];
                        self.cursor = cell;
                    }
                }
            }
        }
    }

    fn request_analysis(&mut self, move_cursor: bool) {
        if self.game.is_over() {
            return;
        }
        self.evaluation_pending = true;
        let game = self.game.clone();
        let difficulty = self.difficulty;
        let game_id = self.game_id;
        let tx = self.result_tx.clone();
        std::thread::spawn(move || {
            let evaluations = search::evaluate_moves(&game, difficulty.search_limits());
            let _ = tx.send(SearchReply {
                game_id,
                payload: SearchPayload::Analysis { evaluations, move_cursor },
            });
        });
        if !move_cursor {
            self.hint_cells.clear();
        }
    }

    fn invalidate_searches(&mut self) {
        self.game_id = self.game_id.wrapping_add(1);
        self.evaluation_pending = false;
    }

    fn finish_normal(&mut self) {
        if self.screen == Screen::GameOver {
            return;
        }
        self.display_outcome = self.game.outcome();
        match &self.display_outcome {
            Some(Outcome::Win { side: Side::Human, .. }) => {
                self.stats.wins += 1;
                self.message = Msg::HumanWins.text(self.lang).into();
            }
            Some(Outcome::Win { side: Side::Cpu, .. }) => {
                self.stats.losses += 1;
                self.message = Msg::CpuWins.text(self.lang).into();
            }
            Some(Outcome::Draw) => {
                self.stats.draws += 1;
                self.message = Msg::Draw.text(self.lang).into();
            }
            None => return,
        }
        self.screen = Screen::GameOver;
        self.game_over_at = Some(Instant::now());
        self.spawn_result_particles();
    }

    fn spawn_result_particles(&mut self) {
        let win = matches!(self.display_outcome, Some(Outcome::Win { side: Side::Human, .. }));
        let loss = matches!(self.display_outcome, Some(Outcome::Win { side: Side::Cpu, .. }));
        for index in 0..if win {
            120
        } else if loss {
            80
        } else {
            30
        } {
            let random = self.rng.unit();
            self.particles.push(Particle {
                x: self.rng.unit(),
                y: if loss { random } else { -random * 0.2 },
                vx: (self.rng.unit() - 0.5) * 0.008,
                vy: if loss {
                    0.008 + self.rng.unit() * 0.012
                } else {
                    0.005 + self.rng.unit() * 0.02
                },
                glyph: if loss { '│' } else { ['◆', '●', '▲', '■'][index % 4] },
                color: if loss { (80, 140, 210) } else { (120 + (index % 100) as u8, 90, 180) },
            });
        }
    }

    fn update_particles(&mut self) {
        for particle in &mut self.particles {
            particle.x += particle.vx;
            particle.y += particle.vy;
            if particle.glyph != '│' {
                particle.vy += 0.0005;
            }
        }
        self.particles
            .retain(|particle| particle.y < 1.1 && particle.x > -0.1 && particle.x < 1.1);
    }

    fn save_record(&mut self) {
        let record = Record {
            size: self.game.size,
            rule: self.game.rule,
            first: self.game.first,
            names: [None, None],
            moves: self.game.history.iter().map(|&(_, cell)| cell).collect(),
        };
        match save_record_file(&self.record_dir, &record) {
            Ok(path) => self.message = self.lang.record_saved(&path.display().to_string()),
            Err(error) => self.message = self.lang.record_save_failed(error),
        }
    }

    fn on_replay_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(' ') => {
                if let Some(replay) = self.replay_mut() {
                    replay.paused = !replay.paused;
                    replay.next_advance_at = Instant::now() + replay.interval;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => self.set_replay_position_by(1),
            KeyCode::Left | KeyCode::Char('h') => self.set_replay_position_by(-1),
            KeyCode::Home | KeyCode::Char('g') => self.set_replay_position(0),
            KeyCode::End | KeyCode::Char('G') => {
                if let Some(length) = self.replay().map(|replay| replay.record.moves.len()) {
                    self.set_replay_position(length);
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => self.change_replay_interval(-250),
            KeyCode::Char('-') => self.change_replay_interval(250),
            KeyCode::Char('e') => {
                self.show_evaluation = !self.show_evaluation;
                if self.show_evaluation {
                    self.request_analysis(false);
                }
            }
            _ => {}
        }
    }

    fn on_replay_tick(&mut self) {
        if self
            .placement
            .as_ref()
            .is_some_and(|placement| placement.started.elapsed() >= PLACEMENT_ANIM)
        {
            self.placement = None;
        }
        let should_advance = self
            .replay()
            .is_some_and(|replay| !replay.paused && replay.position < replay.record.moves.len() && Instant::now() >= replay.next_advance_at);
        if should_advance && self.placement.is_none() {
            self.set_replay_position_by(1);
        }
    }

    fn set_replay_position_by(&mut self, delta: isize) {
        let Some(replay) = self.replay() else { return };
        let next = (replay.position as isize + delta).clamp(0, replay.record.moves.len() as isize) as usize;
        self.set_replay_position(next);
    }

    fn set_replay_position(&mut self, position: usize) {
        let Some((game, previous, interval, length)) = self.replay().and_then(|replay| {
            replay
                .record
                .game_at(position)
                .map(|game| (game, replay.position, replay.interval, replay.record.moves.len()))
        }) else {
            return;
        };
        self.game = game;
        self.cursor = self.game.history.last().map_or(self.cursor, |&(_, cell)| cell);
        self.placement = if position > previous {
            self.game.history.last().map(|&(by, cell)| Placement {
                cell,
                stone: self.game.stone_for(by),
                by,
                started: Instant::now(),
            })
        } else {
            None
        };
        if let Some(replay) = self.replay_mut() {
            replay.position = position;
            replay.next_advance_at = Instant::now() + interval;
        }
        if position == length {
            self.display_outcome = self.game.outcome();
            self.screen = Screen::GameOver;
            self.game_over_at = Some(Instant::now());
            self.message = match self.display_outcome.as_ref() {
                Some(Outcome::Win { side, .. }) => self.lang.winner(self.side_name(*side)),
                Some(Outcome::Draw) => Msg::Draw.text(self.lang).into(),
                None => Msg::EndOfRecord.text(self.lang).into(),
            };
        } else {
            self.screen = Screen::Playing;
            self.display_outcome = None;
            self.message = self.lang.replay_progress(position, length);
        }
    }

    fn change_replay_interval(&mut self, delta_ms: i64) {
        if let Some(replay) = self.replay_mut() {
            let current = replay.interval.as_millis() as i64;
            replay.interval = Duration::from_millis((current + delta_ms).max(30) as u64);
            replay.next_advance_at = Instant::now() + replay.interval;
        }
    }
}

fn save_record_file(directory: &Path, record: &Record) -> Result<PathBuf, String> {
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| error.to_string())?.as_secs();
    let path = directory.join(format!("gomoku-{seconds}.gmkrec"));
    std::fs::write(&path, to_text(record)).map_err(|error| error.to_string())?;
    Ok(path)
}
