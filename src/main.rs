use std::fmt::Write as _;
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, ValueEnum};
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use rs_gomoku::app::App;
use rs_gomoku::game::{Game, Outcome, Rule, Stone};
use rs_gomoku::i18n::{Lang, Msg};
use rs_gomoku::record::Record;
use rs_gomoku::{record, ui};

const TICK: Duration = Duration::from_millis(33);
const DEFAULT_REPLAY_INTERVAL_MS: u64 = 1_500;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum RuleArg {
    Freestyle,
    Standard,
}

impl From<RuleArg> for Rule {
    fn from(value: RuleArg) -> Self {
        match value {
            RuleArg::Freestyle => Self::Freestyle,
            RuleArg::Standard => Self::Standard,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "rs-gomoku")]
struct Cli {
    /// Interface language.
    #[arg(long, default_value = "en", global = true, value_parser = parse_lang)]
    lang: Lang,

    /// Initial board size (7 through 19).
    #[arg(long)]
    size: Option<usize>,

    /// Initial rule set.
    #[arg(long, value_enum)]
    rule: Option<RuleArg>,

    /// Directory used when saving records from a normal game.
    #[arg(long)]
    record_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Replay a .gmkrec game record.
    Replay {
        /// Record file, or - to read from standard input.
        file: PathBuf,

        /// Automatic playback interval in milliseconds.
        #[arg(long, default_value_t = DEFAULT_REPLAY_INTERVAL_MS)]
        interval: u64,

        /// Start playback paused.
        #[arg(long)]
        paused: bool,

        /// Exit three seconds after reaching a finished record's end.
        #[arg(long)]
        exit_on_end: bool,

        /// Print moves and the final board instead of starting the TUI.
        #[arg(long)]
        print: bool,
    },
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rs-gomoku: {error}");
            ExitCode::from(1)
        }
    }
}

fn execute(cli: Cli) -> Result<(), String> {
    match cli.command {
        None => {
            let size = cli.size.unwrap_or(15);
            let rule = cli.rule.unwrap_or(RuleArg::Freestyle).into();
            Game::try_new(size, rule, rs_gomoku::game::Side::Human).map_err(|error| error.to_string())?;
            let mut app = App::new(size, rule).with_lang(cli.lang);
            app.set_record_dir(cli.record_dir.unwrap_or_else(|| PathBuf::from(".")));
            run_tui(app).map_err(|error| error.to_string())
        }
        Some(Command::Replay {
            file,
            interval,
            paused,
            exit_on_end,
            print,
        }) => {
            if cli.size.is_some() || cli.rule.is_some() || cli.record_dir.is_some() {
                return Err("--size, --rule, and --record-dir cannot be used with replay".into());
            }
            let text = read_record_text(&file)?;
            let record = record::parse(&text).map_err(|error| format!("{}: {error}", file.display()))?;
            if print {
                io::stdout()
                    .write_all(format_replay(&record, cli.lang).as_bytes())
                    .map_err(|error| format!("failed to write replay: {error}"))?;
                return Ok(());
            }
            run_tui(App::new_replay(record, Duration::from_millis(interval), paused, exit_on_end, cli.lang)).map_err(|error| error.to_string())
        }
    }
}

fn read_record_text(path: &Path) -> Result<String, String> {
    if path.as_os_str() == "-" {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("failed to read stdin: {error}"))?;
        Ok(text)
    } else {
        std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
    }
}

fn parse_lang(value: &str) -> Result<Lang, String> {
    Lang::from_str(value).ok_or_else(|| "language must be 'en' or 'ja'".into())
}

fn format_replay(record: &Record, lang: Lang) -> String {
    let mut output = String::new();
    let mut game = Game::new(record.size, record.rule, record.first);
    for (index, &cell) in record.moves.iter().enumerate() {
        let side = game.to_move;
        let stone = game.stone_for(side);
        game.play(cell).expect("parsed record moves are legal");
        let line = lang.move_line(
            index + 1,
            record.name(side, lang),
            stone.label(lang),
            &rs_gomoku::game::cell_name(cell, record.size),
        );
        let _ = writeln!(output, "{line}");
    }
    output.push('\n');
    output.push_str(&format_board(&game));
    match game.outcome() {
        Some(Outcome::Win { side, .. }) => {
            let _ = writeln!(output, "{}", lang.result_winner(record.name(side, lang)));
        }
        Some(Outcome::Draw) => {
            output.push_str(Msg::ResultDraw.text(lang));
            output.push('\n');
        }
        None => {
            output.push_str(Msg::ResultUnfinished.text(lang));
            output.push('\n');
        }
    }
    output
}

fn format_board(game: &Game) -> String {
    let mut output = String::with_capacity((game.size + 1) * game.size);
    for row in 0..game.size {
        for column in 0..game.size {
            output.push(match game.board[row * game.size + column] {
                Some(Stone::Black) => 'X',
                Some(Stone::White) => 'O',
                None => '.',
            });
            if column + 1 < game.size {
                output.push(' ');
            }
        }
        output.push('\n');
    }
    output
}

struct RestoreTerminal;

impl Drop for RestoreTerminal {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

fn run_tui(app: App) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let _restore = RestoreTerminal;
    run(&mut terminal, app)
}

fn run(terminal: &mut ratatui::DefaultTerminal, mut app: App) -> io::Result<()> {
    let mut last_tick = Instant::now();
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &app))?;
        let timeout = TICK.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }
        if last_tick.elapsed() >= TICK {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
    Ok(())
}
