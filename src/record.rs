//! `.gmkrec` 棋譜の読み書きと局面再構成．

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::game::{DEFAULT_BOARD_SIZE, Game, MAX_BOARD_SIZE, MIN_BOARD_SIZE, MoveError, Rule, Side, cell_name, parse_cell};
use crate::i18n::Lang;

/// 盤設定，表示名，着手列を保持する棋譜．
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub size: usize,
    pub rule: Rule,
    pub first: Side,
    /// `[人間側, CPU 側]` の表示名．
    pub names: [Option<String>; 2],
    pub moves: Vec<usize>,
}

impl Record {
    /// `ply` 手を適用した局面を返す．棋譜の範囲外なら `None`．
    pub fn game_at(&self, ply: usize) -> Option<Game> {
        game_at(self, ply)
    }

    pub fn name(&self, side: Side, lang: Lang) -> &str {
        let index = usize::from(side == Side::Cpu);
        self.names[index].as_deref().unwrap_or_else(|| match side {
            Side::Human => lang.default_human_name(),
            Side::Cpu => lang.default_cpu_name(),
        })
    }
}

impl Default for Record {
    fn default() -> Self {
        Self {
            size: DEFAULT_BOARD_SIZE,
            rule: Rule::Freestyle,
            first: Side::Human,
            names: [None, None],
            moves: Vec::new(),
        }
    }
}

/// 棋譜の構文・合法性エラー．すべて入力上の行番号を保持する．
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordError {
    UnknownKey { line: usize, key: String },
    DuplicateKey { line: usize, key: String },
    HeaderAfterMove { line: usize },
    InvalidSize { line: usize, value: String },
    InvalidRule { line: usize, value: String },
    InvalidFirst { line: usize, value: String },
    EmptyName { line: usize, key: String },
    InvalidMove { line: usize, value: String },
    CellOccupied { line: usize, cell: usize, size: usize },
    MoveAfterGameOver { line: usize },
}

impl RecordError {
    pub fn line(&self) -> usize {
        match self {
            Self::UnknownKey { line, .. }
            | Self::DuplicateKey { line, .. }
            | Self::HeaderAfterMove { line }
            | Self::InvalidSize { line, .. }
            | Self::InvalidRule { line, .. }
            | Self::InvalidFirst { line, .. }
            | Self::EmptyName { line, .. }
            | Self::InvalidMove { line, .. }
            | Self::CellOccupied { line, .. }
            | Self::MoveAfterGameOver { line } => *line,
        }
    }
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: ", self.line())?;
        match self {
            Self::UnknownKey { key, .. } => write!(f, "unknown key '{key}'"),
            Self::DuplicateKey { key, .. } => write!(f, "duplicate key '{key}'"),
            Self::HeaderAfterMove { .. } => f.write_str("header after first move"),
            Self::InvalidSize { value, .. } => write!(f, "invalid board size '{value}'"),
            Self::InvalidRule { value, .. } => write!(f, "invalid rule '{value}'"),
            Self::InvalidFirst { value, .. } => write!(f, "invalid first side '{value}'"),
            Self::EmptyName { key, .. } => write!(f, "{key} name must not be empty"),
            Self::InvalidMove { value, .. } => write!(f, "invalid move '{value}'"),
            Self::CellOccupied { cell, size, .. } => {
                write!(f, "cell {} is already occupied", cell_name(*cell, *size))
            }
            Self::MoveAfterGameOver { .. } => f.write_str("move after game over"),
        }
    }
}

impl Error for RecordError {}

/// UTF-8 の `.gmkrec` テキストを解析する．
pub fn parse(text: &str) -> Result<Record, RecordError> {
    let mut record = Record::default();
    let mut game = Game::new(record.size, record.rule, record.first);
    let mut seen_keys = HashSet::new();
    let mut saw_move = false;

    for (index, raw_line) in text.lines().enumerate() {
        let line = index + 1;
        let content = raw_line.split_once('#').map_or(raw_line, |(before, _)| before).trim();
        if content.is_empty() {
            continue;
        }

        if let Some((raw_key, raw_value)) = content.split_once('=') {
            if saw_move {
                return Err(RecordError::HeaderAfterMove { line });
            }
            let key = raw_key.trim();
            let value = raw_value.trim();
            if !matches!(key, "size" | "rule" | "first" | "human" | "cpu") {
                return Err(RecordError::UnknownKey { line, key: key.to_string() });
            }
            if !seen_keys.insert(key.to_string()) {
                return Err(RecordError::DuplicateKey { line, key: key.to_string() });
            }
            match key {
                "size" => {
                    record.size = value
                        .parse::<usize>()
                        .ok()
                        .filter(|size| (MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(size))
                        .ok_or_else(|| RecordError::InvalidSize {
                            line,
                            value: value.to_string(),
                        })?;
                }
                "rule" => {
                    record.rule = match value {
                        "freestyle" => Rule::Freestyle,
                        "standard" => Rule::Standard,
                        _ => {
                            return Err(RecordError::InvalidRule {
                                line,
                                value: value.to_string(),
                            });
                        }
                    };
                }
                "first" => {
                    record.first = match value {
                        "human" => Side::Human,
                        "cpu" => Side::Cpu,
                        _ => {
                            return Err(RecordError::InvalidFirst {
                                line,
                                value: value.to_string(),
                            });
                        }
                    };
                }
                "human" | "cpu" => {
                    if value.is_empty() {
                        return Err(RecordError::EmptyName { line, key: key.to_string() });
                    }
                    record.names[usize::from(key == "cpu")] = Some(value.to_string());
                }
                _ => unreachable!(),
            }
            game = Game::new(record.size, record.rule, record.first);
            continue;
        }

        saw_move = true;
        if game.is_over() {
            return Err(RecordError::MoveAfterGameOver { line });
        }
        let cell = parse_cell(content, record.size).ok_or_else(|| RecordError::InvalidMove {
            line,
            value: content.to_string(),
        })?;
        match game.play(cell) {
            Ok(()) => record.moves.push(cell),
            Err(MoveError::Occupied) => {
                return Err(RecordError::CellOccupied {
                    line,
                    cell,
                    size: record.size,
                });
            }
            Err(MoveError::GameOver) => return Err(RecordError::MoveAfterGameOver { line }),
            Err(MoveError::OutOfRange) => unreachable!("parse_cell returned an invalid cell"),
        }
    }

    Ok(record)
}

/// 棋譜を座標表記の標準テキストへ変換する．
pub fn to_text(record: &Record) -> String {
    let first = match record.first {
        Side::Human => "human",
        Side::Cpu => "cpu",
    };
    let mut text = format!(
        "# rs-gomoku record v1\nsize  = {}\nrule  = {}\nfirst = {first}\n",
        record.size,
        record.rule.as_str(),
    );
    if let Some(name) = record.names[0].as_deref().filter(|name| !matches!(*name, "You" | "あなた")) {
        text.push_str(&format!("human = {name}\n"));
    }
    if let Some(name) = record.names[1].as_deref().filter(|name| *name != "CPU") {
        text.push_str(&format!("cpu   = {name}\n"));
    }
    text.push('\n');
    for &cell in &record.moves {
        text.push_str(&cell_name(cell, record.size));
        text.push('\n');
    }
    text
}

/// `ply` 手を適用した局面を，履歴を含めて再構成する．
pub fn game_at(record: &Record, ply: usize) -> Option<Game> {
    if ply > record.moves.len() {
        return None;
    }
    let mut game = Game::try_new(record.size, record.rule, record.first).ok()?;
    for &cell in record.moves.iter().take(ply) {
        game.play(cell).ok()?;
    }
    Some(game)
}
