//! 五目並べのパターン評価と期限付き探索．

use std::cmp::Reverse;
use std::time::{Duration, Instant};

use crate::game::{Game, Outcome, Rule, Stone};

const DIRECTIONS: [(isize, isize); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
const MATE_SCORE: i32 = 10_000_000;
const OPPONENT_ATTACK_WEIGHT_PERCENT: i32 = 80;

/// ある候補手を選んだ側から見た評価．
///
/// `Win` と `Loss` の手数は，候補手そのものを 1 手目として数える．
/// これらは終局または VCF で証明できた場合だけ返す．
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Evaluation {
    Win(u16),
    Loss(u16),
    Score(i32),
}

impl Evaluation {
    /// UI の並べ替えやヒートマップに使える数値へ写像する．
    pub fn score(self) -> i32 {
        match self {
            Self::Win(turns) => MATE_SCORE - i32::from(turns),
            Self::Loss(turns) => -MATE_SCORE + i32::from(turns),
            Self::Score(score) => score,
        }
    }
}

/// 1 点へ石を置いたときに，各方向でできる代表的な形の個数．
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PatternCounts {
    pub five: u8,
    pub open_four: u8,
    pub four: u8,
    pub open_three: u8,
    pub sleeping_three: u8,
    pub open_two: u8,
}

/// 探索量と期限．
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchLimits {
    pub max_depth: u8,
    pub candidate_limit: usize,
    pub time_limit: Duration,
    pub use_vcf: bool,
}

impl SearchLimits {
    pub const fn normal() -> Self {
        Self {
            max_depth: 4,
            candidate_limit: 12,
            time_limit: Duration::from_millis(300),
            use_vcf: false,
        }
    }

    pub const fn strong() -> Self {
        Self {
            max_depth: 6,
            candidate_limit: 16,
            time_limit: Duration::from_millis(1_000),
            use_vcf: true,
        }
    }
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self::normal()
    }
}

/// 空盤なら中央，それ以外なら既存の石からチェビシェフ距離 2 以内の空点を返す．
pub fn candidate_moves(game: &Game) -> Vec<usize> {
    if game.is_over() {
        return Vec::new();
    }
    if game.board.iter().all(Option::is_none) {
        return vec![(game.size / 2) * game.size + game.size / 2];
    }

    let mut nearby = vec![false; game.board.len()];
    for (occupied, stone) in game.board.iter().enumerate() {
        if stone.is_none() {
            continue;
        }
        let column = occupied % game.size;
        let row = occupied / game.size;
        let column_start = column.saturating_sub(2);
        let column_end = (column + 2).min(game.size - 1);
        let row_start = row.saturating_sub(2);
        let row_end = (row + 2).min(game.size - 1);
        for candidate_row in row_start..=row_end {
            for candidate_column in column_start..=column_end {
                let candidate = candidate_row * game.size + candidate_column;
                if game.board[candidate].is_none() {
                    nearby[candidate] = true;
                }
            }
        }
    }
    nearby
        .into_iter()
        .enumerate()
        .filter_map(|(cell, is_nearby)| is_nearby.then_some(cell))
        .collect()
}

/// `cell` に `stone` を仮置きしたときの連と開閉を数える．
pub fn patterns_after_move(game: &Game, cell: usize, stone: Stone) -> PatternCounts {
    if cell >= game.board.len() || game.board[cell].is_some_and(|current| current != stone) {
        return PatternCounts::default();
    }

    let mut result = PatternCounts::default();
    let column = cell % game.size;
    let row = cell / game.size;
    for (dx, dy) in DIRECTIONS {
        let (backward, backward_open) = ray(game, column, row, -dx, -dy, stone);
        let (forward, forward_open) = ray(game, column, row, dx, dy, stone);
        let length = backward + 1 + forward;
        let open_ends = u8::from(backward_open) + u8::from(forward_open);
        let is_five = match game.rule {
            Rule::Freestyle => length >= 5,
            Rule::Standard => length == 5,
        };
        if is_five {
            result.five += 1;
        } else {
            match (length, open_ends) {
                (4, 2) => result.open_four += 1,
                (4, 1) => result.four += 1,
                (3, 2) => result.open_three += 1,
                (3, 1) => result.sleeping_three += 1,
                (2, 2) => result.open_two += 1,
                _ => {}
            }
        }
    }
    result
}

fn ray(game: &Game, column: usize, row: usize, dx: isize, dy: isize, stone: Stone) -> (usize, bool) {
    let mut length = 0;
    let mut x = column as isize + dx;
    let mut y = row as isize + dy;
    while in_board(game.size, x, y) {
        let cell = y as usize * game.size + x as usize;
        if game.board[cell] != Some(stone) {
            return (length, game.board[cell].is_none());
        }
        length += 1;
        x += dx;
        y += dy;
    }
    (length, false)
}

fn in_board(size: usize, column: isize, row: isize) -> bool {
    column >= 0 && column < size as isize && row >= 0 && row < size as isize
}

fn pattern_score(patterns: PatternCounts) -> i32 {
    i32::from(patterns.five) * 2_000_000
        + i32::from(patterns.open_four) * 260_000
        + i32::from(patterns.four) * 65_000
        + i32::from(patterns.open_three) * 18_000
        + i32::from(patterns.sleeping_three) * 3_000
        + i32::from(patterns.open_two) * 700
}

fn static_move_score(game: &Game, cell: usize, stone: Stone) -> i32 {
    let attack = patterns_after_move(game, cell, stone);
    let defence = patterns_after_move(game, cell, stone.other());
    let center = game.size as isize / 2;
    let column = cell % game.size;
    let row = cell / game.size;
    let centrality = (game.size as isize - (column as isize - center).abs() - (row as isize - center).abs()).max(0) as i32;
    pattern_score(attack) + pattern_score(defence) * 9 / 10 + centrality
}

fn ordered_candidates(game: &Game, limit: usize) -> Vec<usize> {
    let stone = game.stone_for(game.to_move);
    let mut candidates = candidate_moves(game);
    candidates.sort_by_key(|&cell| Reverse((static_move_score(game, cell, stone), Reverse(cell))));
    candidates.truncate(limit.max(1));
    candidates
}

fn winning_moves_for(game: &Game, stone: Stone) -> Vec<usize> {
    candidate_moves(game)
        .into_iter()
        .filter(|&cell| patterns_after_move(game, cell, stone).five > 0)
        .collect()
}

fn attack_balance(game: &Game) -> i32 {
    let stone = game.stone_for(game.to_move);
    let opponent = stone.other();
    let mut best_attack = 0;
    let mut best_opponent_attack = 0;
    for cell in candidate_moves(game) {
        best_attack = best_attack.max(pattern_score(patterns_after_move(game, cell, stone)));
        best_opponent_attack = best_opponent_attack.max(pattern_score(patterns_after_move(game, cell, opponent)));
    }
    // 同じ強さの攻撃なら，先に着手できる手番側を高く評価する．
    best_attack - best_opponent_attack * OPPONENT_ATTACK_WEIGHT_PERCENT / 100
}

#[derive(Clone, Copy, Debug)]
struct Timeout;

fn position_score(game: &Game, ply: u16) -> i32 {
    let stone = game.stone_for(game.to_move);
    let opponent = stone.other();
    let mut best_attack = 0;
    let mut best_opponent_attack = 0;
    let mut opponent_winning_point = None;
    let mut opponent_winning_point_count = 0;

    for cell in candidate_moves(game) {
        let attack = patterns_after_move(game, cell, stone);
        if attack.five > 0 {
            return MATE_SCORE - i32::from(ply.saturating_add(1));
        }
        best_attack = best_attack.max(pattern_score(attack));

        let opponent_attack = patterns_after_move(game, cell, opponent);
        if opponent_attack.five > 0 {
            opponent_winning_point = Some(cell);
            opponent_winning_point_count += 1;
        }
        best_opponent_attack = best_opponent_attack.max(pattern_score(opponent_attack));
    }

    if opponent_winning_point_count >= 2 {
        return -MATE_SCORE + i32::from(ply.saturating_add(2));
    }
    if let Some(forced_block) = opponent_winning_point {
        // 勝点が一つだけなら，その強制ブロック後の局面を次手番側から評価して戻す．
        let mut after_block = game.clone();
        after_block.play(forced_block).expect("a winning point is a legal forced block");
        return -attack_balance(&after_block);
    }

    let opponent_pressure = best_opponent_attack * OPPONENT_ATTACK_WEIGHT_PERCENT / 100;
    best_attack - opponent_pressure
}

fn negamax(game: &Game, depth: u8, mut alpha: i32, beta: i32, deadline: Instant, candidate_limit: usize, ply: u16) -> Result<i32, Timeout> {
    if Instant::now() >= deadline {
        return Err(Timeout);
    }
    if let Some(outcome) = game.outcome() {
        return Ok(match outcome {
            Outcome::Win { .. } => -MATE_SCORE + i32::from(ply),
            Outcome::Draw => 0,
        });
    }
    if depth == 0 {
        return Ok(position_score(game, ply));
    }

    let mut best = -MATE_SCORE;
    for cell in ordered_candidates(game, candidate_limit) {
        let mut next = game.clone();
        next.play(cell).expect("ordered candidates are legal");
        let score = -negamax(&next, depth - 1, -beta, -alpha, deadline, candidate_limit, ply + 1)?;
        best = best.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }
    Ok(best)
}

/// VCF の攻撃側の手番から，勝ちまでの最短手数を探す．
fn vcf_from_attacker_turn(game: &Game, attacker: Stone, attacker_moves_left: u8, deadline: Instant, candidate_limit: usize) -> Option<u16> {
    if attacker_moves_left == 0 || Instant::now() >= deadline {
        return None;
    }
    let mut best = None;
    let mut forcing = candidate_moves(game);
    forcing.sort_by_key(|&cell| Reverse(static_move_score(game, cell, attacker)));
    forcing.truncate(candidate_limit);
    for cell in forcing {
        if Instant::now() >= deadline {
            break;
        }
        let mut after_attack = game.clone();
        if after_attack.play(cell).is_err() {
            continue;
        }
        if matches!(after_attack.outcome(), Some(Outcome::Win { .. })) {
            best = Some(best.map_or(1, |turns: u16| turns.min(1)));
            continue;
        }
        let threats = winning_moves_for(&after_attack, attacker);
        if threats.is_empty() {
            continue;
        }
        let defender = attacker.other();
        if !winning_moves_for(&after_attack, defender).is_empty() {
            continue;
        }
        if threats.len() >= 2 {
            best = Some(best.map_or(3, |turns: u16| turns.min(3)));
            continue;
        }

        let mut after_defence = after_attack;
        if after_defence.play(threats[0]).is_err() {
            continue;
        }
        if let Some(rest) = vcf_from_attacker_turn(&after_defence, attacker, attacker_moves_left - 1, deadline, candidate_limit) {
            let turns = rest.saturating_add(2);
            best = Some(best.map_or(turns, |current: u16| current.min(turns)));
        }
    }
    best
}

fn proven_evaluation_after_move(game: &Game, cell: usize, limits: SearchLimits, deadline: Instant) -> Option<Evaluation> {
    let attacker = game.stone_for(game.to_move);
    let mut next = game.clone();
    next.play(cell).ok()?;
    if matches!(next.outcome(), Some(Outcome::Win { .. })) {
        return Some(Evaluation::Win(1));
    }

    // 相手に即勝ちがあれば，この候補手が 2 手で負けることは証明済みである．
    if !winning_moves_for(&next, attacker.other()).is_empty() {
        return Some(Evaluation::Loss(2));
    }
    if limits.use_vcf {
        let threats = winning_moves_for(&next, attacker);
        if threats.len() >= 2 {
            return Some(Evaluation::Win(3));
        }
        if threats.len() == 1 {
            let mut after_defence = next;
            after_defence.play(threats[0]).ok()?;
            if let Some(rest) = vcf_from_attacker_turn(&after_defence, attacker, 4, deadline, limits.candidate_limit) {
                return Some(Evaluation::Win(rest.saturating_add(2)));
            }
        }
    }
    None
}

fn evaluation_rank(evaluation: Evaluation) -> (u8, i32) {
    match evaluation {
        Evaluation::Win(turns) => (2, -i32::from(turns)),
        Evaluation::Score(score) => (1, score),
        Evaluation::Loss(turns) => (0, i32::from(turns)),
    }
}

/// 候補手を反復深化で評価し，良い順に返す．
pub fn evaluate_moves(game: &Game, limits: SearchLimits) -> Vec<(usize, Evaluation)> {
    if game.is_over() {
        return Vec::new();
    }
    let started = Instant::now();
    let deadline = started.checked_add(limits.time_limit).unwrap_or(started);
    let stone = game.stone_for(game.to_move);
    let candidates = ordered_candidates(game, limits.candidate_limit);
    let mut evaluations: Vec<_> = candidates
        .iter()
        .copied()
        .map(|cell| {
            let evaluation =
                proven_evaluation_after_move(game, cell, limits, deadline).unwrap_or_else(|| Evaluation::Score(static_move_score(game, cell, stone)));
            (cell, evaluation)
        })
        .collect();

    // 完了した深さの結果だけを採用するため，期限到達で中途半端な値を混ぜない．
    for depth in 1..=limits.max_depth {
        if Instant::now() >= deadline {
            break;
        }
        let mut iteration = Vec::with_capacity(evaluations.len());
        let mut completed = true;
        for &(cell, proven) in &evaluations {
            if !matches!(proven, Evaluation::Score(_)) {
                iteration.push((cell, proven));
                continue;
            }
            let mut next = game.clone();
            next.play(cell).expect("evaluated candidates are legal");
            match negamax(
                &next,
                depth.saturating_sub(1),
                -MATE_SCORE,
                MATE_SCORE,
                deadline,
                limits.candidate_limit,
                1,
            ) {
                Ok(score) => iteration.push((cell, Evaluation::Score(-score))),
                Err(Timeout) => {
                    completed = false;
                    break;
                }
            }
        }
        if completed {
            evaluations = iteration;
        } else {
            break;
        }
    }

    evaluations.sort_by_key(|&(cell, evaluation)| Reverse((evaluation_rank(evaluation), Reverse(cell))));
    evaluations
}

/// 最良評価と同値の候補手を盤の読み順で返す．
pub fn best_moves(game: &Game, limits: SearchLimits) -> Vec<usize> {
    let evaluations = evaluate_moves(game, limits);
    let Some(&(_, best)) = evaluations.first() else {
        return Vec::new();
    };
    let best_rank = evaluation_rank(best);
    let mut result: Vec<_> = evaluations
        .into_iter()
        .filter_map(|(cell, evaluation)| (evaluation_rank(evaluation) == best_rank).then_some(cell))
        .collect();
    result.sort_unstable();
    result
}

/// 同値なら盤の読み順を優先して最良手を 1 つ返す．
pub fn choose_best(game: &Game, limits: SearchLimits) -> Option<usize> {
    best_moves(game, limits).into_iter().next()
}
