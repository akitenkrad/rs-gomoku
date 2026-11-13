//! 五目並べの盤面とルール．UI に依存しない純粋なゲームロジックを提供する．

use std::error::Error;
use std::fmt;

use crate::i18n::{Lang, Msg};

pub const MIN_BOARD_SIZE: usize = 7;
pub const MAX_BOARD_SIZE: usize = 19;
pub const DEFAULT_BOARD_SIZE: usize = 15;

/// 盤上の石．先手は常に黒を使う．
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stone {
    Black,
    White,
}

impl Stone {
    pub fn other(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Black => Msg::Black.text(lang),
            Self::White => Msg::White.text(lang),
        }
    }
}

/// 対局者．
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Human,
    Cpu,
}

impl Side {
    pub fn other(self) -> Self {
        match self {
            Self::Human => Self::Cpu,
            Self::Cpu => Self::Human,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Human => lang.default_human_name(),
            Self::Cpu => lang.default_cpu_name(),
        }
    }
}

/// 勝利となる連の長さに関するルール．
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rule {
    /// 5 個以上の連を勝ちとする．
    Freestyle,
    /// ちょうど 5 個の連だけを勝ちとする．
    Standard,
}

impl Rule {
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Freestyle => Msg::Freestyle.text(lang),
            Self::Standard => Msg::Standard.text(lang),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Freestyle => "freestyle",
            Self::Standard => "standard",
        }
    }
}

/// 盤サイズが対応範囲外であることを表す．
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizeError {
    pub size: usize,
}

impl fmt::Display for SizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "board size must be between {MIN_BOARD_SIZE} and {MAX_BOARD_SIZE}, got {}", self.size)
    }
}

impl Error for SizeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win { side: Side, cells: Vec<usize> },
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    OutOfRange,
    Occupied,
    GameOver,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutOfRange => "cell is outside the board",
            Self::Occupied => "cell is already occupied",
            Self::GameOver => "game is already over",
        })
    }
}

impl Error for MoveError {}

/// 座標表記を内部番号へ変換する．列は `a` から始まり，行は上から 1 と数える．
pub fn parse_cell(text: &str, size: usize) -> Option<usize> {
    if !(MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&size) {
        return None;
    }
    let bytes = text.as_bytes();
    if !(2..=3).contains(&bytes.len()) || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    let column = bytes[0].to_ascii_lowercase().checked_sub(b'a')? as usize;
    if column >= size || bytes[1] == b'0' || !bytes[1..].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let row = text[1..].parse::<usize>().ok()?.checked_sub(1)?;
    (row < size).then_some(row * size + column)
}

/// 内部番号を座標表記へ変換する．盤外なら `?` を返す．
pub fn cell_name(cell: usize, size: usize) -> String {
    if !(MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&size) || cell >= size * size {
        return "?".into();
    }
    let column = char::from(b'a' + (cell % size) as u8);
    format!("{column}{}", cell / size + 1)
}

/// 進行中または終局済みの対局．
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    pub size: usize,
    pub rule: Rule,
    pub board: Vec<Option<Stone>>,
    pub first: Side,
    pub to_move: Side,
    pub history: Vec<(Side, usize)>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new(DEFAULT_BOARD_SIZE, Rule::Freestyle, Side::Human)
    }
}

impl Game {
    /// 対応範囲内の盤を作る．範囲外には [`Game::try_new`] を使用する．
    pub fn new(size: usize, rule: Rule, first: Side) -> Self {
        Self::try_new(size, rule, first).expect("board size must be between 7 and 19")
    }

    pub fn try_new(size: usize, rule: Rule, first: Side) -> Result<Self, SizeError> {
        if !(MIN_BOARD_SIZE..=MAX_BOARD_SIZE).contains(&size) {
            return Err(SizeError { size });
        }
        Ok(Self {
            size,
            rule,
            board: vec![None; size * size],
            first,
            to_move: first,
            history: Vec::new(),
        })
    }

    pub fn stone_for(&self, side: Side) -> Stone {
        if side == self.first { Stone::Black } else { Stone::White }
    }

    pub fn side_for(&self, stone: Stone) -> Side {
        if stone == Stone::Black { self.first } else { self.first.other() }
    }

    pub fn stone_at(&self, cell: usize) -> Option<Stone> {
        self.board.get(cell).copied().flatten()
    }

    pub fn legal_moves(&self) -> Vec<usize> {
        if self.is_over() {
            return Vec::new();
        }
        self.board
            .iter()
            .enumerate()
            .filter_map(|(cell, stone)| stone.is_none().then_some(cell))
            .collect()
    }

    /// 現在の手番の石を置く．失敗時には盤面を変更しない．
    pub fn play(&mut self, cell: usize) -> Result<(), MoveError> {
        if cell >= self.board.len() {
            return Err(MoveError::OutOfRange);
        }
        if self.is_over() {
            return Err(MoveError::GameOver);
        }
        if self.board[cell].is_some() {
            return Err(MoveError::Occupied);
        }
        let side = self.to_move;
        self.board[cell] = Some(self.stone_for(side));
        self.history.push((side, cell));
        self.to_move = side.other();
        Ok(())
    }

    /// 参考実装との互換用に [`Game::play`] と同じ処理を行う．
    pub fn apply(&mut self, cell: usize) -> Result<(), MoveError> {
        self.play(cell)
    }

    pub fn outcome(&self) -> Option<Outcome> {
        const DIRECTIONS: [(isize, isize); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
        for row in 0..self.size {
            for column in 0..self.size {
                let start = row * self.size + column;
                let Some(stone) = self.board[start] else {
                    continue;
                };
                for (dx, dy) in DIRECTIONS {
                    if self.preceded_by(column, row, dx, dy, stone) {
                        continue;
                    }
                    let cells = self.run_from(column, row, dx, dy, stone);
                    let wins = match self.rule {
                        Rule::Freestyle => cells.len() >= 5,
                        Rule::Standard => cells.len() == 5,
                    };
                    if wins {
                        return Some(Outcome::Win {
                            side: self.side_for(stone),
                            cells,
                        });
                    }
                }
            }
        }
        self.board.iter().all(Option::is_some).then_some(Outcome::Draw)
    }

    pub fn is_over(&self) -> bool {
        self.outcome().is_some()
    }

    fn preceded_by(&self, column: usize, row: usize, dx: isize, dy: isize, stone: Stone) -> bool {
        let previous_column = column as isize - dx;
        let previous_row = row as isize - dy;
        self.index(previous_column, previous_row)
            .is_some_and(|cell| self.board[cell] == Some(stone))
    }

    fn run_from(&self, column: usize, row: usize, dx: isize, dy: isize, stone: Stone) -> Vec<usize> {
        let mut cells = Vec::new();
        let mut column = column as isize;
        let mut row = row as isize;
        while let Some(cell) = self.index(column, row) {
            if self.board[cell] != Some(stone) {
                break;
            }
            cells.push(cell);
            column += dx;
            row += dy;
        }
        cells
    }

    fn index(&self, column: isize, row: isize) -> Option<usize> {
        if column < 0 || column >= self.size as isize || row < 0 || row >= self.size as isize {
            return None;
        }
        Some(row as usize * self.size + column as usize)
    }
}
