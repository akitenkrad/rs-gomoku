//! CPU の難易度，乱数，バックグラウンド探索．

use std::sync::mpsc::{self, Receiver};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::game::Game;
use crate::i18n::{Lang, Msg};
use crate::search::{self, Evaluation, SearchLimits};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Strong,
}

impl Difficulty {
    pub fn next(self) -> Self {
        match self {
            Self::Easy => Self::Normal,
            Self::Normal => Self::Strong,
            Self::Strong => Self::Easy,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::Easy => Msg::Easy.text(lang),
            Self::Normal => Msg::Normal.text(lang),
            Self::Strong => Msg::Strong.text(lang),
        }
    }

    pub fn search_limits(self) -> SearchLimits {
        match self {
            Self::Easy => SearchLimits {
                max_depth: 1,
                candidate_limit: 8,
                time_limit: std::time::Duration::from_millis(30),
                use_vcf: false,
            },
            Self::Normal => SearchLimits::normal(),
            Self::Strong => SearchLimits::strong(),
        }
    }
}

/// 依存を増やさないための xorshift 乱数．
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn seeded() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self::from_seed(seed)
    }

    pub fn from_seed(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    /// `low..=high` の一様乱数．
    pub fn range(&mut self, low: usize, high: usize) -> usize {
        debug_assert!(low <= high);
        low + self.next_u64() as usize % (high - low + 1)
    }

    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1_u64 << 24) as f32
    }

    pub fn chance(&mut self, percent: u32) -> bool {
        self.range(0, 99) < percent.min(100) as usize
    }
}

fn easy_choose(game: &Game, rng: &mut Rng) -> Option<usize> {
    let limits = Difficulty::Easy.search_limits();
    let evaluations = search::evaluate_moves(game, limits);
    let immediate: Vec<_> = evaluations
        .iter()
        .filter_map(|&(cell, evaluation)| (evaluation == Evaluation::Win(1)).then_some(cell))
        .collect();
    if !immediate.is_empty() {
        return Some(immediate[rng.range(0, immediate.len() - 1)]);
    }

    // 相手の五を止めない手は Loss(2) になる．7 割だけ安全な手を選ぶ．
    if evaluations.iter().any(|entry| entry.1 == Evaluation::Loss(2)) {
        let losing: Vec<_> = evaluations
            .iter()
            .filter_map(|&(cell, evaluation)| (evaluation == Evaluation::Loss(2)).then_some(cell))
            .collect();
        let safe: Vec<_> = evaluations
            .iter()
            .filter_map(|&(cell, evaluation)| (evaluation != Evaluation::Loss(2)).then_some(cell))
            .collect();
        if !safe.is_empty() && rng.chance(70) {
            return Some(safe[rng.range(0, safe.len() - 1)]);
        }
        return Some(losing[rng.range(0, losing.len() - 1)]);
    }

    let pool = evaluations.len().min(5);
    (pool > 0).then(|| evaluations[rng.range(0, pool - 1)].0)
}

/// CPU の着手を同期的に選ぶ．UI は [`spawn_choose`] を使って描画を止めない．
pub fn choose(game: &Game, difficulty: Difficulty, rng: &mut Rng) -> Option<usize> {
    if game.is_over() {
        return None;
    }
    match difficulty {
        Difficulty::Easy => easy_choose(game, rng),
        Difficulty::Normal | Difficulty::Strong => {
            let best = search::best_moves(game, difficulty.search_limits());
            (!best.is_empty()).then(|| best[rng.range(0, best.len() - 1)])
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuResult {
    pub game_id: u64,
    pub cell: Option<usize>,
}

/// 探索をワーカースレッドで実行する．呼び出し側は `game_id` が現在値と一致する結果だけ使う．
pub fn spawn_choose(game: Game, difficulty: Difficulty, seed: u64, game_id: u64) -> Receiver<CpuResult> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let cell = choose(&game, difficulty, &mut Rng::from_seed(seed));
        let _ = sender.send(CpuResult { game_id, cell });
    });
    receiver
}
