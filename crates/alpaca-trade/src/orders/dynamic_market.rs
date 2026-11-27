use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Pace profile. Index products, the close, hedges, and emergency exits use `Fast`.
/// Ordinary opens use `Standard`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[ts(export, export_to = "../../../packages/alpaca-trade/src/generated/")]
#[serde(rename_all = "snake_case")]
pub enum DynamicMarketProfile {
    #[default]
    Standard,
    Fast,
}

/// Persisted fields of `Execution::DynamicMarket`. New fields default so old JSON still loads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicMarketState {
    pub limit_price: Decimal,
    pub start_percentage: f64,
    pub current_percentage: f64,
    pub percentage_step: f64,
    pub interval_seconds: i64,
    pub last_adjustment_time: Option<NaiveDateTime>,
    pub profile: DynamicMarketProfile,
    pub start_at_mid: bool,
    pub initial_mid: Option<Decimal>,
    pub initial_spread: Option<Decimal>,
    pub step_index: u16,
    pub step_entered_at: Option<NaiveDateTime>,
    pub round: u8,
    pub max_rounds: u8,
    pub drift_accelerated: bool,
    pub sanity_pause_count: u8,
    pub replace_count: u16,
    pub stalled: bool,
    pub phase_label: String,
    pub leg_count: u8,
    /// One-shot request to start another high-band round without rewinding to 40%.
    pub restart: bool,
}

impl Default for DynamicMarketState {
    fn default() -> Self {
        Self {
            limit_price: Decimal::ZERO,
            start_percentage: 0.4,
            current_percentage: 0.4,
            percentage_step: 0.05,
            interval_seconds: 9,
            last_adjustment_time: None,
            profile: DynamicMarketProfile::Standard,
            start_at_mid: false,
            initial_mid: None,
            initial_spread: None,
            step_index: 0,
            step_entered_at: None,
            round: 1,
            max_rounds: 3,
            drift_accelerated: false,
            sanity_pause_count: 0,
            replace_count: 0,
            stalled: false,
            phase_label: String::new(),
            leg_count: 0,
            restart: false,
        }
    }
}

impl DynamicMarketState {
    pub fn legacy(
        limit_price: Decimal,
        start_percentage: f64,
        current_percentage: f64,
        percentage_step: f64,
        interval_seconds: i64,
        last_adjustment_time: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            limit_price,
            start_percentage,
            current_percentage,
            percentage_step,
            interval_seconds,
            last_adjustment_time,
            ..Self::default()
        }
    }

    pub fn normalize_bounds(&mut self) {
        if self.round == 0 {
            self.round = 1;
        }
        if self.max_rounds == 0 {
            self.max_rounds = 3;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicMarketQuote {
    pub best: Decimal,
    pub worst: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicMarketStepOptions {
    pub positive_price: bool,
    pub leg_count: u8,
    pub force: bool,
}

impl Default for DynamicMarketStepOptions {
    fn default() -> Self {
        Self {
            positive_price: false,
            leg_count: 0,
            force: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicMarketKind {
    Wait,
    Replace,
    Recreate,
    MarkStalled,
    MissingQuote,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMarketOutcome {
    pub state: DynamicMarketState,
    pub kind: DynamicMarketKind,
}

#[derive(Clone, Copy)]
enum Anchor {
    Percent(i64),
    MidTick,
    NaturalCents(i64),
}

struct Rung {
    anchor: Anchor,
    wait_secs: i64,
}

pub fn step_dynamic_market_state(
    state: &DynamicMarketState,
    quote: Option<DynamicMarketQuote>,
    now: NaiveDateTime,
    filled_qty: i32,
    options: DynamicMarketStepOptions,
) -> DynamicMarketOutcome {
    let mut state = state.clone();
    state.normalize_bounds();
    if options.leg_count > 0 {
        state.leg_count = options.leg_count;
    }
    if state.restart {
        let Some(quote) = quote.filter(quote_usable) else {
            return DynamicMarketOutcome {
                state,
                kind: DynamicMarketKind::MissingQuote,
            };
        };
        state.restart = false;
        state.stalled = false;
        state.max_rounds = state.max_rounds.saturating_add(1);
        let _seeded = ensure_benchmark(&mut state, quote, now);
        let direction = price_direction(quote.best, quote.worst);
        let spread0 = state.initial_spread.unwrap_or_else(|| spread_width(quote));
        let cap = penetration_cap(state.profile, spread0, state.leg_count);
        return finish_round(&mut state, quote, now, filled_qty, cap, direction, options);
    }

    if state.stalled && !options.force {
        state.phase_label = stalled_label(state.max_rounds);
        return DynamicMarketOutcome {
            state,
            kind: DynamicMarketKind::Wait,
        };
    }
    if options.force {
        state.stalled = false;
    }

    let unpriced = state.limit_price == Decimal::ZERO
        && state.initial_mid.is_none()
        && state.step_entered_at.is_none()
        && state.last_adjustment_time.is_none();

    let Some(quote) = quote.filter(quote_usable) else {
        return DynamicMarketOutcome {
            state,
            kind: DynamicMarketKind::MissingQuote,
        };
    };

    let seeded = ensure_benchmark(&mut state, quote, now);
    let direction = price_direction(quote.best, quote.worst);
    let spread0 = state.initial_spread.unwrap_or_else(|| spread_width(quote));
    let cap = penetration_cap(state.profile, spread0, state.leg_count);
    let ladder = ladder_for(&state, spread0, cap);
    if ladder.is_empty() {
        return DynamicMarketOutcome {
            state,
            kind: DynamicMarketKind::Wait,
        };
    }
    if seeded {
        state.step_index = map_legacy_index(&ladder, state.current_percentage);
        // A working order that already has a price only records the benchmark.
        // Replacing it on the takeover tick burns a broker replace.
        if !unpriced {
            let index = (state.step_index as usize).min(ladder.len() - 1);
            publish_rung(&mut state, &ladder[index], index);
            return DynamicMarketOutcome {
                state,
                kind: DynamicMarketKind::Wait,
            };
        }
    }
    let mut index = if seeded {
        state.step_index as usize
    } else {
        let reconciled = reconcile_step_index(&state, &ladder);
        state.step_index = reconciled;
        reconciled as usize
    };
    index = index.min(ladder.len() - 1);

    let mut drifted = false;
    if let Some(jump) = drift_index(&state, quote, &ladder, index) {
        if jump > index {
            index = jump;
            state.drift_accelerated = true;
            state.step_entered_at = Some(now);
            drifted = true;
        }
    }

    let elapsed = options.force || wait_elapsed(&state, &ladder, index, now);
    if !options.force && !seeded && !drifted && !elapsed {
        publish_rung(&mut state, &ladder[index], index);
        return DynamicMarketOutcome {
            state,
            kind: DynamicMarketKind::Wait,
        };
    }

    if elapsed && !seeded && !drifted {
        if index + 1 >= ladder.len() {
            return finish_round(&mut state, quote, now, filled_qty, cap, direction, options);
        }
        index += 1;
    }

    if rung_needs_sanity(ladder[index].anchor) && sanity_blocked(&mut state, quote, spread0) {
        let stay = index.saturating_sub(1);
        publish_rung(&mut state, &ladder[stay], stay);
        return DynamicMarketOutcome {
            state,
            kind: DynamicMarketKind::Wait,
        };
    }

    let hanging = state.limit_price;
    let mut target = price_anchor(
        ladder[index].anchor,
        quote,
        direction,
        cap,
        options.positive_price,
    );
    if !unpriced && retreats(target, hanging, direction) {
        publish_rung(&mut state, &ladder[index], index);
        state.step_entered_at = Some(now);
        return DynamicMarketOutcome {
            state,
            kind: DynamicMarketKind::Wait,
        };
    }
    if !unpriced && target == hanging {
        if elapsed || options.force {
            let mut advanced = false;
            for cand_idx in (index + 1)..ladder.len() {
                let anchor = ladder[cand_idx].anchor;
                if rung_needs_sanity(anchor) && sanity_blocked(&mut state, quote, spread0) {
                    publish_rung(&mut state, &ladder[index], index);
                    return DynamicMarketOutcome {
                        state,
                        kind: DynamicMarketKind::Wait,
                    };
                }
                let next_price =
                    price_anchor(anchor, quote, direction, cap, options.positive_price);
                if next_price != hanging && !retreats(next_price, hanging, direction) {
                    index = cand_idx;
                    target = next_price;
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                let nudged = clamp_to_cap(
                    hanging + tick(direction),
                    quote,
                    direction,
                    cap,
                    options.positive_price,
                );
                let past_worst =
                    (nudged - quote.worst) * Decimal::from(direction) > Decimal::ZERO;
                if past_worst && sanity_blocked(&mut state, quote, spread0) {
                    publish_rung(&mut state, &ladder[index], index);
                    return DynamicMarketOutcome {
                        state,
                        kind: DynamicMarketKind::Wait,
                    };
                }
                if nudged != hanging && !retreats(nudged, hanging, direction) {
                    target = nudged;
                }
            }
        } else {
            publish_rung(&mut state, &ladder[index], index);
            state.step_entered_at.get_or_insert(now);
            return DynamicMarketOutcome {
                state,
                kind: DynamicMarketKind::Wait,
            };
        }
    }

    publish_rung(&mut state, &ladder[index], index);
    state.limit_price = target;
    state.step_entered_at = Some(now);
    state.last_adjustment_time = Some(now);
    if target == hanging && !unpriced {
        return DynamicMarketOutcome {
            state,
            kind: DynamicMarketKind::Wait,
        };
    }
    state.replace_count = state.replace_count.saturating_add(1);
    DynamicMarketOutcome {
        state,
        kind: DynamicMarketKind::Replace,
    }
}

fn finish_round(
    state: &mut DynamicMarketState,
    quote: DynamicMarketQuote,
    now: NaiveDateTime,
    filled_qty: i32,
    cap: Decimal,
    direction: i32,
    options: DynamicMarketStepOptions,
) -> DynamicMarketOutcome {
    if state.round >= state.max_rounds {
        state.stalled = true;
        state.phase_label = stalled_label(state.max_rounds);
        state.last_adjustment_time = Some(now);
        return DynamicMarketOutcome {
            state: state.clone(),
            kind: DynamicMarketKind::MarkStalled,
        };
    }
    let spread0 = state.initial_spread.unwrap_or_else(|| spread_width(quote));
    if sanity_blocked(state, quote, spread0) {
        return DynamicMarketOutcome {
            state: state.clone(),
            kind: DynamicMarketKind::Wait,
        };
    }
    state.round = state.round.saturating_add(1);
    state.step_index = 0;
    state.drift_accelerated = false;
    let anchor = if state.round >= 3 {
        Anchor::NaturalCents(cents_of(cap).max(2))
    } else {
        Anchor::NaturalCents(1)
    };
    let hanging = state.limit_price;
    let mut target = price_anchor(anchor, quote, direction, cap, options.positive_price);
    if target == hanging {
        target = clamp_to_cap(
            hanging + tick(direction),
            quote,
            direction,
            cap,
            options.positive_price,
        );
    }
    if retreats(target, hanging, direction) {
        target = hanging;
    }
    state.limit_price = target;
    state.current_percentage = 1.0;
    state.interval_seconds = reroute_wait(state.profile);
    state.step_entered_at = Some(now);
    state.last_adjustment_time = Some(now);
    state.phase_label = format!("R{} reroute", state.round);
    if filled_qty > 0 && target == hanging {
        return DynamicMarketOutcome {
            state: state.clone(),
            kind: DynamicMarketKind::Wait,
        };
    }
    state.replace_count = state.replace_count.saturating_add(1);
    let kind = if filled_qty > 0 {
        DynamicMarketKind::Replace
    } else {
        DynamicMarketKind::Recreate
    };
    DynamicMarketOutcome {
        state: state.clone(),
        kind,
    }
}

fn ensure_benchmark(
    state: &mut DynamicMarketState,
    quote: DynamicMarketQuote,
    now: NaiveDateTime,
) -> bool {
    if state.initial_mid.is_some() && state.initial_spread.is_some() {
        return false;
    }
    state.initial_mid = Some(mid_price(quote));
    state.initial_spread = Some(spread_width(quote));
    state.step_entered_at.get_or_insert(now);
    true
}

fn quote_usable(quote: &DynamicMarketQuote) -> bool {
    quote.best != Decimal::ZERO || quote.worst != Decimal::ZERO
}

fn price_direction(best: Decimal, worst: Decimal) -> i32 {
    if worst >= best { 1 } else { -1 }
}

fn spread_width(quote: DynamicMarketQuote) -> Decimal {
    (quote.worst - quote.best).abs()
}

fn mid_price(quote: DynamicMarketQuote) -> Decimal {
    (quote.best + quote.worst) / Decimal::from(2)
}

fn tick(direction: i32) -> Decimal {
    Decimal::new(i64::from(direction), 2)
}

fn cents_of(price: Decimal) -> i64 {
    let cents = (price * Decimal::ONE_HUNDRED).round_dp(0).mantissa();
    i64::try_from(cents).unwrap_or(2).clamp(1, 4)
}

fn favorable_round(price: Decimal, direction: i32) -> Decimal {
    let cents = price * Decimal::ONE_HUNDRED;
    let rounded = if direction > 0 { cents.floor() } else { cents.ceil() };
    rounded / Decimal::ONE_HUNDRED
}

fn clamp_positive(price: Decimal, positive: bool) -> Decimal {
    if positive && price < Decimal::new(1, 2) {
        Decimal::new(1, 2)
    } else {
        price
    }
}

fn retreats(target: Decimal, hanging: Decimal, direction: i32) -> bool {
    let delta = target - hanging;
    if direction > 0 {
        delta < Decimal::ZERO
    } else {
        delta > Decimal::ZERO
    }
}

fn penetration_cap(profile: DynamicMarketProfile, spread0: Decimal, leg_count: u8) -> Decimal {
    let legs = if leg_count == 0 { 2 } else { leg_count };
    match profile {
        DynamicMarketProfile::Fast => {
            if legs >= 3 {
                Decimal::new(3, 2)
            } else {
                Decimal::new(2, 2)
            }
        }
        DynamicMarketProfile::Standard => {
            if spread0 < Decimal::new(6, 2) {
                Decimal::new(2, 2)
            } else {
                let raw = spread0 * Decimal::new(15, 2);
                let floored = (raw * Decimal::ONE_HUNDRED).floor() / Decimal::ONE_HUNDRED;
                floored.max(Decimal::new(2, 2)).min(Decimal::new(4, 2))
            }
        }
    }
}

fn ladder_for(state: &DynamicMarketState, spread0: Decimal, cap: Decimal) -> Vec<Rung> {
    if state.round <= 1 {
        let narrow = state.start_at_mid || spread0 <= Decimal::new(5, 2);
        return match (state.profile, narrow) {
            (DynamicMarketProfile::Fast, true) => vec![
                rung(Anchor::Percent(50), 10),
                rung(Anchor::MidTick, 8),
                rung(Anchor::Percent(100), 4),
                rung(Anchor::NaturalCents(1), 4),
                rung(Anchor::NaturalCents(2), 4),
            ],
            (DynamicMarketProfile::Fast, false) => vec![
                rung(Anchor::Percent(40), 8),
                rung(Anchor::Percent(50), 12),
                rung(Anchor::Percent(60), 8),
                rung(Anchor::Percent(75), 6),
                rung(Anchor::Percent(100), 4),
                rung(Anchor::NaturalCents(1), 4),
                rung(Anchor::NaturalCents(2), 4),
            ],
            (DynamicMarketProfile::Standard, true) => vec![
                rung(Anchor::Percent(50), 15),
                rung(Anchor::MidTick, 12),
                rung(Anchor::Percent(100), 6),
                rung(Anchor::NaturalCents(1), 5),
                rung(Anchor::NaturalCents(2), 5),
            ],
            (DynamicMarketProfile::Standard, false) => vec![
                rung(Anchor::Percent(40), 12),
                rung(Anchor::Percent(45), 12),
                rung(Anchor::Percent(50), 20),
                rung(Anchor::Percent(55), 15),
                rung(Anchor::Percent(60), 12),
                rung(Anchor::Percent(75), 8),
                rung(Anchor::Percent(100), 6),
                rung(Anchor::NaturalCents(2), 5),
            ],
        };
    }
    let wait = reroute_wait(state.profile);
    if state.round == 2 {
        vec![
            rung(Anchor::NaturalCents(1), wait),
            rung(Anchor::NaturalCents(2), wait),
        ]
    } else {
        vec![rung(Anchor::NaturalCents(cents_of(cap).max(2)), wait)]
    }
}

fn reroute_wait(profile: DynamicMarketProfile) -> i64 {
    match profile {
        DynamicMarketProfile::Fast => 4,
        DynamicMarketProfile::Standard => 5,
    }
}

fn rung(anchor: Anchor, wait_secs: i64) -> Rung {
    Rung { anchor, wait_secs }
}

fn anchor_rank(anchor: Anchor) -> f64 {
    match anchor {
        Anchor::Percent(percent) => percent as f64 / 100.0,
        Anchor::MidTick => 0.55,
        Anchor::NaturalCents(cents) => 1.0 + cents as f64 / 1000.0,
    }
}

fn map_legacy_index(ladder: &[Rung], current_percentage: f64) -> u16 {
    let mut index = 0usize;
    for (position, rung) in ladder.iter().enumerate() {
        if anchor_rank(rung.anchor) <= current_percentage + 1e-9 {
            index = position;
        }
    }
    index as u16
}

/// Keep an in-flight order on the rung its phase label names.
/// Percentage is only the fallback for rows that never stored a label.
/// Round 2+ ladders did not change shape, so their indexes stay put.
fn reconcile_step_index(state: &DynamicMarketState, ladder: &[Rung]) -> u16 {
    let index = (state.step_index as usize).min(ladder.len().saturating_sub(1));
    if state.round > 1 || ladder.is_empty() {
        return index as u16;
    }
    let expected = |rung: &Rung| format!("R{} {}", state.round, anchor_label(rung.anchor));
    if ladder
        .get(index)
        .is_some_and(|rung| state.phase_label == expected(rung))
    {
        return index as u16;
    }
    if let Some(matched) = ladder
        .iter()
        .position(|rung| state.phase_label == expected(rung))
    {
        return matched as u16;
    }
    map_legacy_index(ladder, state.current_percentage)
}

/// `initial_mid` stays fixed so slippage and the severe threshold keep accumulating.
/// Severe drift goes to 100% natural. A mild move below 60% stops at 60% or
/// mid+1tick and sets `drift_accelerated`, which then holds that defense rung
/// until the move becomes severe. Reaching 60% by the timer, without that flag,
/// still jumps to 100% natural on a later mild move.
fn drift_index(
    state: &DynamicMarketState,
    quote: DynamicMarketQuote,
    ladder: &[Rung],
    index: usize,
) -> Option<usize> {
    let mid0 = state.initial_mid?;
    let spread0 = state.initial_spread?;
    let direction = price_direction(quote.best, quote.worst);
    let adverse = (mid_price(quote) - mid0) * Decimal::from(direction);
    let (mild, severe) = drift_thresholds(state.profile, spread0);
    if adverse < mild {
        return None;
    }
    let front = ladder
        .get(index)
        .is_some_and(|rung| anchor_rank(rung.anchor) < 0.60);
    let target = if adverse >= severe {
        natural_index(ladder)
    } else if front {
        defense_index(ladder)
    } else if state.drift_accelerated {
        None
    } else {
        natural_index(ladder)
    };
    target.filter(|pos| *pos > index)
}

fn drift_thresholds(profile: DynamicMarketProfile, spread0: Decimal) -> (Decimal, Decimal) {
    match profile {
        DynamicMarketProfile::Fast => (
            Decimal::new(3, 2).max(spread0 * Decimal::new(45, 2)),
            Decimal::new(6, 2).max(spread0 * Decimal::new(80, 2)),
        ),
        DynamicMarketProfile::Standard => (
            Decimal::new(4, 2).max(spread0 * Decimal::new(50, 2)),
            Decimal::new(8, 2).max(spread0 * Decimal::new(90, 2)),
        ),
    }
}

fn defense_index(ladder: &[Rung]) -> Option<usize> {
    ladder
        .iter()
        .position(|rung| matches!(rung.anchor, Anchor::Percent(60)))
        .or_else(|| {
            ladder
                .iter()
                .position(|rung| matches!(rung.anchor, Anchor::MidTick))
        })
}

fn natural_index(ladder: &[Rung]) -> Option<usize> {
    ladder
        .iter()
        .position(|rung| matches!(rung.anchor, Anchor::Percent(100)))
}

fn wait_elapsed(
    state: &DynamicMarketState,
    ladder: &[Rung],
    index: usize,
    now: NaiveDateTime,
) -> bool {
    let wait = ladder.get(index).map(|rung| rung.wait_secs).unwrap_or(state.interval_seconds);
    match state.step_entered_at.or(state.last_adjustment_time) {
        Some(started) => now.signed_duration_since(started).num_seconds() >= wait,
        None => true,
    }
}

fn rung_needs_sanity(anchor: Anchor) -> bool {
    matches!(anchor, Anchor::NaturalCents(_))
}

fn sanity_blocked(state: &mut DynamicMarketState, quote: DynamicMarketQuote, spread0: Decimal) -> bool {
    let width = spread_width(quote);
    let abnormal = width > spread0 * Decimal::from(2) && width > Decimal::new(15, 2);
    if !abnormal {
        state.sanity_pause_count = 0;
        return false;
    }
    if state.sanity_pause_count < 3 {
        state.sanity_pause_count = state.sanity_pause_count.saturating_add(1);
    }
    true
}

fn publish_rung(state: &mut DynamicMarketState, rung: &Rung, index: usize) {
    state.step_index = index as u16;
    state.interval_seconds = rung.wait_secs;
    state.current_percentage = match rung.anchor {
        Anchor::Percent(percent) => percent as f64 / 100.0,
        Anchor::MidTick => 0.55,
        Anchor::NaturalCents(_) => 1.0,
    };
    state.phase_label = if state.round >= 2 {
        format!("R{} reroute", state.round)
    } else {
        format!("R{} {}", state.round, anchor_label(rung.anchor))
    };
}

fn anchor_label(anchor: Anchor) -> String {
    match anchor {
        Anchor::Percent(50) => "50% mid".to_string(),
        Anchor::Percent(100) => "100% natural".to_string(),
        Anchor::Percent(percent) => format!("{percent}%"),
        Anchor::MidTick => "mid+1tick".to_string(),
        Anchor::NaturalCents(cents) => format!("natural+{:.2}", cents as f64 / 100.0),
    }
}

fn stalled_label(_max_rounds: u8) -> String {
    "stalled".to_string()
}

fn clamp_to_cap(
    price: Decimal,
    quote: DynamicMarketQuote,
    direction: i32,
    cap: Decimal,
    positive: bool,
) -> Decimal {
    let edge = quote.worst + Decimal::from(direction) * cap;
    let limited = if direction > 0 {
        price.min(edge)
    } else {
        price.max(edge)
    };
    clamp_positive(favorable_round(limited, direction), positive)
}

/// Lock rung 0 from the live spread before the broker submit.
/// Narrow books and `start_at_mid` open at 50%. Wide books open at 40%.
pub fn install_submission_benchmark(
    state: &mut DynamicMarketState,
    quote: DynamicMarketQuote,
    now: NaiveDateTime,
    positive_price: bool,
    leg_count: u8,
) -> bool {
    if !quote_usable(&quote) {
        return false;
    }
    if state.initial_mid.is_some()
        && state.initial_spread.is_some()
        && state.step_entered_at.is_some()
    {
        return true;
    }
    state.normalize_bounds();
    if leg_count > 0 {
        state.leg_count = leg_count;
    }
    state.initial_mid = Some(mid_price(quote));
    state.initial_spread = Some(spread_width(quote));
    let spread0 = state.initial_spread.unwrap_or_else(|| spread_width(quote));
    let cap = penetration_cap(state.profile, spread0, state.leg_count);
    let ladder = ladder_for(state, spread0, cap);
    let Some(rung0) = ladder.first() else {
        return false;
    };
    let direction = price_direction(quote.best, quote.worst);
    state.limit_price = price_anchor(rung0.anchor, quote, direction, cap, positive_price);
    state.step_index = 0;
    state.step_entered_at = Some(now);
    state.last_adjustment_time = Some(now);
    state.stalled = false;
    state.restart = false;
    publish_rung(state, rung0, 0);
    true
}

fn price_anchor(
    anchor: Anchor,
    quote: DynamicMarketQuote,
    direction: i32,
    cap: Decimal,
    positive: bool,
) -> Decimal {
    let width = quote.worst - quote.best;
    let raw = match anchor {
        Anchor::Percent(percent) => {
            quote.best + width * Decimal::new(percent, 2)
        }
        Anchor::MidTick => mid_price(quote) + tick(direction),
        Anchor::NaturalCents(cents) => {
            let buffer = Decimal::new(cents, 2).min(cap);
            quote.worst + Decimal::from(direction) * buffer
        }
    };
    clamp_positive(favorable_round(raw, direction), positive)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    fn quote(best: i64, worst: i64) -> DynamicMarketQuote {
        DynamicMarketQuote {
            best: Decimal::new(best, 2),
            worst: Decimal::new(worst, 2),
        }
    }

    fn fresh(profile: DynamicMarketProfile, percentage: f64) -> DynamicMarketState {
        DynamicMarketState {
            current_percentage: percentage,
            start_percentage: percentage,
            profile,
            ..DynamicMarketState::default()
        }
    }

    #[test]
    fn wide_standard_starts_at_forty_and_narrow_fast_starts_at_mid() {
        let wide = step_dynamic_market_state(
            &fresh(DynamicMarketProfile::Standard, 0.4),
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:00"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(wide.kind, DynamicMarketKind::Replace);
        assert_eq!(wide.state.limit_price, Decimal::new(108, 2));
        assert_eq!(wide.state.phase_label, "R1 40%");
        assert_eq!(wide.state.initial_spread, Some(Decimal::new(20, 2)));

        let mut narrow = fresh(DynamicMarketProfile::Fast, 0.5);
        narrow.start_at_mid = true;
        let stepped = step_dynamic_market_state(
            &narrow,
            Some(quote(100, 140)),
            ts("2026-04-17 10:00:00"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(stepped.state.limit_price, Decimal::new(120, 2));
        assert_eq!(stepped.state.phase_label, "R1 50% mid");
    }

    #[test]
    fn sell_side_rounds_toward_the_seller_and_stays_positive() {
        let mut state = fresh(DynamicMarketProfile::Standard, 0.5);
        state.start_at_mid = true;
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(3, 1)),
            ts("2026-04-17 10:00:00"),
            0,
            DynamicMarketStepOptions {
                positive_price: true,
                ..DynamicMarketStepOptions::default()
            },
        );
        assert_eq!(stepped.state.limit_price, Decimal::new(2, 2));

        state.limit_price = Decimal::new(1, 2);
        state.initial_mid = Some(Decimal::new(2, 2));
        state.initial_spread = Some(Decimal::new(2, 2));
        state.step_index = 4;
        state.phase_label = "R1 natural+0.02".to_string();
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let penetrated = step_dynamic_market_state(
            &state,
            Some(quote(3, 1)),
            ts("2026-04-17 10:00:10"),
            0,
            DynamicMarketStepOptions {
                positive_price: true,
                force: true,
                ..DynamicMarketStepOptions::default()
            },
        );
        assert!(penetrated.state.limit_price >= Decimal::new(1, 2));
    }

    #[test]
    fn legacy_percentage_maps_forward_and_does_not_retreat() {
        let mut state = fresh(DynamicMarketProfile::Standard, 0.7);
        state.limit_price = Decimal::new(120, 2);
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:00"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(stepped.kind, DynamicMarketKind::Wait);
        assert_eq!(stepped.state.limit_price, Decimal::new(120, 2));
        assert!(stepped.state.current_percentage >= 0.6);
        assert!(stepped.state.initial_mid.is_some());
    }

    #[test]
    fn drift_jumps_to_natural_without_becoming_a_market_order() {
        let mut state = fresh(DynamicMarketProfile::Fast, 0.4);
        state.limit_price = Decimal::new(104, 2);
        state.initial_mid = Some(Decimal::new(105, 2));
        state.initial_spread = Some(Decimal::new(10, 2));
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(120, 140)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert!(stepped.state.drift_accelerated);
        assert_eq!(stepped.state.current_percentage, 1.0);
        assert_eq!(stepped.kind, DynamicMarketKind::Replace);
    }

    #[test]
    fn zero_fill_reroutes_high_and_partial_fill_replaces() {
        let mut state = fresh(DynamicMarketProfile::Fast, 1.0);
        state.limit_price = Decimal::new(122, 2);
        state.initial_mid = Some(Decimal::new(110, 2));
        state.initial_spread = Some(Decimal::new(20, 2));
        state.step_index = 6;
        state.phase_label = "R1 natural+0.02".to_string();
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let recreated = step_dynamic_market_state(
            &state,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:05"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(recreated.kind, DynamicMarketKind::Recreate);
        assert_eq!(recreated.state.round, 2);
        assert_eq!(recreated.state.phase_label, "R2 reroute");
        assert_eq!(recreated.state.interval_seconds, 4);
        assert!(recreated.state.limit_price >= Decimal::new(120, 2));

        state.limit_price = Decimal::new(110, 2);
        let replaced = step_dynamic_market_state(
            &state,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:05"),
            1,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(replaced.kind, DynamicMarketKind::Replace);
        assert_eq!(replaced.state.round, 2);
        assert!(replaced.state.limit_price > Decimal::new(110, 2));
    }

    #[test]
    fn third_round_stalls_on_the_resting_limit() {
        let mut state = fresh(DynamicMarketProfile::Standard, 1.0);
        state.round = 3;
        state.max_rounds = 3;
        state.limit_price = Decimal::new(123, 2);
        state.initial_mid = Some(Decimal::new(110, 2));
        state.initial_spread = Some(Decimal::new(20, 2));
        state.step_index = 0;
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let stalled = step_dynamic_market_state(
            &state,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:05"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(stalled.kind, DynamicMarketKind::MarkStalled);
        assert!(stalled.state.stalled);
        assert_eq!(stalled.state.phase_label, "stalled");
        assert_eq!(stalled.state.limit_price, Decimal::new(123, 2));
    }

    #[test]
    fn sanity_cap_pauses_before_penetration() {
        let mut state = fresh(DynamicMarketProfile::Fast, 1.0);
        state.limit_price = Decimal::new(110, 2);
        state.initial_mid = Some(Decimal::new(105, 2));
        state.initial_spread = Some(Decimal::new(10, 2));
        state.step_index = 4;
        state.phase_label = "R1 100% natural".to_string();
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let paused = step_dynamic_market_state(
            &state,
            Some(quote(100, 140)),
            ts("2026-04-17 10:00:05"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(paused.kind, DynamicMarketKind::Wait);
        assert_eq!(paused.state.sanity_pause_count, 1);
        assert_eq!(paused.state.round, 1);
    }

    #[test]
    fn missing_quote_does_not_invent_a_price() {
        let stepped = step_dynamic_market_state(
            &fresh(DynamicMarketProfile::Standard, 0.4),
            None,
            ts("2026-04-17 10:00:00"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(stepped.kind, DynamicMarketKind::MissingQuote);
        assert!(stepped.state.initial_mid.is_none());
    }

    #[test]
    fn seeded_working_order_waits_instead_of_replacing() {
        let mut state = fresh(DynamicMarketProfile::Standard, 0.0);
        state.limit_price = Decimal::new(10000, 2);
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(10000, 10200)),
            ts("2026-04-17 10:00:00"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(stepped.kind, DynamicMarketKind::Wait);
        assert_eq!(stepped.state.limit_price, Decimal::new(10000, 2));
        assert!(stepped.state.initial_mid.is_some());
        assert!(stepped.state.initial_spread.is_some());
    }

    #[test]
    fn same_price_nudge_stops_at_the_penetration_cap() {
        let mut state = fresh(DynamicMarketProfile::Fast, 1.0);
        state.limit_price = Decimal::new(112, 2);
        state.initial_mid = Some(Decimal::new(105, 2));
        state.initial_spread = Some(Decimal::new(10, 2));
        state.step_index = 5;
        state.phase_label = "R1 natural+0.01".to_string();
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        state.leg_count = 2;
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(100, 110)),
            ts("2026-04-17 10:00:05"),
            0,
            DynamicMarketStepOptions {
                leg_count: 2,
                ..DynamicMarketStepOptions::default()
            },
        );
        assert!(stepped.state.limit_price <= Decimal::new(112, 2));
        assert_ne!(stepped.kind, DynamicMarketKind::MarkStalled);
    }

    #[test]
    fn sanity_cap_stays_blocked_after_three_pauses() {
        let mut state = fresh(DynamicMarketProfile::Fast, 1.0);
        state.limit_price = Decimal::new(110, 2);
        state.initial_mid = Some(Decimal::new(105, 2));
        state.initial_spread = Some(Decimal::new(10, 2));
        state.step_index = 4;
        state.phase_label = "R1 100% natural".to_string();
        state.sanity_pause_count = 3;
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let paused = step_dynamic_market_state(
            &state,
            Some(quote(100, 140)),
            ts("2026-04-17 10:00:05"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(paused.kind, DynamicMarketKind::Wait);
        assert_eq!(paused.state.sanity_pause_count, 3);
        assert_eq!(paused.state.step_index, 4);
        assert_eq!(paused.state.phase_label, "R1 100% natural");
    }

    #[test]
    fn retreat_resets_the_rung_timer() {
        let mut state = fresh(DynamicMarketProfile::Fast, 0.4);
        state.limit_price = Decimal::new(112, 2);
        state.initial_mid = Some(Decimal::new(105, 2));
        state.initial_spread = Some(Decimal::new(10, 2));
        state.step_index = 0;
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let now = ts("2026-04-17 10:00:08");
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(100, 110)),
            now,
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(stepped.kind, DynamicMarketKind::Wait);
        assert_eq!(stepped.state.step_entered_at, Some(now));
        assert!(stepped.state.step_index > 0);
    }

    #[test]
    fn zero_net_with_a_rung_timer_is_a_real_price() {
        let mut state = fresh(DynamicMarketProfile::Standard, 0.5);
        state.start_at_mid = true;
        state.limit_price = Decimal::ZERO;
        state.initial_mid = Some(Decimal::ZERO);
        state.initial_spread = Some(Decimal::new(20, 2));
        state.step_index = 0;
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(-20, 0)),
            ts("2026-04-17 10:00:15"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(stepped.kind, DynamicMarketKind::Wait);
        assert_eq!(stepped.state.limit_price, Decimal::ZERO);
    }

    #[test]
    fn restart_emits_one_high_band_recreate() {
        let mut state = fresh(DynamicMarketProfile::Fast, 1.0);
        state.round = 3;
        state.max_rounds = 3;
        state.stalled = true;
        state.restart = true;
        state.limit_price = Decimal::new(123, 2);
        state.initial_mid = Some(Decimal::new(110, 2));
        state.initial_spread = Some(Decimal::new(20, 2));
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:05"),
            0,
            DynamicMarketStepOptions {
                leg_count: 2,
                ..DynamicMarketStepOptions::default()
            },
        );
        assert_eq!(stepped.kind, DynamicMarketKind::Recreate);
        assert!(!stepped.state.restart);
        assert!(!stepped.state.stalled);
        assert_eq!(stepped.state.round, 4);
        assert_eq!(stepped.state.max_rounds, 4);
        assert_eq!(stepped.state.phase_label, "R4 reroute");
        assert!(stepped.state.limit_price >= Decimal::new(120, 2));
    }

    #[test]
    fn submission_benchmark_prices_rung_zero() {
        let now = ts("2026-04-17 10:00:00");
        let mut wide = fresh(DynamicMarketProfile::Standard, 0.0);
        assert!(install_submission_benchmark(
            &mut wide,
            quote(10000, 10200),
            now,
            true,
            1,
        ));
        assert_eq!(wide.limit_price, Decimal::new(10080, 2));
        assert_eq!(wide.step_index, 0);
        assert_eq!(wide.step_entered_at, Some(now));
        assert_eq!(wide.phase_label, "R1 40%");

        let mut narrow = fresh(DynamicMarketProfile::Fast, 0.0);
        assert!(install_submission_benchmark(
            &mut narrow,
            quote(10000, 10004),
            now,
            true,
            1,
        ));
        assert_eq!(narrow.limit_price, Decimal::new(10002, 2));
        assert_eq!(narrow.phase_label, "R1 50% mid");

        let again = step_dynamic_market_state(
            &wide,
            Some(quote(10000, 10200)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions {
                positive_price: true,
                leg_count: 1,
                ..DynamicMarketStepOptions::default()
            },
        );
        assert_eq!(again.kind, DynamicMarketKind::Wait);
        assert_eq!(again.state.limit_price, Decimal::new(10080, 2));
    }

    fn working(profile: DynamicMarketProfile, percentage: f64) -> DynamicMarketState {
        let mut state = fresh(profile, percentage);
        state.initial_mid = Some(Decimal::new(110, 2));
        state.initial_spread = Some(Decimal::new(20, 2));
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        state
    }

    #[test]
    fn opening_and_mid_rungs_hold_for_their_full_dwell() {
        let start = ts("2026-04-17 10:00:00");
        let opened = step_dynamic_market_state(
            &fresh(DynamicMarketProfile::Standard, 0.4),
            Some(quote(100, 120)),
            start,
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(opened.state.phase_label, "R1 40%");
        assert_eq!(opened.state.interval_seconds, 12);
        assert_eq!(opened.state.limit_price, Decimal::new(108, 2));

        let early = step_dynamic_market_state(
            &opened.state,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:11"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(early.kind, DynamicMarketKind::Wait);
        assert_eq!(early.state.step_index, 0);
        assert_eq!(early.state.phase_label, "R1 40%");

        let next = step_dynamic_market_state(
            &opened.state,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:12"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(next.kind, DynamicMarketKind::Replace);
        assert_eq!(next.state.phase_label, "R1 45%");
        assert_eq!(next.state.interval_seconds, 12);
        assert_eq!(next.state.limit_price, Decimal::new(109, 2));

        let mut mid = opened.state;
        mid.step_index = 2;
        mid.current_percentage = 0.5;
        mid.phase_label = "R1 50% mid".to_string();
        mid.limit_price = Decimal::new(110, 2);
        mid.interval_seconds = 20;
        let held = step_dynamic_market_state(
            &mid,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:19"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(held.kind, DynamicMarketKind::Wait);
        assert_eq!(held.state.step_index, 2);
        assert_eq!(held.state.phase_label, "R1 50% mid");

        let advanced = step_dynamic_market_state(
            &mid,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:20"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(advanced.kind, DynamicMarketKind::Replace);
        assert_eq!(advanced.state.phase_label, "R1 55%");
        assert_eq!(advanced.state.interval_seconds, 15);
        assert_eq!(advanced.state.limit_price, Decimal::new(111, 2));

        let fast = step_dynamic_market_state(
            &fresh(DynamicMarketProfile::Fast, 0.4),
            Some(quote(100, 120)),
            start,
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(fast.state.interval_seconds, 8);
        let fast_early = step_dynamic_market_state(
            &fast.state,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:07"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(fast_early.kind, DynamicMarketKind::Wait);
        assert_eq!(fast_early.state.step_index, 0);
        assert_eq!(fast_early.state.phase_label, "R1 40%");
    }

    #[test]
    fn same_cent_rungs_skip_to_the_next_distinct_price() {
        let mut state = fresh(DynamicMarketProfile::Standard, 0.5);
        state.limit_price = Decimal::new(103, 2);
        state.initial_mid = Some(Decimal::new(103, 2));
        state.initial_spread = Some(Decimal::new(6, 2));
        state.step_index = 2;
        state.phase_label = "R1 50% mid".to_string();
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let stepped = step_dynamic_market_state(
            &state,
            Some(quote(100, 106)),
            ts("2026-04-17 10:00:20"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(stepped.kind, DynamicMarketKind::Replace);
        assert_eq!(stepped.state.limit_price, Decimal::new(104, 2));
        assert_eq!(stepped.state.phase_label, "R1 75%");
        assert_eq!(stepped.state.interval_seconds, 8);
        assert_eq!(stepped.state.step_index, 5);
    }

    #[test]
    fn drift_uses_two_thresholds_before_natural() {
        let mut calm = working(DynamicMarketProfile::Standard, 0.4);
        calm.limit_price = Decimal::new(108, 2);
        calm.phase_label = "R1 40%".to_string();
        let held = step_dynamic_market_state(
            &calm,
            Some(quote(109, 129)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert!(!held.state.drift_accelerated);
        assert_eq!(held.state.phase_label, "R1 40%");
        assert_eq!(held.state.limit_price, Decimal::new(108, 2));
        assert_eq!(held.kind, DynamicMarketKind::Wait);

        let mild = step_dynamic_market_state(
            &calm,
            Some(quote(110, 130)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert!(mild.state.drift_accelerated);
        assert_eq!(mild.state.phase_label, "R1 60%");
        assert_eq!(mild.state.current_percentage, 0.6);
        assert_eq!(mild.state.limit_price, Decimal::new(122, 2));
        assert_eq!(mild.kind, DynamicMarketKind::Replace);

        let severe = step_dynamic_market_state(
            &calm,
            Some(quote(118, 138)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert!(severe.state.drift_accelerated);
        assert_eq!(severe.state.phase_label, "R1 100% natural");
        assert_eq!(severe.state.current_percentage, 1.0);
        assert_eq!(severe.state.limit_price, Decimal::new(138, 2));

        let mut narrow = fresh(DynamicMarketProfile::Standard, 0.5);
        narrow.start_at_mid = true;
        narrow.limit_price = Decimal::new(102, 2);
        narrow.initial_mid = Some(Decimal::new(102, 2));
        narrow.initial_spread = Some(Decimal::new(4, 2));
        narrow.phase_label = "R1 50% mid".to_string();
        narrow.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let defended = step_dynamic_market_state(
            &narrow,
            Some(quote(104, 108)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert!(defended.state.drift_accelerated);
        assert_eq!(defended.state.phase_label, "R1 mid+1tick");
        assert_eq!(defended.state.current_percentage, 0.55);
        assert_eq!(defended.state.limit_price, Decimal::new(107, 2));
        assert_ne!(defended.state.current_percentage, 1.0);

        let mut fast = working(DynamicMarketProfile::Fast, 0.4);
        fast.initial_mid = Some(Decimal::new(105, 2));
        fast.initial_spread = Some(Decimal::new(10, 2));
        fast.limit_price = Decimal::new(104, 2);
        fast.phase_label = "R1 40%".to_string();
        let fast_mild = step_dynamic_market_state(
            &fast,
            Some(quote(105, 114)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert!(fast_mild.state.drift_accelerated);
        assert_eq!(fast_mild.state.phase_label, "R1 60%");
        assert_eq!(fast_mild.state.limit_price, Decimal::new(110, 2));
        assert_ne!(fast_mild.state.current_percentage, 1.0);

        let mut later = working(DynamicMarketProfile::Standard, 0.6);
        later.limit_price = Decimal::new(112, 2);
        later.step_index = 4;
        later.phase_label = "R1 60%".to_string();
        let later_mild = step_dynamic_market_state(
            &later,
            Some(quote(110, 130)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert!(later_mild.state.drift_accelerated);
        assert_eq!(later_mild.state.phase_label, "R1 100% natural");
        assert_eq!(later_mild.state.limit_price, Decimal::new(130, 2));
    }

    #[test]
    fn mild_drift_holds_the_wide_defense_rung_until_severe() {
        let mut standard = working(DynamicMarketProfile::Standard, 0.4);
        standard.limit_price = Decimal::new(108, 2);
        standard.phase_label = "R1 40%".to_string();
        let defended = step_dynamic_market_state(
            &standard,
            Some(quote(110, 130)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(defended.kind, DynamicMarketKind::Replace);
        assert!(defended.state.drift_accelerated);
        assert_eq!(defended.state.phase_label, "R1 60%");
        assert_eq!(defended.state.initial_mid, Some(Decimal::new(110, 2)));

        let held = step_dynamic_market_state(
            &defended.state,
            Some(quote(110, 130)),
            ts("2026-04-17 10:00:02"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(held.kind, DynamicMarketKind::Wait);
        assert_eq!(held.state.phase_label, "R1 60%");
        assert_eq!(held.state.limit_price, defended.state.limit_price);
        assert_eq!(held.state.initial_mid, Some(Decimal::new(110, 2)));

        let severe = step_dynamic_market_state(
            &defended.state,
            Some(quote(118, 138)),
            ts("2026-04-17 10:00:02"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(severe.kind, DynamicMarketKind::Replace);
        assert_eq!(severe.state.phase_label, "R1 100% natural");
        assert_eq!(severe.state.limit_price, Decimal::new(138, 2));

        let mut fast = working(DynamicMarketProfile::Fast, 0.4);
        fast.initial_mid = Some(Decimal::new(105, 2));
        fast.initial_spread = Some(Decimal::new(10, 2));
        fast.limit_price = Decimal::new(104, 2);
        fast.phase_label = "R1 40%".to_string();
        let fast_defended = step_dynamic_market_state(
            &fast,
            Some(quote(105, 114)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(fast_defended.state.phase_label, "R1 60%");
        assert!(fast_defended.state.drift_accelerated);
        let fast_held = step_dynamic_market_state(
            &fast_defended.state,
            Some(quote(105, 114)),
            ts("2026-04-17 10:00:02"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(fast_held.kind, DynamicMarketKind::Wait);
        assert_eq!(fast_held.state.phase_label, "R1 60%");
        assert_eq!(fast_held.state.limit_price, fast_defended.state.limit_price);

        let fast_severe = step_dynamic_market_state(
            &fast_defended.state,
            Some(quote(109, 117)),
            ts("2026-04-17 10:00:02"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(fast_severe.kind, DynamicMarketKind::Replace);
        assert_eq!(fast_severe.state.phase_label, "R1 100% natural");
        assert_eq!(fast_severe.state.limit_price, Decimal::new(117, 2));
    }

    #[test]
    fn widened_book_does_not_pierce_natural_from_a_same_price_scan() {
        let mut state = fresh(DynamicMarketProfile::Fast, 0.75);
        state.limit_price = Decimal::new(140, 2);
        state.initial_mid = Some(Decimal::new(120, 2));
        state.initial_spread = Some(Decimal::new(10, 2));
        state.step_index = 3;
        state.phase_label = "R1 75%".to_string();
        state.step_entered_at = Some(ts("2026-04-17 10:00:00"));
        let paused = step_dynamic_market_state(
            &state,
            Some(quote(100, 140)),
            ts("2026-04-17 10:00:06"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(paused.kind, DynamicMarketKind::Wait);
        assert_eq!(paused.state.phase_label, "R1 100% natural");
        assert_eq!(paused.state.limit_price, Decimal::new(140, 2));
        assert_eq!(paused.state.sanity_pause_count, 1);
        assert_eq!(paused.state.step_index, 4);
    }

    #[test]
    fn inflight_rung_follows_phase_label_onto_the_widened_ladder() {
        let mut stale = working(DynamicMarketProfile::Standard, 0.6);
        stale.limit_price = Decimal::new(112, 2);
        stale.step_index = 2;
        stale.phase_label = "R1 60%".to_string();
        let kept = step_dynamic_market_state(
            &stale,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(kept.kind, DynamicMarketKind::Wait);
        assert_eq!(kept.state.step_index, 4);
        assert_eq!(kept.state.phase_label, "R1 60%");
        assert_eq!(kept.state.limit_price, Decimal::new(112, 2));
        assert_eq!(kept.state.interval_seconds, 12);

        let mut natural = working(DynamicMarketProfile::Standard, 1.0);
        natural.limit_price = Decimal::new(122, 2);
        natural.step_index = 5;
        natural.phase_label = "R1 natural+0.02".to_string();
        let high = step_dynamic_market_state(
            &natural,
            Some(quote(100, 120)),
            ts("2026-04-17 10:00:01"),
            0,
            DynamicMarketStepOptions::default(),
        );
        assert_eq!(high.kind, DynamicMarketKind::Wait);
        assert_eq!(high.state.step_index, 7);
        assert_eq!(high.state.phase_label, "R1 natural+0.02");
        assert_eq!(high.state.limit_price, Decimal::new(122, 2));
    }
}
