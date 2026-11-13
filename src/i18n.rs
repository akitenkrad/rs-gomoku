//! 画面とテキスト出力に用いる日本語・英語の文言．

use std::fmt::Display;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Ja,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Self::En => Self::Ja,
            Self::Ja => Self::En,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("en") {
            Some(Self::En)
        } else if value.eq_ignore_ascii_case("ja") {
            Some(Self::Ja)
        } else {
            None
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ja => "日本語",
        }
    }

    pub fn default_human_name(self) -> &'static str {
        match self {
            Self::En => "You",
            Self::Ja => "あなた",
        }
    }

    pub fn default_cpu_name(self) -> &'static str {
        "CPU"
    }

    pub fn placed(self, name: &str, cell: &str) -> String {
        match self {
            Self::En => format!("{name} placed at {cell}"),
            Self::Ja => format!("{name} が {cell} に打ちました"),
        }
    }

    pub fn showing_move(self, position: usize) -> String {
        match self {
            Self::En => format!("Showing move {position}"),
            Self::Ja => format!("{position} 手目を表示しています"),
        }
    }

    pub fn replay_position(self, position: usize) -> String {
        self.showing_move(position)
    }

    pub fn cpu_selected(self, cell: &str) -> String {
        match self {
            Self::En => format!("CPU selected {cell}"),
            Self::Ja => format!("CPU は {cell} を選びました"),
        }
    }

    pub fn winner(self, name: &str) -> String {
        match self {
            Self::En => format!("{name} wins"),
            Self::Ja => format!("{name} の勝ち"),
        }
    }

    pub fn record_saved(self, path: &str) -> String {
        match self {
            Self::En => format!("Record saved: {path}"),
            Self::Ja => format!("棋譜を保存しました: {path}"),
        }
    }

    pub fn record_save_failed(self, error: impl Display) -> String {
        match self {
            Self::En => format!("Failed to save record: {error}"),
            Self::Ja => format!("棋譜の保存に失敗しました: {error}"),
        }
    }

    pub fn resize(self, min_width: u16, min_height: u16, width: u16, height: u16) -> String {
        match self {
            Self::En => format!("Resize the terminal to at least {min_width}x{min_height} (currently {width}x{height})"),
            Self::Ja => format!("端末を {min_width}x{min_height} 以上に広げてください (現在 {width}x{height})"),
        }
    }

    pub fn replay_progress(self, position: usize, total: usize) -> String {
        match self {
            Self::En => format!("Move {position}/{total}"),
            Self::Ja => format!("手数 {position}/{total}"),
        }
    }

    pub fn turn(self, name: &str, stone: &str) -> String {
        match self {
            Self::En if name == self.default_human_name() => format!("Your turn — {stone}"),
            Self::En => format!("{name}'s turn — {stone}"),
            Self::Ja => format!("{name}の手番 — {stone}"),
        }
    }

    pub fn stats(self, wins: u32, losses: u32, draws: u32) -> String {
        match self {
            Self::En => format!("  Wins {wins}   Losses {losses}   Draws {draws}"),
            Self::Ja => format!("  勝 {wins}   敗 {losses}   分 {draws}"),
        }
    }

    pub fn evaluation(self, result: Msg, turns: u16) -> String {
        format!("{} {turns}", result.text(self))
    }

    pub fn moves_count(self, count: usize) -> String {
        match self {
            Self::En => format!("Moves {count}"),
            Self::Ja => format!("手数 {count}"),
        }
    }

    pub fn move_line(self, index: usize, name: &str, stone: &str, cell: &str) -> String {
        match self {
            Self::En => format!("Move {index}: {name} {stone} {cell}"),
            Self::Ja => format!("手数 {index}: {name} {stone} {cell}"),
        }
    }

    pub fn result_winner(self, name: &str) -> String {
        match self {
            Self::En => format!("Result: {name} wins"),
            Self::Ja => format!("結果: {name} の勝ち"),
        }
    }

    pub fn board_rule(self, size: usize, rule: &str) -> String {
        match self {
            Self::En => format!("{size}x{size}  {rule}"),
            Self::Ja => format!("{size}路  {rule}ルール"),
        }
    }

    pub fn board_size(self, size: usize) -> String {
        match self {
            Self::En => format!("{size}x{size}"),
            Self::Ja => format!("{size} 路"),
        }
    }

    pub fn first_player(self, name: &str, stone: &str) -> String {
        match self {
            Self::En => format!("{name} ({stone})"),
            Self::Ja => format!("{name}（{stone}）"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    TitleSubtitle,
    StartGame,
    Difficulty,
    First,
    Rule,
    BoardSize,
    Language,
    Quit,
    Settings,
    StartPrompt,
    Easy,
    Normal,
    Strong,
    Freestyle,
    Standard,
    Black,
    White,
    HumanTurn,
    CpuThinking,
    CpuSelected,
    CellUnavailable,
    HumanWins,
    CpuWins,
    Draw,
    RecordStart,
    EndOfRecord,
    RecordFinished,
    Paused,
    Playing,
    Speed,
    Turn,
    Board,
    EvalWin,
    EvalLoss,
    Calculating,
    TopMoves,
    Hint,
    HighlightWinning,
    CannotAvoidLoss,
    Status,
    SessionScore,
    MoveLog,
    PlayPause,
    PreviousNext,
    BeginningEnd,
    Evaluation,
    NewGame,
    Save,
    Title,
    Select,
    ArrowKeys,
    Place,
    New,
    Back,
    End,
    Result,
    ReplayBackQuit,
    NewGameSave,
    ResultDraw,
    ResultUnfinished,
}

impl Msg {
    pub const ALL: [Self; 60] = [
        Self::TitleSubtitle,
        Self::StartGame,
        Self::Difficulty,
        Self::First,
        Self::Rule,
        Self::BoardSize,
        Self::Language,
        Self::Quit,
        Self::Settings,
        Self::StartPrompt,
        Self::Easy,
        Self::Normal,
        Self::Strong,
        Self::Freestyle,
        Self::Standard,
        Self::Black,
        Self::White,
        Self::HumanTurn,
        Self::CpuThinking,
        Self::CpuSelected,
        Self::CellUnavailable,
        Self::HumanWins,
        Self::CpuWins,
        Self::Draw,
        Self::RecordStart,
        Self::EndOfRecord,
        Self::RecordFinished,
        Self::Paused,
        Self::Playing,
        Self::Speed,
        Self::Turn,
        Self::Board,
        Self::EvalWin,
        Self::EvalLoss,
        Self::Calculating,
        Self::TopMoves,
        Self::Hint,
        Self::HighlightWinning,
        Self::CannotAvoidLoss,
        Self::Status,
        Self::SessionScore,
        Self::MoveLog,
        Self::PlayPause,
        Self::PreviousNext,
        Self::BeginningEnd,
        Self::Evaluation,
        Self::NewGame,
        Self::Save,
        Self::Title,
        Self::Select,
        Self::ArrowKeys,
        Self::Place,
        Self::New,
        Self::Back,
        Self::End,
        Self::Result,
        Self::ReplayBackQuit,
        Self::NewGameSave,
        Self::ResultDraw,
        Self::ResultUnfinished,
    ];

    pub fn text(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (Self::TitleSubtitle, Lang::En) => "Gomoku in your terminal — play, analyse, replay",
            (Self::TitleSubtitle, Lang::Ja) => "ターミナル五目並べ — 対局・解析・棋譜再生",
            (Self::StartGame, Lang::En) => "Start game",
            (Self::StartGame, Lang::Ja) => "対局開始",
            (Self::Difficulty, Lang::En) => "Difficulty",
            (Self::Difficulty, Lang::Ja) => "難易度",
            (Self::First, Lang::En) => "First player",
            (Self::First, Lang::Ja) => "先手",
            (Self::Rule, Lang::En) => "Rule",
            (Self::Rule, Lang::Ja) => "ルール",
            (Self::BoardSize, Lang::En) => "Board size",
            (Self::BoardSize, Lang::Ja) => "盤サイズ",
            (Self::Language, Lang::En) => "Language",
            (Self::Language, Lang::Ja) => "言語",
            (Self::Quit, Lang::En) => "Quit",
            (Self::Quit, Lang::Ja) => "終了",
            (Self::Settings, Lang::En) => "Settings",
            (Self::Settings, Lang::Ja) => "設定",
            (Self::StartPrompt, Lang::En) => "Press Enter / Space to start",
            (Self::StartPrompt, Lang::Ja) => "Enter / Space で開始",
            (Self::Easy, Lang::En) => "Easy",
            (Self::Easy, Lang::Ja) => "やさしい",
            (Self::Normal, Lang::En) => "Normal",
            (Self::Normal, Lang::Ja) => "ふつう",
            (Self::Strong, Lang::En) => "Strong",
            (Self::Strong, Lang::Ja) => "つよい",
            (Self::Freestyle, Lang::En) => "Freestyle",
            (Self::Freestyle, Lang::Ja) => "自由",
            (Self::Standard, Lang::En) => "Standard",
            (Self::Standard, Lang::Ja) => "標準",
            (Self::Black, Lang::En) => "Black",
            (Self::Black, Lang::Ja) => "黒",
            (Self::White, Lang::En) => "White",
            (Self::White, Lang::Ja) => "白",
            (Self::HumanTurn, Lang::En) => "Your turn — choose an intersection, then Enter",
            (Self::HumanTurn, Lang::Ja) => "あなたの番です．交点を選んで Enter",
            (Self::CpuThinking, Lang::En) => "CPU is thinking",
            (Self::CpuThinking, Lang::Ja) => "CPU が考えています",
            (Self::CpuSelected, Lang::En) => "CPU selected a move",
            (Self::CpuSelected, Lang::Ja) => "CPU が着手を選びました",
            (Self::CellUnavailable, Lang::En) => "You cannot place there",
            (Self::CellUnavailable, Lang::Ja) => "その交点には打てません",
            (Self::HumanWins, Lang::En) => "You win!",
            (Self::HumanWins, Lang::Ja) => "あなたの勝ちです！",
            (Self::CpuWins, Lang::En) => "CPU wins",
            (Self::CpuWins, Lang::Ja) => "CPU の勝ちです",
            (Self::Draw, Lang::En) => "Draw",
            (Self::Draw, Lang::Ja) => "引き分けです",
            (Self::RecordStart, Lang::En) => "Start of record",
            (Self::RecordStart, Lang::Ja) => "棋譜の先頭です",
            (Self::EndOfRecord, Lang::En) => "End of record",
            (Self::EndOfRecord, Lang::Ja) => "棋譜の終わりです",
            (Self::RecordFinished, Lang::En) => "Record is already finished",
            (Self::RecordFinished, Lang::Ja) => "終局後には着手できません",
            (Self::Paused, Lang::En) => "Paused",
            (Self::Paused, Lang::Ja) => "一時停止",
            (Self::Playing, Lang::En) => "Playing",
            (Self::Playing, Lang::Ja) => "再生中",
            (Self::Speed, Lang::En) => "Speed",
            (Self::Speed, Lang::Ja) => "速度",
            (Self::Turn, Lang::En) => "Turn",
            (Self::Turn, Lang::Ja) => "手番",
            (Self::Board, Lang::En) => "Board",
            (Self::Board, Lang::Ja) => "盤面",
            (Self::EvalWin, Lang::En) => "Win",
            (Self::EvalWin, Lang::Ja) => "勝",
            (Self::EvalLoss, Lang::En) => "Loss",
            (Self::EvalLoss, Lang::Ja) => "負",
            (Self::Calculating, Lang::En) => "… calculating",
            (Self::Calculating, Lang::Ja) => "… 計算中",
            (Self::TopMoves, Lang::En) => "Top 5 moves",
            (Self::TopMoves, Lang::Ja) => "上位 5 手",
            (Self::Hint, Lang::En) => "Hint",
            (Self::Hint, Lang::Ja) => "ヒント",
            (Self::HighlightWinning, Lang::En) => "Highlighting winning moves",
            (Self::HighlightWinning, Lang::Ja) => "勝てる手を強調中",
            (Self::CannotAvoidLoss, Lang::En) => "A loss cannot be avoided",
            (Self::CannotAvoidLoss, Lang::Ja) => "負けを避けられません",
            (Self::Status, Lang::En) => "Status",
            (Self::Status, Lang::Ja) => "状態",
            (Self::SessionScore, Lang::En) => "Session score",
            (Self::SessionScore, Lang::Ja) => "セッション戦績",
            (Self::MoveLog, Lang::En) => "Move log",
            (Self::MoveLog, Lang::Ja) => "着手ログ",
            (Self::PlayPause, Lang::En) => "Play/Pause",
            (Self::PlayPause, Lang::Ja) => "再生/停止",
            (Self::PreviousNext, Lang::En) => "Previous/Next",
            (Self::PreviousNext, Lang::Ja) => "前後",
            (Self::BeginningEnd, Lang::En) => "Beginning/End",
            (Self::BeginningEnd, Lang::Ja) => "先頭/末尾",
            (Self::Evaluation, Lang::En) => "Evaluation",
            (Self::Evaluation, Lang::Ja) => "評価",
            (Self::NewGame, Lang::En) => "New game",
            (Self::NewGame, Lang::Ja) => "新しい対局",
            (Self::Save, Lang::En) => "Save",
            (Self::Save, Lang::Ja) => "保存",
            (Self::Title, Lang::En) => "Title",
            (Self::Title, Lang::Ja) => "タイトル",
            (Self::Select, Lang::En) => "Select",
            (Self::Select, Lang::Ja) => "選択",
            (Self::ArrowKeys, Lang::En) => "Arrows",
            (Self::ArrowKeys, Lang::Ja) => "矢印",
            (Self::Place, Lang::En) => "Place",
            (Self::Place, Lang::Ja) => "置く",
            (Self::New, Lang::En) => "New",
            (Self::New, Lang::Ja) => "新規",
            (Self::Back, Lang::En) => "Back",
            (Self::Back, Lang::Ja) => "戻る",
            (Self::End, Lang::En) => "End",
            (Self::End, Lang::Ja) => "終了",
            (Self::Result, Lang::En) => "Result",
            (Self::Result, Lang::Ja) => "結果",
            (Self::ReplayBackQuit, Lang::En) => "← Back   q Quit",
            (Self::ReplayBackQuit, Lang::Ja) => "← で戻る   q で終了",
            (Self::NewGameSave, Lang::En) => "Enter New game   s Save",
            (Self::NewGameSave, Lang::Ja) => "Enter で新規対局   s で保存",
            (Self::ResultDraw, Lang::En) => "Result: draw",
            (Self::ResultDraw, Lang::Ja) => "結果: 引き分け",
            (Self::ResultUnfinished, Lang::En) => "Result: unfinished",
            (Self::ResultUnfinished, Lang::Ja) => "結果: 未終局",
        }
    }
}
