import { clock as timeClock, session as timeSession } from '@alpaca/time';

import { fail } from './error';
import { refineBracketedRoot, roundHalfEven, scanRangeExtrema } from './numeric';
import { OptionStrategy } from './optionStrategy';
import { chanceOfProfit, polishedDirectedBreakEvens, type DirectedBreakEven } from './probability';
import type { RiskFreeRatePoint } from './rate';
import { quoteMidOptionValue } from './snapshot';
import type { Greeks, OptionPosition, OptionRight, OptionStrategyCurvePoint } from './types';

const COST_TOLERANCE = 0.01;
const ZERO_THETA = 1e-9;
const NODE_DEDUPE = 1e-8;
const DEFAULT_SCAN_STEP = 1;
const DEFAULT_TOLERANCE = 1e-6;
const DEFAULT_MAX_ITERATIONS = 80;
const DEFAULT_MAX_SAMPLES = 4096;
const DEFAULT_THETA_PRECISION_SECONDS = 3600;
const DEFAULT_THETA_ITERATIONS = 64;

export type AnalysisSection =
  | 'mark'
  | 'greeks'
  | 'break_even'
  | 'curve'
  | 'peak'
  | 'probability'
  | 'theta_zero';

export type SectionStatus = 'not_requested' | 'ok' | 'unavailable' | 'budget_exhausted';
export type ValueSource = 'mid' | 'model';
export type RangeStatus = 'roots' | 'no_root_profit' | 'no_root_loss' | 'no_root_flat';
export type ThetaZeroKind =
  | 'found'
  | 'initially_non_positive'
  | 'no_crossing'
  | 'budget_exhausted'
  | 'unavailable';

export type StrategyAnalysisInput = {
  positions: OptionPosition[];
  qty?: number;
  stock_qty?: number;
  stock_cashflow?: string | number;
  entry_cost?: number | null;
  cashflow?: string | number | null;
  underlying_price: number;
  evaluation_time: string;
  dividend_yield?: number | null;
  rate?: number | null;
  rate_curve?: Array<{ years: number; rate: number }> | null;
};

export type PriceSearch = {
  lower_bound: number;
  upper_bound: number;
  scan_step?: number | null;
  nodes?: number[];
  tolerance?: number | null;
  max_iterations?: number | null;
  max_samples?: number | null;
};

export type ProbabilityRequest = {
  horizon_days: number;
  volatility: number;
};

export type ThetaZeroSearch = {
  start_time: string;
  end_time: string;
  precision_seconds?: number | null;
  max_iterations?: number | null;
};

export type EvaluationRequest = {
  evaluation_time: string;
  iv_multiplier?: number | null;
};

export type StrategyAnalysisOptions = {
  sections?: AnalysisSection[];
  price_search?: PriceSearch | null;
  probability?: ProbabilityRequest | null;
  time_search?: ThetaZeroSearch | null;
  evaluations?: EvaluationRequest[];
};

export type DirectedRoot = {
  at: number;
  direction: 1 | -1;
};

export type ProfitRegion = {
  lower: number;
  upper: number;
  lower_is_window: boolean;
  upper_is_window: boolean;
};

export type BreakEvenSection = {
  points: DirectedRoot[];
  touches: number[];
  profit_regions: ProfitRegion[];
  range_status: RangeStatus;
  lower_truncated: boolean;
  upper_truncated: boolean;
  lower_unbounded: boolean;
  upper_unbounded: boolean;
};

export type PeakSection = {
  spot: number;
  pnl: number;
  at_boundary: boolean;
  window_limited: boolean;
  non_positive: boolean;
};

export type ThetaZeroSection = {
  kind: ThetaZeroKind;
  at?: string;
  days_from_start?: number;
  days_to_end?: number;
};

export type StrategyAnalysis = {
  cost: string;
  option_cost: string;
  stock_cost: string;
  value: string | null;
  value_source: ValueSource | null;
  model_value: string | null;
  mid_value: string | null;
  pnl: string | null;
  mark_status: SectionStatus;
  greeks: Greeks | null;
  greeks_status: SectionStatus;
  break_even: BreakEvenSection | null;
  break_even_status: SectionStatus;
  curve: OptionStrategyCurvePoint[] | null;
  curve_status: SectionStatus;
  peak: PeakSection | null;
  peak_status: SectionStatus;
  probability: number | null;
  probability_status: SectionStatus;
  theta_zero: ThetaZeroSection | null;
  theta_zero_status: SectionStatus;
  evaluations?: StrategyAnalysis[];
};

type ResolvedBook = {
  optionCost: number;
  stockCost: number;
  totalCost: number;
  stockCashflow: number;
};

function money(value: number): string {
  const rounded = roundHalfEven(value, 2);
  if (rounded === 0) {
    return '0.00';
  }
  return rounded.toFixed(2);
}

function parseMoney(value: string | number | null | undefined, name: string): number | null {
  if (value == null || value === '') {
    return null;
  }
  const parsed = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(parsed)) {
    fail('invalid_analysis_input', `${name} must be finite: ${value}`);
  }
  return roundHalfEven(parsed, 2);
}

function requestedSections(options: StrategyAnalysisOptions): Set<AnalysisSection> {
  const sections = options.sections ?? [];
  return new Set(sections.length === 0 ? ['mark', 'greeks'] : sections);
}

function legOptionCost(positions: readonly OptionPosition[], qty: number): number {
  let total = 0;
  for (const position of positions) {
    total += Number(position.avg_cost) * position.qty * 100;
  }
  return roundHalfEven(total * qty, 2);
}

// OptionStrategy always multiplies by a structure count. An omitted qty uses 1
// so the stored leg quantities are not scaled again.
function structureScale(qty: number | null | undefined): number {
  if (qty == null) {
    return 1;
  }
  if (!Number.isInteger(qty) || qty <= 0) {
    fail('invalid_analysis_input', `qty must be greater than zero: ${qty}`);
  }
  return qty;
}

function resolveBook(input: StrategyAnalysisInput): ResolvedBook {
  const scale = structureScale(input.qty);
  if (!Number.isFinite(input.underlying_price) || input.underlying_price < 0) {
    fail('invalid_analysis_input', `underlying_price must be non-negative: ${input.underlying_price}`);
  }
  if (input.rate != null && input.rate_curve != null) {
    fail('invalid_analysis_input', 'rate and rate_curve cannot both be set');
  }
  if (input.rate != null && (!Number.isFinite(input.rate) || input.rate <= 0 || input.rate >= 1)) {
    fail('invalid_analysis_input', `rate must be in (0, 1): ${input.rate}`);
  }
  if (input.rate_curve != null && input.rate_curve.length < 2) {
    fail('invalid_analysis_input', 'rate_curve must contain at least two points');
  }
  const stockCashflow = parseMoney(input.stock_cashflow, 'stock_cashflow') ?? 0;
  const stockCost = roundHalfEven(-stockCashflow, 2);
  const fromLegs = legOptionCost(input.positions, scale);
  const cashflow = parseMoney(input.cashflow, 'cashflow');
  const entryCost = input.entry_cost == null ? null : input.entry_cost;
  if (entryCost != null && !Number.isFinite(entryCost)) {
    fail('invalid_analysis_input', `entry_cost must be finite: ${entryCost}`);
  }
  let optionCost: number;
  let totalCost: number;
  if (cashflow != null && entryCost != null) {
    const fromCashflow = roundHalfEven(-cashflow, 2);
    const fromEntry = roundHalfEven(roundHalfEven(entryCost, 2) - stockCashflow, 2);
    const difference = roundHalfEven(Math.abs(fromCashflow - fromEntry), 2);
    if (difference > COST_TOLERANCE) {
      fail('inconsistent_strategy_cost', 'cashflow and entry_cost do not describe the same total cost');
    }
    totalCost = fromCashflow;
    optionCost = roundHalfEven(fromCashflow - stockCost, 2);
  } else if (cashflow != null) {
    totalCost = roundHalfEven(-cashflow, 2);
    optionCost = roundHalfEven(totalCost - stockCost, 2);
  } else if (entryCost != null) {
    optionCost = roundHalfEven(entryCost, 2);
    totalCost = roundHalfEven(optionCost + stockCost, 2);
  } else {
    optionCost = fromLegs;
    totalCost = roundHalfEven(fromLegs + stockCost, 2);
  }
  return { optionCost, stockCost, totalCost, stockCashflow };
}

function buildStrategy(input: StrategyAnalysisInput, book: ResolvedBook): OptionStrategy {
  const strategy = OptionStrategy.prepare({
    positions: input.positions,
    qty: structureScale(input.qty),
    evaluation_time: input.evaluation_time.trim(),
    entry_cost: book.optionCost,
    dividend_yield: input.dividend_yield ?? null,
  });
  strategy.cost = book.optionCost;
  strategy.stock_qty = input.stock_qty ?? 0;
  strategy.stock_cashflow = book.stockCashflow;
  strategy.cashflow = -book.totalCost;
  strategy.underlying_price = input.underlying_price;
  if (input.rate_curve != null) {
    strategy.useRateCurve(input.rate_curve);
  } else if (input.rate != null) {
    const curve: RiskFreeRatePoint[] = [
      { years: 1 / 12, rate: input.rate },
      { years: 30, rate: input.rate },
    ];
    strategy.useRateCurve(curve);
  }
  return strategy;
}

function emptyAnalysis(book: ResolvedBook): StrategyAnalysis {
  return {
    cost: money(book.totalCost),
    option_cost: money(book.optionCost),
    stock_cost: money(book.stockCost),
    value: null,
    value_source: null,
    model_value: null,
    mid_value: null,
    pnl: null,
    mark_status: 'not_requested',
    greeks: null,
    greeks_status: 'not_requested',
    break_even: null,
    break_even_status: 'not_requested',
    curve: null,
    curve_status: 'not_requested',
    peak: null,
    peak_status: 'not_requested',
    probability: null,
    probability_status: 'not_requested',
    theta_zero: null,
    theta_zero_status: 'not_requested',
  };
}

function fillMark(
  analysis: StrategyAnalysis,
  input: StrategyAnalysisInput,
  strategy: OptionStrategy,
  book: ResolvedBook,
): void {
  const model = roundHalfEven(strategy.markValueAt(input.underlying_price), 2);
  const stockValue = roundHalfEven((input.stock_qty ?? 0) * input.underlying_price, 2);
  const optionMid = quoteMidOptionValue(input.positions, structureScale(input.qty));
  const mid = optionMid == null ? null : roundHalfEven(optionMid + stockValue, 2);
  const regular = timeSession.isRegularSessionAt(input.evaluation_time.trim());
  const source: ValueSource = regular && mid != null ? 'mid' : 'model';
  const value = source === 'mid' && mid != null ? mid : model;
  analysis.model_value = money(model);
  analysis.mid_value = mid == null ? null : money(mid);
  analysis.value = money(value);
  analysis.value_source = source;
  analysis.pnl = money(roundHalfEven(value - book.totalCost, 2));
  analysis.mark_status = 'ok';
}

function validateSearch(search: PriceSearch): { step: number; tolerance: number; maxIterations: number; maxSamples: number } {
  if (!Number.isFinite(search.lower_bound) || search.lower_bound < 0) {
    fail('invalid_analysis_input', `lower_bound must be non-negative: ${search.lower_bound}`);
  }
  if (!Number.isFinite(search.upper_bound) || search.lower_bound >= search.upper_bound) {
    fail('invalid_analysis_input', `lower_bound must be less than upper_bound: ${search.lower_bound} >= ${search.upper_bound}`);
  }
  const step = search.scan_step ?? DEFAULT_SCAN_STEP;
  if (!Number.isFinite(step) || step <= 0) {
    fail('invalid_analysis_input', `scan_step must be positive: ${step}`);
  }
  const tolerance = search.tolerance ?? DEFAULT_TOLERANCE;
  if (!Number.isFinite(tolerance) || tolerance <= 0) {
    fail('invalid_analysis_input', `tolerance must be positive: ${tolerance}`);
  }
  const maxIterations = search.max_iterations ?? DEFAULT_MAX_ITERATIONS;
  if (!Number.isInteger(maxIterations) || maxIterations <= 0) {
    fail('invalid_analysis_input', 'max_iterations must be greater than zero');
  }
  const maxSamples = search.max_samples ?? DEFAULT_MAX_SAMPLES;
  if (!Number.isInteger(maxSamples) || maxSamples <= 0) {
    fail('invalid_analysis_input', 'max_samples must be greater than zero');
  }
  return { step, tolerance, maxIterations, maxSamples };
}

function sampleCount(lower: number, upper: number, step: number): number {
  return Math.ceil((upper - lower) / step) + 1;
}

function optionRightOf(position: OptionPosition): OptionRight {
  return position.option_right ?? position.snapshot.contract.option_right;
}

function searchNodes(strategy: OptionStrategy, search: PriceSearch, step: number): number[] {
  const nodes = [search.lower_bound, search.upper_bound, ...(search.nodes ?? [])];
  for (const position of strategy.positions) {
    const strike = position.strike;
    if (strike != null && strike >= search.lower_bound && strike <= search.upper_bound) {
      nodes.push(strike);
    }
  }
  let spot = search.lower_bound;
  while (spot < search.upper_bound) {
    spot = Math.min(spot + step, search.upper_bound);
    nodes.push(spot);
  }
  const finite = nodes.filter((value) => Number.isFinite(value)).sort((left, right) => left - right);
  const unique: number[] = [];
  for (const value of finite) {
    const previous = unique[unique.length - 1];
    if (previous == null || Math.abs(previous - value) > NODE_DEDUPE) {
      unique.push(value);
    }
  }
  return unique;
}

function deltaAt(strategy: OptionStrategy, price: number): number | null {
  try {
    const delta = strategy.greeksAt(price).delta;
    return Number.isFinite(delta) ? delta : null;
  } catch {
    return null;
  }
}

/** A slope sign change inside one sample can hide two roots. Insert that turning price. */
function turningNodes(
  strategy: OptionStrategy,
  nodes: readonly number[],
  tolerance: number,
  maxIterations: number,
): number[] {
  const extra: number[] = [];
  for (let index = 1; index < nodes.length; index += 1) {
    const left = nodes[index - 1]!;
    const right = nodes[index]!;
    if (!(right > left + tolerance)) {
      continue;
    }
    const leftDelta = deltaAt(strategy, left);
    const rightDelta = deltaAt(strategy, right);
    if (leftDelta == null || rightDelta == null) {
      continue;
    }
    if (Math.abs(leftDelta) <= tolerance || Math.abs(rightDelta) <= tolerance || Math.sign(leftDelta) === Math.sign(rightDelta)) {
      continue;
    }
    try {
      extra.push(refineBracketedRoot(
        left,
        right,
        (price) => {
          const delta = deltaAt(strategy, price);
          if (delta == null) {
            fail('invalid_analysis_input', 'delta is unavailable while refining a turning point');
          }
          return delta;
        },
        Math.max(tolerance, 1e-8),
        maxIterations,
      ));
    } catch {
      // This span did not converge. The coarse samples still stand.
    }
  }
  return extra;
}

function mergeNodes(nodes: readonly number[], extra: readonly number[]): number[] {
  const finite = [...nodes, ...extra].filter((value) => Number.isFinite(value)).sort((left, right) => left - right);
  const unique: number[] = [];
  for (const value of finite) {
    const previous = unique[unique.length - 1];
    if (previous == null || Math.abs(previous - value) > NODE_DEDUPE) {
      unique.push(value);
    }
  }
  return unique;
}

function terminalSlopes(strategy: OptionStrategy): { lower: number; upper: number } {
  let callQty = 0;
  let putQty = 0;
  for (const position of strategy.positions) {
    const signed = position.qty * strategy.qty;
    if (optionRightOf(position) === 'call') {
      callQty += signed;
    } else {
      putQty += signed;
    }
  }
  return {
    lower: strategy.stock_qty - 100 * putQty,
    upper: strategy.stock_qty + 100 * callQty,
  };
}

function profitRegions(
  strategy: OptionStrategy,
  search: PriceSearch,
  points: DirectedRoot[],
  tolerance: number,
): { regions: ProfitRegion[]; status: RangeStatus } {
  if (points.length === 0) {
    const mid = (search.lower_bound + search.upper_bound) / 2;
    const pnl = strategy.pnlAt(mid);
    const status: RangeStatus = pnl > tolerance
      ? 'no_root_profit'
      : pnl < -tolerance
        ? 'no_root_loss'
        : 'no_root_flat';
    return {
      status,
      regions: status === 'no_root_profit'
        ? [{
          lower: search.lower_bound,
          upper: search.upper_bound,
          lower_is_window: true,
          upper_is_window: true,
        }]
        : [],
    };
  }

  const regions: ProfitRegion[] = [];
  let inProfit = points[0]!.direction < 0;
  let left = search.lower_bound;
  for (const point of points) {
    if (inProfit && point.at > left) {
      regions.push({
        lower: left,
        upper: point.at,
        lower_is_window: Math.abs(left - search.lower_bound) <= NODE_DEDUPE,
        upper_is_window: false,
      });
    }
    inProfit = !inProfit;
    left = point.at;
  }
  if (inProfit && search.upper_bound > left) {
    regions.push({
      lower: left,
      upper: search.upper_bound,
      lower_is_window: false,
      upper_is_window: true,
    });
  }
  return { regions, status: 'roots' };
}

function breakEvenSection(strategy: OptionStrategy, search: PriceSearch): BreakEvenSection {
  const { step, tolerance, maxIterations, maxSamples } = validateSearch(search);
  const sampled = searchNodes(strategy, search, step);
  const nodes = mergeNodes(sampled, turningNodes(strategy, sampled, tolerance, maxIterations));
  if (nodes.length > maxSamples) {
    fail('invalid_analysis_input', 'break-even node count exceeds max_samples; raise the budget or widen scan_step');
  }
  const touches = nodes.filter((price) => Math.abs(strategy.pnlAt(price)) <= tolerance);
  const directed = polishedDirectedBreakEvens({
    prices: nodes,
    evaluate: (price) => {
      try {
        return strategy.pnlAt(price);
      } catch {
        return Number.NaN;
      }
    },
    tolerance,
    maxIterations,
  });
  const duplicateGap = Math.max(tolerance, 1e-4);
  const points: DirectedRoot[] = [];
  for (const point of directed) {
    const previous = points[points.length - 1];
    if (previous != null && Math.abs(previous.at - point.at) <= duplicateGap) {
      continue;
    }
    points.push({ at: point.at, direction: point.direction });
  }
  const { regions, status } = profitRegions(strategy, search, points, tolerance);
  const slopes = terminalSlopes(strategy);
  return {
    points,
    touches,
    profit_regions: regions,
    range_status: status,
    lower_truncated: regions.some((region) => region.lower_is_window),
    upper_truncated: regions.some((region) => region.upper_is_window),
    lower_unbounded: slopes.lower < 0,
    upper_unbounded: slopes.upper > 0,
  };
}

function refinedExtrema(strategy: OptionStrategy, search: PriceSearch): {
  maxSpot: number;
  maxValue: number;
  minSpot: number;
  minValue: number;
  tolerance: number;
} {
  const { step, tolerance, maxIterations } = validateSearch(search);
  const coarse = scanRangeExtrema(search.lower_bound, search.upper_bound, step, (price) => strategy.pnlAt(price));
  let maxSpot = coarse.maxSpot;
  let maxValue = coarse.maxValue;
  let minSpot = coarse.minSpot;
  let minValue = coarse.minValue;
  const sampled = searchNodes(strategy, search, step);
  for (const price of turningNodes(strategy, sampled, tolerance, maxIterations)) {
    const value = strategy.pnlAt(price);
    if (value > maxValue) {
      maxSpot = price;
      maxValue = value;
    }
    if (value < minValue) {
      minSpot = price;
      minValue = value;
    }
  }
  return { maxSpot, maxValue, minSpot, minValue, tolerance };
}

function peakSection(strategy: OptionStrategy, search: PriceSearch): PeakSection {
  const extrema = refinedExtrema(strategy, search);
  return {
    spot: extrema.maxSpot,
    pnl: extrema.maxValue,
    at_boundary: Math.abs(extrema.maxSpot - search.lower_bound) <= extrema.tolerance
      || Math.abs(extrema.maxSpot - search.upper_bound) <= extrema.tolerance,
    window_limited: true,
    non_positive: extrema.maxValue <= extrema.tolerance,
  };
}

function fillProbability(
  analysis: StrategyAnalysis,
  input: StrategyAnalysisInput,
  options: StrategyAnalysisOptions,
  strategy: OptionStrategy,
  search: PriceSearch,
  section: BreakEvenSection,
): void {
  const request = options.probability;
  if (request == null
    || !Number.isFinite(request.horizon_days)
    || request.horizon_days <= 0
    || !Number.isFinite(request.volatility)
    || request.volatility <= 0
    || input.underlying_price <= 0) {
    analysis.probability_status = 'unavailable';
    return;
  }
  const extrema = refinedExtrema(strategy, search);
  analysis.probability = chanceOfProfit({
    spot: input.underlying_price,
    days: request.horizon_days,
    volatility: request.volatility,
    breakevens: section.points,
    maxProfit: extrema.maxValue,
    minPnl: extrema.minValue,
    pnlWithoutBreakEven: strategy.pnlAt(input.underlying_price),
  });
  analysis.probability_status = 'ok';
}

export function findThetaZero(
  search: ThetaZeroSearch,
  thetaAt: (timestamp: string) => number | null,
): ThetaZeroSection {
  const precision = search.precision_seconds ?? DEFAULT_THETA_PRECISION_SECONDS;
  const maxIterations = search.max_iterations ?? DEFAULT_THETA_ITERATIONS;
  if (!Number.isInteger(precision) || precision <= 0) {
    fail('invalid_analysis_input', `precision_seconds must be positive: ${precision}`);
  }
  if (!Number.isInteger(maxIterations) || maxIterations <= 0) {
    fail('invalid_analysis_input', 'max_iterations must be greater than zero');
  }
  let startTime: string;
  let endTime: string;
  try {
    startTime = timeClock.parseTimestamp(search.start_time.trim());
    endTime = timeClock.parseTimestamp(search.end_time.trim());
  } catch (error) {
    fail('invalid_analysis_input', `invalid theta zero time: ${String(error)}`);
  }
  const startUnix = timeClock.unixSeconds(startTime);
  const endUnix = timeClock.unixSeconds(endTime);
  if (endUnix <= startUnix) {
    return { kind: 'no_crossing' };
  }
  const read = (timestamp: string): number | null => {
    const theta = thetaAt(timestamp);
    return theta != null && Number.isFinite(theta) ? theta : null;
  };
  const initial = read(startTime);
  if (initial == null) {
    return { kind: 'unavailable' };
  }
  if (initial <= ZERO_THETA) {
    return { kind: 'initially_non_positive' };
  }
  const ending = read(endTime);
  if (ending == null) {
    return { kind: 'unavailable' };
  }
  if (ending >= -ZERO_THETA) {
    return { kind: 'no_crossing' };
  }
  let low = startUnix;
  let high = endUnix;
  let iterations = 0;
  while (high - low > precision) {
    if (iterations >= maxIterations) {
      return { kind: 'budget_exhausted' };
    }
    iterations += 1;
    const mid = low + Math.trunc((high - low) / 2);
    const theta = read(timeClock.fromUnixSeconds(mid));
    if (theta == null) {
      return { kind: 'unavailable' };
    }
    if (theta > ZERO_THETA) {
      low = mid;
    } else {
      high = mid;
    }
  }
  const at = timeClock.fromUnixSeconds(high);
  const daysFromStart = timeClock.fractionalDaysBetween(startTime, at);
  const daysToEnd = timeClock.fractionalDaysBetween(at, endTime);
  if (daysFromStart == null || daysToEnd == null) {
    fail('invalid_analysis_input', 'theta zero day count failed');
  }
  return {
    kind: 'found',
    at,
    days_from_start: daysFromStart,
    days_to_end: daysToEnd,
  };
}

function analyzeInner(
  input: StrategyAnalysisInput,
  options: StrategyAnalysisOptions,
  includeEvaluations: boolean,
): StrategyAnalysis {
  const requested = requestedSections(options);
  const wants = (section: AnalysisSection) => requested.has(section);
  const book = resolveBook(input);
  const strategy = buildStrategy(input, book);
  const analysis = emptyAnalysis(book);
  if (wants('mark')) {
    fillMark(analysis, input, strategy, book);
  }
  if (wants('greeks')) {
    analysis.greeks = strategy.greeksAt(input.underlying_price);
    analysis.greeks_status = 'ok';
  }
  if (wants('break_even') || wants('curve') || wants('peak') || wants('probability')) {
    const search = options.price_search;
    if (search == null) {
      fail('invalid_analysis_input', 'price_search is required for break_even, curve, peak, or probability');
    }
    const limits = validateSearch(search);
    if (sampleCount(search.lower_bound, search.upper_bound, limits.step) > limits.maxSamples) {
      if (wants('break_even')) {
        analysis.break_even_status = 'budget_exhausted';
      }
      if (wants('curve')) {
        analysis.curve_status = 'budget_exhausted';
      }
      if (wants('peak')) {
        analysis.peak_status = 'budget_exhausted';
      }
      if (wants('probability')) {
        analysis.probability_status = 'budget_exhausted';
      }
    } else {
      if (wants('break_even') || wants('probability')) {
        const section = breakEvenSection(strategy, search);
        if (wants('probability')) {
          fillProbability(analysis, input, options, strategy, search, section);
        }
        if (wants('break_even')) {
          analysis.break_even = section;
          analysis.break_even_status = 'ok';
        }
      }
      if (wants('curve')) {
        analysis.curve = strategy.sampleCurve({
          lower_bound: search.lower_bound,
          upper_bound: search.upper_bound,
          step: limits.step,
        });
        analysis.curve_status = 'ok';
      }
      if (wants('peak')) {
        analysis.peak = peakSection(strategy, search);
        analysis.peak_status = 'ok';
      }
    }
  }
  if (wants('theta_zero')) {
    const search = options.time_search;
    if (search == null) {
      fail('invalid_analysis_input', 'time_search is required for theta_zero');
    }
    const section = findThetaZero(search, (timestamp) => {
      try {
        const timed: StrategyAnalysisInput = { ...input, evaluation_time: timestamp, positions: input.positions.map((position) => ({ ...position, snapshot: { ...position.snapshot } })) };
        return buildStrategy(timed, book).greeksAt(input.underlying_price).theta;
      } catch {
        return null;
      }
    });
    analysis.theta_zero = section;
    analysis.theta_zero_status = section.kind === 'budget_exhausted'
      ? 'budget_exhausted'
      : section.kind === 'unavailable'
        ? 'unavailable'
        : 'ok';
  }
  if (includeEvaluations) {
    const evaluations: StrategyAnalysis[] = [];
    for (const evaluation of options.evaluations ?? []) {
      const positions = input.positions.map((position) => ({
        ...position,
        snapshot: {
          ...position.snapshot,
          implied_volatility: evaluation.iv_multiplier == null || position.snapshot.implied_volatility == null
            ? position.snapshot.implied_volatility
            : position.snapshot.implied_volatility * evaluation.iv_multiplier,
        },
      }));
      if (evaluation.iv_multiplier != null && (!Number.isFinite(evaluation.iv_multiplier) || evaluation.iv_multiplier <= 0)) {
        fail('invalid_analysis_input', `iv_multiplier must be positive: ${evaluation.iv_multiplier}`);
      }
      evaluations.push(analyzeInner(
        { ...input, positions, evaluation_time: evaluation.evaluation_time },
        { ...options, evaluations: [] },
        false,
      ));
    }
    if (evaluations.length > 0) {
      analysis.evaluations = evaluations;
    }
  }
  return analysis;
}

export function analyzeStrategy(
  input: StrategyAnalysisInput,
  options: StrategyAnalysisOptions = {},
): StrategyAnalysis {
  return analyzeInner(input, options, true);
}
