import { fail } from './error';
import { normalCdf } from './numeric';

function ensureFinite(name: string, value: number): void {
  if (!Number.isFinite(value)) {
    fail('invalid_probability_input', `${name} must be finite: ${value}`);
  }
}

function ensurePositive(name: string, value: number): void {
  ensureFinite(name, value);
  if (value <= 0) {
    fail('invalid_probability_input', `${name} must be greater than zero: ${value}`);
  }
}

export function expiryProbabilityInRange(input: {
  spot: number;
  lowerPrice: number;
  upperPrice: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
}): number {
  ensurePositive('spot', input.spot);
  ensurePositive('lowerPrice', input.lowerPrice);
  ensurePositive('upperPrice', input.upperPrice);
  ensurePositive('years', input.years);
  ensureFinite('rate', input.rate);
  ensureFinite('dividendYield', input.dividendYield);
  ensurePositive('volatility', input.volatility);
  if (input.lowerPrice >= input.upperPrice) {
    fail('invalid_probability_input', `lowerPrice must be less than upperPrice: ${input.lowerPrice} >= ${input.upperPrice}`);
  }

  const sigmaSqrtT = input.volatility * Math.sqrt(input.years);
  const zLower = Math.log(input.spot / input.lowerPrice) / sigmaSqrtT;
  const zUpper = Math.log(input.spot / input.upperPrice) / sigmaSqrtT;

  return normalCdf(zLower) - normalCdf(zUpper);
}

/** +1 means PnL turns from loss to profit as price rises. -1 is the other way. */
export type DirectedBreakEven = {
  at: number;
  direction: 1 | -1;
};

export type ChanceTarget = {
  above: number;
  below: number;
};

function pnlSign(value: number): -1 | 0 | 1 {
  if (value > 0) {
    return 1;
  }
  if (value < 0) {
    return -1;
  }
  return 0;
}

function pushDirected(roots: DirectedBreakEven[], at: number, direction: 1 | -1): void {
  if (!Number.isFinite(at)) {
    return;
  }
  const previous = roots[roots.length - 1];
  if (previous != null && Math.abs(previous.at - at) <= 1e-8) {
    return;
  }
  roots.push({ at, direction });
}

/**
 * Zero-drift chance the terminal price is above or below `target`.
 * `days` is divided by 365. Rate and dividend are not used.
 */
export function chanceOfTarget(input: {
  spot: number;
  target: number;
  days: number;
  volatility: number;
}): ChanceTarget {
  ensurePositive('spot', input.spot);
  ensureFinite('target', input.target);
  ensureFinite('days', input.days);
  ensureFinite('volatility', input.volatility);
  if (input.target < 0) {
    return { above: 1, below: 0 };
  }
  if (input.days <= 0) {
    if (input.spot > input.target) {
      return { above: 1, below: 0 };
    }
    if (input.target > input.spot) {
      return { above: 0, below: 1 };
    }
    return { above: 0.5, below: 0.5 };
  }
  ensurePositive('volatility', input.volatility);
  const denominator = input.volatility * Math.sqrt(input.days / 365);
  const below = normalCdf(Math.log(input.target / input.spot) / denominator);
  return { above: 1 - below, below };
}

/**
 * Probability of profit for any number of breakevens.
 * Each point adds or removes the lognormal mass below it, following its direction.
 * A non-positive best PnL is 0. A worst PnL that is still non-negative is 1.
 */
export function chanceOfProfit(input: {
  spot: number;
  days: number;
  volatility: number;
  breakevens: readonly DirectedBreakEven[];
  maxProfit: number;
  minPnl: number;
  pnlWithoutBreakEven: number;
}): number {
  if (Number.isNaN(input.maxProfit) || Number.isNaN(input.minPnl)) {
    fail('invalid_probability_input', 'maxProfit and minPnl must be finite or infinite');
  }
  ensureFinite('pnlWithoutBreakEven', input.pnlWithoutBreakEven);
  if (!(input.maxProfit > 0)) {
    return 0;
  }
  if (!(input.minPnl < 0)) {
    return 1;
  }
  if (input.breakevens.length === 0) {
    return input.pnlWithoutBreakEven > 0 ? 1 : 0;
  }

  const ordered = [...input.breakevens].sort((left, right) => left.at - right.at);
  const unique: DirectedBreakEven[] = [];
  for (const item of ordered) {
    ensureFinite('breakeven', item.at);
    if (item.direction !== 1 && item.direction !== -1) {
      fail('invalid_probability_input', `direction must be 1 or -1: ${item.direction}`);
    }
    pushDirected(unique, item.at, item.direction);
  }
  const last = unique[unique.length - 1];
  let share = last != null && last.direction > 0 ? 1 : 0;
  for (const item of unique) {
    share -= chanceOfTarget({
      spot: input.spot,
      target: item.at,
      days: input.days,
      volatility: input.volatility,
    }).below * item.direction;
  }
  return share;
}

/** Linear roots between samples. Each sign change keeps its direction. */
export function directedBreakEvens(input: {
  prices: readonly number[];
  pnls: readonly number[];
}): DirectedBreakEven[] {
  if (input.prices.length !== input.pnls.length) {
    fail(
      'invalid_probability_input',
      `prices and pnls must have the same length: ${input.prices.length} != ${input.pnls.length}`,
    );
  }
  const roots: DirectedBreakEven[] = [];
  for (let index = 1; index < input.prices.length; index += 1) {
    const leftPrice = input.prices[index - 1] ?? Number.NaN;
    const rightPrice = input.prices[index] ?? Number.NaN;
    const left = input.pnls[index - 1] ?? Number.NaN;
    const right = input.pnls[index] ?? Number.NaN;
    if (!Number.isFinite(leftPrice) || !Number.isFinite(rightPrice) || !Number.isFinite(left) || !Number.isFinite(right) || !(rightPrice > leftPrice)) {
      continue;
    }
    const leftSign = pnlSign(left);
    const rightSign = pnlSign(right);
    if (leftSign === rightSign) {
      continue;
    }
    const span = right - left;
    const at = span === 0 ? leftPrice : leftPrice - (left * (rightPrice - leftPrice)) / span;
    pushDirected(roots, at, leftSign < rightSign ? 1 : -1);
  }
  return roots;
}

function bisectRoot(
  evaluate: (price: number) => number,
  left: number,
  right: number,
  tolerance: number,
  maxIterations: number,
): number {
  let lower = left;
  let upper = right;
  let lowerValue = evaluate(lower);
  const upperValue = evaluate(upper);
  if (!Number.isFinite(lowerValue) || !Number.isFinite(upperValue) || lowerValue * upperValue > 0) {
    const span = upperValue - lowerValue;
    if (span === 0) {
      return (lower + upper) / 2;
    }
    return lower - (lowerValue * (upper - lower)) / span;
  }
  for (let iteration = 0; iteration < maxIterations; iteration += 1) {
    if (Math.abs(upper - lower) <= tolerance) {
      break;
    }
    const mid = (lower + upper) / 2;
    const midValue = evaluate(mid);
    if (!Number.isFinite(midValue) || midValue === 0) {
      return mid;
    }
    if (lowerValue * midValue <= 0) {
      upper = mid;
    } else {
      lower = mid;
      lowerValue = midValue;
    }
  }
  return (lower + upper) / 2;
}

/** Sign scan, then bisection of `evaluate` down to `tolerance`. A non-finite sample is dropped. */
export function polishedDirectedBreakEvens(input: {
  prices: readonly number[];
  evaluate: (price: number) => number;
  tolerance: number;
  maxIterations: number;
}): DirectedBreakEven[] {
  ensurePositive('tolerance', input.tolerance);
  if (!Number.isInteger(input.maxIterations) || input.maxIterations <= 0) {
    fail('invalid_probability_input', `maxIterations must be a positive integer: ${input.maxIterations}`);
  }
  const sampled: { price: number; pnl: number }[] = [];
  for (const price of input.prices) {
    ensureFinite('price', price);
    const pnl = input.evaluate(price);
    if (Number.isFinite(pnl)) {
      sampled.push({ price, pnl });
    }
  }
  const roots: DirectedBreakEven[] = [];
  for (let index = 1; index < sampled.length; index += 1) {
    const left = sampled[index - 1];
    const right = sampled[index];
    if (left == null || right == null || !(right.price > left.price)) {
      continue;
    }
    const leftSign = pnlSign(left.pnl);
    const rightSign = pnlSign(right.pnl);
    if (leftSign === rightSign) {
      continue;
    }
    const at = bisectRoot(input.evaluate, left.price, right.price, input.tolerance, input.maxIterations);
    pushDirected(roots, at, leftSign < rightSign ? 1 : -1);
  }
  return roots;
}

/** Value approached from one side. A still-diverging wing is infinite. */
export function approachingValue(
  evaluate: (price: number) => number,
  target: number,
  direction: 1 | -1,
): number {
  const bound = Number.isFinite(target) ? target : (target > 0 ? Number.MAX_SAFE_INTEGER : -Number.MAX_SAFE_INTEGER);
  const step = Math.max(1, Math.abs(bound) / 100);
  let previousDelta: number | null = null;
  let previous: number | null = null;
  let settled = false;
  for (let index = 0; index < 10; index += 1) {
    const price = bound - ((10 - index) / 10) * step * direction;
    const value = evaluate(price);
    if (previous != null) {
      const delta = Math.abs(value - previous);
      if ((index > 1 && previousDelta != null && delta + 1e-4 < previousDelta) || delta <= 1e-4) {
        settled = true;
      }
      previousDelta = delta;
    }
    previous = value;
  }
  const atBound = evaluate(bound);
  if (!settled && previous != null) {
    const sign = Math.sign(atBound - previous);
    return sign === 0 ? atBound : sign * Number.POSITIVE_INFINITY;
  }
  return atBound;
}

/** Lowest underlying the several-expiration scan will price. */
export const SCAN_PRICE_FLOOR = 0.1;

/**
 * Several-expiration scan window.
 * Two nearest-expiration implied moves around the spot and the strikes.
 * The lower bound is raised to {@link SCAN_PRICE_FLOOR} when that span crosses zero.
 * A positive strike below the floor stays inside the window.
 */
export function profitLossScanWindow(input: {
  spot: number;
  impliedMove: number;
  strikes: readonly number[];
}): { lower: number; upper: number } {
  const move = Number.isFinite(input.impliedMove) ? Math.max(input.impliedMove, 0) : 0;
  const pad = 2 * move;
  const strikes = input.strikes.filter((strike) => strike > 0);
  const anchors = [input.spot, ...strikes].filter((price) => price > 0);
  const low = anchors.length > 0 ? Math.min(...anchors) : SCAN_PRICE_FLOOR;
  const high = anchors.length > 0 ? Math.max(...anchors) : SCAN_PRICE_FLOOR;
  let lower = low - pad;
  let upper = high + pad;
  if (!(lower >= SCAN_PRICE_FLOOR)) {
    lower = SCAN_PRICE_FLOOR;
  }
  for (const strike of strikes) {
    if (strike < lower) {
      lower = strike;
    }
  }
  if (!(upper > lower)) {
    upper = lower + 1;
  }
  return { lower, upper };
}

/**
 * Best and worst PnL.
 * One expiration uses the strike knots out to 100 times spot.
 * Several expirations use derivative zeros inside {@link profitLossScanWindow}, plus the far wing.
 */
export function profitLossBounds(input: {
  evaluate: (price: number) => number;
  spot: number;
  strikes: readonly number[];
  singleExpiration: boolean;
  impliedMove: number;
  samples?: number;
}): { maxProfit: number; minPnl: number } {
  ensureFinite('spot', input.spot);
  const strikes = [...input.strikes].filter((strike) => strike > 0).sort((left, right) => left - right);
  if (input.singleExpiration) {
    const right = Math.max(input.spot, 0) * 100;
    const knots = strikes.length === 0 ? [0, input.spot + right] : [0, ...strikes, strikes[strikes.length - 1]! + right];
    let maxProfit = Number.NEGATIVE_INFINITY;
    let minPnl = Number.POSITIVE_INFINITY;
    let previous: number | null = null;
    for (let index = 0; index < knots.length; index += 1) {
      const value = input.evaluate(knots[index] ?? 0);
      if (value > maxProfit) {
        maxProfit = value;
      }
      if (value < minPnl) {
        minPnl = value;
      }
      if (index === knots.length - 1 && previous != null) {
        if (value > previous + 1e-4) {
          maxProfit = Number.POSITIVE_INFINITY;
        }
        if (value < previous - 1e-4) {
          minPnl = Number.NEGATIVE_INFINITY;
        }
      }
      previous = value;
    }
    return { maxProfit, minPnl };
  }

  const scan = profitLossScanWindow({ spot: input.spot, impliedMove: input.impliedMove, strikes });
  const lower = scan.lower;
  const upper = scan.upper;
  const count = Math.max(1, input.samples ?? 500);
  const step = (upper - lower) / count;
  const bump = 0.001;
  const slope = (price: number) => (input.evaluate(price + bump) - input.evaluate(price)) / bump;
  const zeros: number[] = [];
  let previousSlope: number | null = null;
  for (let index = 0; index < count; index += 1) {
    const price = lower + index * step;
    if (!(price > 0)) {
      continue;
    }
    const next = slope(price);
    if (Number.isNaN(next)) {
      previousSlope = null;
      continue;
    }
    const left = price - step;
    if (previousSlope != null && left > 0 && Math.sign(previousSlope) !== 0 && Math.sign(next) !== 0 && Math.sign(previousSlope) !== Math.sign(next)) {
      zeros.push(bisectRoot(slope, left, price, bump, 30));
    }
    previousSlope = next;
  }
  let maxProfit = Number.NEGATIVE_INFINITY;
  let minPnl = Number.POSITIVE_INFINITY;
  const consider = (value: number) => {
    if (Number.isNaN(value)) {
      return;
    }
    if (value > maxProfit) {
      maxProfit = value;
    }
    if (value < minPnl) {
      minPnl = value;
    }
  };
  for (const zero of zeros) {
    if (zero > 0) {
      consider(input.evaluate(zero));
    }
  }
  consider(approachingValue(input.evaluate, Math.max(input.spot, 0) * 100, 1));
  return { maxProfit, minPnl };
}
