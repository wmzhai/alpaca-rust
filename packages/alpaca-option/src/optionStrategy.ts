import { clock as timeClock, expiration as timeExpiration } from '@alpaca/time';

import { canonicalContract } from './contract';
import { fail } from './error';
import { refineBracketedRoot } from './numeric';
import { spread as snapshotSpread } from './snapshot';
import {
  greeksBlackScholes,
  intrinsicValue,
  priceBlackScholes,
} from './pricing';
import type {
  Greeks,
  OptionContract,
  OptionPosition,
  OptionRight,
  OptionStrategyBreakEvenBracketInput,
  OptionStrategyCurveInput,
  OptionStrategyCurvePoint,
  OptionStrategyInput,
  StrategyBreakEvenInput,
  StrategyBreakEvenSideInput,
  StrategyPnlInput,
  StrategyPnlPeak,
  StrategyPnlPeakSearchInput,
  StrategyPositionTotals,
} from './types';

const CONTRACT_MULTIPLIER = 100;
const DEFAULT_RISK_FREE_RATE = 0.0368;

function ensureFinite(code: string, name: string, value: number): void {
  if (!Number.isFinite(value)) {
    fail(code, `${name} must be finite: ${value}`);
  }
}

function ensurePositive(code: string, name: string, value: number): void {
  ensureFinite(code, name, value);
  if (value <= 0) {
    fail(code, `${name} must be greater than zero: ${value}`);
  }
}

function strategyPositionContract(position: OptionPosition): OptionContract {
  const contract = canonicalContract(position);
  if (!contract) {
    fail('invalid_occ_symbol', `invalid occ symbol: ${position.contract}`);
  }
  return contract;
}

function snapshotPrice(position: OptionPosition): number {
  const quote = position.snapshot?.quote;
  if (!quote) {
    return 0;
  }
  if (Number.isFinite(quote.mark)) {
    return quote.mark ?? 0;
  }
  if (Number.isFinite(quote.bid) && Number.isFinite(quote.ask)) {
    return ((quote.bid ?? 0) + (quote.ask ?? 0)) / 2;
  }
  if (Number.isFinite(quote.bid)) {
    return quote.bid ?? 0;
  }
  if (Number.isFinite(quote.ask)) {
    return quote.ask ?? 0;
  }
  return Number.isFinite(quote.last) ? quote.last ?? 0 : 0;
}

function validateStrategyPosition(position: OptionPosition, contract: OptionContract): void {
  ensurePositive('invalid_strategy_payoff_input', 'contract.strike', contract.strike);
  if (!Number.isInteger(position.qty) || position.qty === 0) {
    fail('invalid_strategy_payoff_input', `quantity must be a non-zero integer: ${position.qty}`);
  }
  const avgCost = Number(position.avg_cost);
  ensureFinite('invalid_strategy_payoff_input', 'avgCost', avgCost);
  if (position.snapshot?.implied_volatility != null) {
    ensureFinite('invalid_strategy_payoff_input', 'impliedVolatility', position.snapshot.implied_volatility);
  }
  if (position.snapshot) {
    ensureFinite('invalid_strategy_payoff_input', 'markPrice', snapshotPrice(position));
    ensureFinite(
      'invalid_strategy_payoff_input',
      'referenceUnderlyingPrice',
      position.snapshot.underlying_price ?? 0,
    );
  }
}

function valuationYears(expirationDate: string, evaluationTime: string): number {
  try {
    timeExpiration.close(expirationDate);
    return timeExpiration.years(expirationDate, evaluationTime);
  } catch (error) {
    fail('invalid_strategy_payoff_input', `invalid expiration context for ${expirationDate}: ${String(error)}`);
  }
}

function validateStrategyQty(qty: number): number {
  if (!Number.isInteger(qty) || qty <= 0) {
    fail('invalid_strategy_payoff_input', `qty must be a positive integer: ${qty}`);
  }
  return qty;
}

function strategyEntryCost(positions: OptionPosition[], qty: number, entryCost: number | null): number {
  if (entryCost != null) {
    ensureFinite('invalid_strategy_payoff_input', 'entryCost', entryCost);
    return entryCost;
  }

  let total = 0;
  for (const position of positions) {
    total += Number(position.avg_cost) * position.qty * CONTRACT_MULTIPLIER;
  }
  return total * qty;
}

function prepareStrategyContext(input: {
  positions: OptionPosition[];
  qty: number;
  evaluation_time: string;
  entry_cost: number | null;
  dividend_yield: number | null;
}): { positions: OptionPosition[]; entryCost: number; dividendYield: number; qty: number } {
  const dividendYield = input.dividend_yield ?? 0;
  ensureFinite('invalid_strategy_payoff_input', 'dividendYield', dividendYield);
  timeClock.parseTimestamp(input.evaluation_time);

  const qty = validateStrategyQty(input.qty);
  const entryCost = strategyEntryCost(input.positions, qty, input.entry_cost);
  const positions: OptionPosition[] = [];
  for (const position of input.positions) {
    const contract = strategyPositionContract(position);
    validateStrategyPosition(position, contract);
    const years = valuationYears(contract.expiration_date, input.evaluation_time);
    if (years > 0) {
      if (position.snapshot?.implied_volatility == null) {
        fail(
          'invalid_strategy_payoff_input',
          `impliedVolatility is required before expiration: ${position.contract}`,
        );
      }
      ensurePositive('invalid_strategy_payoff_input', 'impliedVolatility', position.snapshot.implied_volatility);
    }
    positions.push({
      ...position,
      option_right: contract.option_right,
      strike: contract.strike,
      valuation_years: years,
    });
  }

  return { positions, entryCost, dividendYield, qty };
}

function preparedOptionRight(position: OptionPosition): OptionRight {
  if (position.option_right !== 'call' && position.option_right !== 'put') {
    fail('invalid_strategy_payoff_input', `optionRight is required on prepared position: ${position.contract}`);
  }
  return position.option_right;
}

function preparedStrike(position: OptionPosition): number {
  const strike = position.strike ?? null;
  if (strike == null || !Number.isFinite(strike)) {
    fail('invalid_strategy_payoff_input', `strike is required on prepared position: ${position.contract}`);
  }
  return strike;
}

function preparedYears(position: OptionPosition): number {
  const years = position.valuation_years ?? null;
  if (years == null || !Number.isFinite(years)) {
    fail('invalid_strategy_payoff_input', `valuationYears is required on prepared position: ${position.contract}`);
  }
  return years;
}

function preparedImpliedVolatility(position: OptionPosition): number {
  const impliedVolatility = position.snapshot?.implied_volatility ?? null;
  if (impliedVolatility == null || !Number.isFinite(impliedVolatility)) {
    fail('invalid_strategy_payoff_input', `impliedVolatility is required before expiration: ${position.contract}`);
  }
  return impliedVolatility;
}

function strategyMarkValuePrepared(input: {
  positions: OptionPosition[];
  underlying_price: number;
  dividend_yield: number;
}): number {
  ensureFinite('invalid_strategy_payoff_input', 'underlyingPrice', input.underlying_price);
  if (input.underlying_price < 0) {
    fail('invalid_strategy_payoff_input', `underlyingPrice must be non-negative: ${input.underlying_price}`);
  }
  ensureFinite('invalid_strategy_payoff_input', 'rate', DEFAULT_RISK_FREE_RATE);

  let total = 0;
  for (const position of input.positions) {
    const optionRight = preparedOptionRight(position);
    const strike = preparedStrike(position);
    const years = preparedYears(position);
    const optionValue = years <= 0
      ? intrinsicValue(input.underlying_price, strike, optionRight)
      : priceBlackScholes({
          spot: input.underlying_price,
          strike,
          years,
          rate: DEFAULT_RISK_FREE_RATE,
          dividendYield: input.dividend_yield,
          volatility: preparedImpliedVolatility(position),
          optionRight,
        });
    total += optionValue * position.qty * CONTRACT_MULTIPLIER;
  }

  return total;
}

function expiryIntrinsicGreeks(underlyingPrice: number, strike: number, optionRight: OptionRight): Greeks {
  const delta = (() => {
    if (optionRight === 'call') {
      if (underlyingPrice > strike) return 1;
      if (underlyingPrice < strike) return 0;
      return 0.5;
    }
    if (underlyingPrice < strike) return -1;
    if (underlyingPrice > strike) return 0;
    return -0.5;
  })();
  return { delta, gamma: 0, vega: 0, theta: 0, rho: 0 };
}

function strategyGreeksPrepared(input: {
  positions: OptionPosition[];
  underlying_price: number;
  dividend_yield: number;
}): Greeks {
  ensurePositive('invalid_strategy_payoff_input', 'underlyingPrice', input.underlying_price);
  ensureFinite('invalid_strategy_payoff_input', 'rate', DEFAULT_RISK_FREE_RATE);

  const total = zeroGreeks();
  for (const position of input.positions) {
    const optionRight = preparedOptionRight(position);
    const strike = preparedStrike(position);
    const years = preparedYears(position);
    const greeks = years <= 0
      ? expiryIntrinsicGreeks(input.underlying_price, strike, optionRight)
      : greeksBlackScholes({
          spot: input.underlying_price,
          strike,
          years,
          rate: DEFAULT_RISK_FREE_RATE,
          dividendYield: input.dividend_yield,
          volatility: preparedImpliedVolatility(position),
          optionRight,
        });

    total.delta += greeks.delta * position.qty * CONTRACT_MULTIPLIER;
    total.gamma += greeks.gamma * position.qty * CONTRACT_MULTIPLIER;
    total.vega += greeks.vega * position.qty * CONTRACT_MULTIPLIER;
    total.theta += greeks.theta * position.qty * CONTRACT_MULTIPLIER;
    total.rho += greeks.rho * position.qty * CONTRACT_MULTIPLIER;
  }

  return total;
}

function pushUniqueRoot(roots: number[], root: number, tolerance: number): void {
  if (roots.some((existing) => Math.abs(existing - root) <= tolerance)) {
    return;
  }
  roots.push(root);
}

function zeroGreeks(): Greeks {
  return { delta: 0, gamma: 0, vega: 0, theta: 0, rho: 0 };
}

export function optionPositionWithModelInputs(
  position: OptionPosition,
  impliedVolatility: number,
  underlyingPrice: number | null,
): OptionPosition {
  return {
    ...position,
    snapshot: {
      ...position.snapshot,
      implied_volatility: impliedVolatility,
      underlying_price: underlyingPrice ?? position.snapshot.underlying_price,
    },
  };
}

export function optionPositionWithQtyMultiplier(
  position: OptionPosition,
  multiplier: number,
): OptionPosition {
  return {
    ...position,
    qty: position.qty * multiplier,
  };
}

export function optionPositionEffectiveIv(
  position: OptionPosition,
  fallbackIv: number | null,
  defaultIv: number,
): number {
  const snapshotIv = position.snapshot?.implied_volatility ?? 0;
  if (Number.isFinite(snapshotIv) && snapshotIv > 0) return snapshotIv;
  if (fallbackIv != null && Number.isFinite(fallbackIv) && fallbackIv > 0) return fallbackIv;
  return defaultIv;
}

export function uniqueBreakEvenPoints(points: Iterable<number>, tolerance: number): number[] {
  const resolvedTolerance = Number.isFinite(tolerance) && tolerance > 0 ? tolerance : 1e-6;
  const unique: number[] = [];
  for (const point of points) {
    if (!Number.isFinite(point)) continue;
    if (unique.some((existing) => Math.abs(existing - point) <= resolvedTolerance * 10)) {
      continue;
    }
    unique.push(point);
  }
  return unique.sort((a, b) => a - b);
}

function strategyPositionTotals(
  positions: OptionPosition[],
  qty: number,
): StrategyPositionTotals {
  let value = 0;
  let cost = 0;
  let spread = 0;

  for (const position of positions) {
    value += snapshotPrice(position) * position.qty * CONTRACT_MULTIPLIER;
    cost += Number(position.avg_cost) * position.qty * CONTRACT_MULTIPLIER;
    const spreadPerContract = Math.round(snapshotSpread(position.snapshot) * 100) / 100;
    spread += spreadPerContract * Math.abs(position.qty) * CONTRACT_MULTIPLIER;
  }

  value *= qty;
  cost *= qty;
  spread *= Math.abs(qty);

  return {
    value,
    cost,
    spread,
    spread_rate: Math.abs(cost) > 1e-10 ? spread / Math.abs(cost) : null,
  };
}

export class OptionStrategy {
  public underlying_price = 0;
  public greeks: Greeks = zeroGreeks();
  public cost = 0;
  public value = 0;
  public pnl = 0;
  public cashflow: number | null = null;
  public stock_qty = 0;
  public stock_cashflow = 0;
  public spread: number | null = null;
  public spread_rate: number | null = null;
  public max_profit: number | null = null;
  public max_loss: number | null = null;
  public buying_power: number | null = null;
  public break_even_points: number[] = [];
  public realtime_break_even_points: number[] = [];
  public break_even_low_open = false;
  public break_even_high_open = false;
  public break_even_low_distance_percent = 0;
  public break_even_high_distance_percent = 0;
  public break_even_width: number | null = null;
  public break_even_width_percent = 0;
  public realtime_break_even_low_open = false;
  public realtime_break_even_high_open = false;
  public realtime_break_even_low_distance_percent = 0;
  public realtime_break_even_high_distance_percent = 0;
  public realtime_break_even_width: number | null = null;
  public realtime_break_even_width_percent = 0;
  public realtime_max_profit_price: number | null = null;
  public realtime_max_profit: number | null = null;
  public realtime_max_profit_unit_value: number | null = null;
  public pnl_at_expire: number | null = null;
  public short_expire_delta: number | null = null;
  public short_expiration: string | null = null;
  public long_expiration: string | null = null;
  public short_dte: number | null = null;
  public long_dte: number | null = null;
  public win_rate: number | null = null;
  public theta_rate: number | null = null;
  public theta_total: number | null = null;
  public score: number | null = null;
  public rank: number | null = null;
  public url: string | null = null;

  private constructor(
    public positions: OptionPosition[],
    public qty: number,
    private entryCost: number,
    private readonly dividendYield: number,
  ) {
    this.cost = entryCost;
    this.calculateValue();
    this.calculateSpread();
    this.calculatePnl();
  }

  static expirationTime(positions: OptionPosition[]): string {
    const expirationDate = positions
      .map((position) => strategyPositionContract(position).expiration_date.trim())
      .filter((value) => value.length > 0)
      .sort()[0];
    if (!expirationDate) {
      fail('invalid_strategy_payoff_input', 'at least one position with expirationDate is required');
    }
    try {
      return timeExpiration.close(expirationDate);
    } catch (error) {
      fail('invalid_strategy_payoff_input', `invalid expiration context for ${expirationDate}: ${String(error)}`);
    }
  }

  static fromInput(input: OptionStrategyInput): OptionStrategy {
    const evaluationTime = input.evaluation_time?.trim() || OptionStrategy.expirationTime(input.positions);
    return OptionStrategy.prepare({
      positions: input.positions,
      qty: input.qty,
      evaluation_time: evaluationTime,
      entry_cost: input.entry_cost,
      dividend_yield: input.dividend_yield ?? null,
    });
  }

  static prepare(input: OptionStrategyInput & { evaluation_time: string }): OptionStrategy {
    const context = prepareStrategyContext({
      positions: input.positions,
      qty: input.qty,
      evaluation_time: input.evaluation_time,
      entry_cost: input.entry_cost,
      dividend_yield: input.dividend_yield ?? null,
    });
    return new OptionStrategy(context.positions, context.qty, context.entryCost, context.dividendYield);
  }

  markValueAt(underlyingPrice: number): number {
    return strategyMarkValuePrepared({
      positions: this.positions,
      underlying_price: underlyingPrice,
      dividend_yield: this.dividendYield,
    }) * this.qty + this.stockValueAt(underlyingPrice);
  }

  pnlAt(underlyingPrice: number): number {
    return this.markValueAt(underlyingPrice) - this.effectiveEntryCost();
  }

  greeksAt(underlyingPrice: number): Greeks {
    const total = strategyGreeksPrepared({
      positions: this.positions,
      underlying_price: underlyingPrice,
      dividend_yield: this.dividendYield,
    });

    return {
      delta: total.delta * this.qty + this.stock_qty,
      gamma: total.gamma * this.qty,
      vega: total.vega * this.qty,
      theta: total.theta * this.qty,
      rho: total.rho * this.qty,
    };
  }

  positionTotals(): StrategyPositionTotals {
    return strategyPositionTotals(this.positions, this.qty);
  }

  private effectiveEntryCost(): number {
    return this.cashflow == null ? this.cost - this.stock_cashflow : -this.cashflow;
  }

  private syncEntryCostFromState(): void {
    this.entryCost = this.effectiveEntryCost();
  }

  private stockValueAt(underlyingPrice: number): number {
    return Number.isFinite(underlyingPrice) ? this.stock_qty * underlyingPrice : 0;
  }

  private stockValueAtCurrentPrice(): number {
    return this.stockValueAt(this.underlying_price);
  }

  calculatePositionTotals(): StrategyPositionTotals {
    const totals = this.positionTotals();
    this.value = totals.value + this.stockValueAtCurrentPrice();
    this.cost = totals.cost;
    this.entryCost = totals.cost;
    this.spread = totals.spread;
    this.spread_rate = totals.spread_rate;
    this.pnl = this.value - this.cost;
    return totals;
  }

  calculateCostFromPositions(): number {
    const totals = this.positionTotals();
    this.cost = totals.cost;
    this.entryCost = totals.cost;
    return this.cost;
  }

  calculateValue(): number {
    this.value = this.positionTotals().value + this.stockValueAtCurrentPrice();
    return this.value;
  }

  calculatePnl(): number {
    this.syncEntryCostFromState();
    this.pnl = this.value - this.effectiveEntryCost();
    return this.pnl;
  }

  calculateSpread(): number | null {
    const totals = this.positionTotals();
    this.spread = totals.spread;
    this.spread_rate = totals.spread_rate;
    return this.spread;
  }

  calculateGreeks(): Greeks {
    if (this.underlying_price > 0) {
      this.greeks = this.greeksAt(this.underlying_price);
    } else {
      const greeks = this.positions.length === 0
        ? zeroGreeks()
        : OptionStrategy.aggregateSnapshotGreeks({ positions: this.positions, qty: this.qty });
      this.greeks = { ...greeks, delta: greeks.delta + this.stock_qty };
    }
    return this.greeks;
  }

  calculateExpirePnl(): number | null {
    if (this.underlying_price <= 0) {
      this.pnl_at_expire = null;
      return null;
    }
    this.pnl_at_expire = this.pnlAt(this.underlying_price);
    return this.pnl_at_expire;
  }

  calculateBreakEvenPoints(
    input: Omit<StrategyBreakEvenInput, 'positions' | 'qty' | 'evaluation_time' | 'entry_cost' | 'dividend_yield'>,
  ): number[] {
    this.break_even_points = this.breakEvenPoints(input);
    return [...this.break_even_points];
  }

  sampleCurve(input: OptionStrategyCurveInput): OptionStrategyCurvePoint[] {
    ensureFinite('invalid_strategy_payoff_input', 'lowerBound', input.lower_bound);
    ensureFinite('invalid_strategy_payoff_input', 'upperBound', input.upper_bound);
    if (input.lower_bound < 0) {
      fail('invalid_strategy_payoff_input', `lowerBound must be non-negative: ${input.lower_bound}`);
    }
    if (input.lower_bound >= input.upper_bound) {
      fail(
        'invalid_strategy_payoff_input',
        `lowerBound must be less than upperBound: ${input.lower_bound} >= ${input.upper_bound}`,
      );
    }
    ensurePositive('invalid_strategy_payoff_input', 'step', input.step);

    const points: OptionStrategyCurvePoint[] = [];
    let underlyingPrice = input.lower_bound;
    while (true) {
      const markValue = this.markValueAt(underlyingPrice);
      points.push({
        underlying_price: underlyingPrice,
        mark_value: markValue,
        pnl: markValue - this.effectiveEntryCost(),
      });

      if (underlyingPrice >= input.upper_bound) {
        break;
      }

      const next = Math.min(underlyingPrice + input.step, input.upper_bound);
      if (next <= underlyingPrice) {
        fail('invalid_strategy_payoff_input', 'step did not advance strategy curve scan');
      }
      underlyingPrice = next;
    }

    return points;
  }

  breakEvenPoints(input: Omit<StrategyBreakEvenInput, 'positions' | 'qty' | 'evaluation_time' | 'entry_cost' | 'dividend_yield'>): number[] {
    ensureFinite('invalid_strategy_payoff_input', 'lowerBound', input.lower_bound);
    ensureFinite('invalid_strategy_payoff_input', 'upperBound', input.upper_bound);
    if (input.lower_bound >= input.upper_bound) {
      fail(
        'invalid_strategy_payoff_input',
        `lowerBound must be less than upperBound: ${input.lower_bound} >= ${input.upper_bound}`,
      );
    }

    const tolerance = input.tolerance ?? 1e-9;
    ensurePositive('invalid_strategy_payoff_input', 'tolerance', tolerance);
    const scanStep = input.scan_step ?? 1;
    ensurePositive('invalid_strategy_payoff_input', 'scanStep', scanStep);
    const maxIterations = input.maxIterations ?? 100;
    if (!Number.isInteger(maxIterations) || maxIterations <= 0) {
      fail('invalid_strategy_payoff_input', `maxIterations must be a positive integer: ${maxIterations}`);
    }

    const roots: number[] = [];
    let previousSpot = input.lower_bound;
    let previousValue = this.pnlAt(previousSpot);
    if (Math.abs(previousValue) <= tolerance) {
      pushUniqueRoot(roots, previousSpot, tolerance * 10);
    }

    let currentSpot = Math.min(previousSpot + scanStep, input.upper_bound);
    while (currentSpot <= input.upper_bound) {
      const currentValue = this.pnlAt(currentSpot);
      if (Math.abs(currentValue) <= tolerance) {
        pushUniqueRoot(roots, currentSpot, tolerance * 10);
      } else if (Math.abs(previousValue) <= tolerance) {
        pushUniqueRoot(roots, previousSpot, tolerance * 10);
      } else if (Math.sign(previousValue) !== Math.sign(currentValue)) {
        pushUniqueRoot(
          roots,
          refineBracketedRoot(previousSpot, currentSpot, (spot) => this.pnlAt(spot), tolerance, maxIterations),
          tolerance * 10,
        );
      }

      if (currentSpot >= input.upper_bound) {
        break;
      }
      previousSpot = currentSpot;
      previousValue = currentValue;
      currentSpot = Math.min(currentSpot + scanStep, input.upper_bound);
    }

    return roots.sort((a, b) => a - b);
  }

  breakEvenBetween(input: OptionStrategyBreakEvenBracketInput): number | null {
    ensureFinite('invalid_strategy_payoff_input', 'lowerBound', input.lower_bound);
    ensureFinite('invalid_strategy_payoff_input', 'upperBound', input.upper_bound);
    if (input.lower_bound >= input.upper_bound) {
      fail(
        'invalid_strategy_payoff_input',
        `lowerBound must be less than upperBound: ${input.lower_bound} >= ${input.upper_bound}`,
      );
    }

    const tolerance = input.tolerance ?? 1e-9;
    ensurePositive('invalid_strategy_payoff_input', 'tolerance', tolerance);
    const maxIterations = input.maxIterations ?? 100;
    if (!Number.isInteger(maxIterations) || maxIterations <= 0) {
      fail('invalid_strategy_payoff_input', `maxIterations must be a positive integer: ${maxIterations}`);
    }

    const lowerValue = this.pnlAt(input.lower_bound);
    if (Math.abs(lowerValue) <= tolerance) {
      return input.lower_bound;
    }

    const upperValue = this.pnlAt(input.upper_bound);
    if (Math.abs(upperValue) <= tolerance) {
      return input.upper_bound;
    }

    if (Math.sign(lowerValue) === Math.sign(upperValue)) {
      return null;
    }

    return refineBracketedRoot(
      input.lower_bound,
      input.upper_bound,
      (spot) => this.pnlAt(spot),
      tolerance,
      maxIterations,
    );
  }

  findBreakEvenLeft(input: StrategyBreakEvenSideInput): number | null {
    return this.findBreakEvenToward(input.pivot, input.boundary, -Math.abs(input.scan_step), input);
  }

  findBreakEvenRight(input: StrategyBreakEvenSideInput): number | null {
    return this.findBreakEvenToward(input.pivot, input.boundary, Math.abs(input.scan_step), input);
  }

  private findBreakEvenToward(
    pivot: number,
    boundary: number,
    signedStep: number,
    input: StrategyBreakEvenSideInput,
  ): number | null {
    const tolerance = input.tolerance ?? 1e-6;
    const maxIterations = input.maxIterations ?? 100;
    ensureFinite('invalid_strategy_payoff_input', 'pivot', pivot);
    ensureFinite('invalid_strategy_payoff_input', 'boundary', boundary);
    ensurePositive('invalid_strategy_payoff_input', 'scanStep', Math.abs(input.scan_step));
    ensurePositive('invalid_strategy_payoff_input', 'tolerance', tolerance);
    if (!Number.isInteger(maxIterations) || maxIterations <= 0) {
      fail('invalid_strategy_payoff_input', `maxIterations must be a positive integer: ${maxIterations}`);
    }
    if ((signedStep < 0 && boundary >= pivot) || (signedStep > 0 && boundary <= pivot)) {
      return null;
    }

    let previous = pivot;
    let previousValue = this.pnlAt(previous);
    if (Math.abs(previousValue) <= tolerance) {
      return previous;
    }

    while (true) {
      const next = signedStep < 0
        ? Math.max(previous + signedStep, boundary)
        : Math.min(previous + signedStep, boundary);
      const nextValue = this.pnlAt(next);
      if (Math.abs(nextValue) <= tolerance) {
        return next;
      }
      if (Math.sign(nextValue) !== Math.sign(previousValue)) {
        return this.breakEvenBetween({
          lower_bound: Math.min(previous, next),
          upper_bound: Math.max(previous, next),
          tolerance,
          maxIterations,
        });
      }
      if (next === boundary) {
        break;
      }
      previous = next;
      previousValue = nextValue;
    }

    return null;
  }

  maximizePnlInRange(lowerBound: number, upperBound: number, iterations: number): StrategyPnlPeak {
    ensureFinite('invalid_strategy_payoff_input', 'lowerBound', lowerBound);
    ensureFinite('invalid_strategy_payoff_input', 'upperBound', upperBound);
    if (lowerBound >= upperBound) {
      fail('invalid_strategy_payoff_input', `lowerBound must be less than upperBound: ${lowerBound} >= ${upperBound}`);
    }
    if (!Number.isInteger(iterations) || iterations <= 0) {
      fail('invalid_strategy_payoff_input', `iterations must be a positive integer: ${iterations}`);
    }

    let left = lowerBound;
    let right = upperBound;
    for (let i = 0; i < iterations; i += 1) {
      const third = (right - left) / 3;
      const midLeft = left + third;
      const midRight = right - third;
      const leftValue = this.pnlAt(midLeft);
      const rightValue = this.pnlAt(midRight);

      if (leftValue < rightValue) {
        left = midLeft;
      } else {
        right = midRight;
      }
    }

    const spot = (left + right) / 2;
    const pnl = this.pnlAt(spot);
    ensureFinite('invalid_strategy_payoff_input', 'peakPnl', pnl);
    return { spot, pnl };
  }

  pnlPeakFromCurrent(input: StrategyPnlPeakSearchInput): StrategyPnlPeak | null {
    ensurePositive('invalid_strategy_payoff_input', 'currentPrice', input.current_price);
    ensurePositive('invalid_strategy_payoff_input', 'leftBoundary', input.left_boundary);
    if (input.left_boundary >= input.current_price) {
      fail(
        'invalid_strategy_payoff_input',
        `leftBoundary must be less than currentPrice: ${input.left_boundary} >= ${input.current_price}`,
      );
    }
    ensureFinite('invalid_strategy_payoff_input', 'rightBoundary', input.right_boundary);
    if (input.right_boundary <= input.current_price) {
      fail(
        'invalid_strategy_payoff_input',
        `rightBoundary must be greater than currentPrice: ${input.right_boundary} <= ${input.current_price}`,
      );
    }
    if (input.right_boundary <= input.left_boundary) {
      fail(
        'invalid_strategy_payoff_input',
        `rightBoundary must be greater than leftBoundary: ${input.right_boundary} <= ${input.left_boundary}`,
      );
    }
    const tolerance = input.tolerance ?? 1e-6;
    ensurePositive('invalid_strategy_payoff_input', 'tolerance', tolerance);
    const maxSearchSteps = input.maxSearchSteps ?? 512;
    if (!Number.isInteger(maxSearchSteps) || maxSearchSteps <= 0) {
      fail('invalid_strategy_payoff_input', `maxSearchSteps must be a positive integer: ${maxSearchSteps}`);
    }

    const preferredStep = Math.min(
      Math.max(
        input.step_hint != null && Number.isFinite(input.step_hint) && input.step_hint > 0
          ? input.step_hint
          : Math.min(Math.max(input.current_price * 0.005, 0.1), 5),
        0.05,
      ),
      Math.max(input.current_price, 1) * 0.05,
    );
    const range = input.right_boundary - input.left_boundary;
    const preferredIntervals = Math.max(Math.ceil(range / preferredStep), 2);
    const intervals = Math.min(preferredIntervals, Math.max(maxSearchSteps, 2));
    const scanStep = range / intervals;
    let bestIndex = 0;
    let bestSpot = input.left_boundary;
    let bestPnl = this.pnlAt(bestSpot);

    for (let index = 1; index <= intervals; index += 1) {
      const spot = index === intervals
        ? input.right_boundary
        : input.left_boundary + scanStep * index;
      const pnl = this.pnlAt(spot);
      if (pnl > bestPnl + tolerance) {
        bestIndex = index;
        bestSpot = spot;
        bestPnl = pnl;
      }
    }

    const peak = bestIndex === 0 || bestIndex === intervals
      ? { spot: bestSpot, pnl: bestPnl }
      : this.maximizePnlInRange(
          input.left_boundary + scanStep * (bestIndex - 1),
          input.left_boundary + scanStep * (bestIndex + 1),
          80,
        );

    return peak.pnl > tolerance ? peak : null;
  }

  static aggregateSnapshotGreeks(input: {
    positions: OptionPosition[];
    qty: number;
  }): Greeks {
    const qty = validateStrategyQty(input.qty);
    const total = zeroGreeks();
    for (const position of input.positions) {
      const quantity = position.qty;
      const greeks = position.snapshot.greeks ?? zeroGreeks();
      total.delta += greeks.delta * quantity * CONTRACT_MULTIPLIER;
      total.gamma += greeks.gamma * quantity * CONTRACT_MULTIPLIER;
      total.vega += greeks.vega * quantity * CONTRACT_MULTIPLIER;
      total.theta += greeks.theta * quantity * CONTRACT_MULTIPLIER;
      total.rho += greeks.rho * quantity * CONTRACT_MULTIPLIER;
    }

    return {
      delta: total.delta * qty,
      gamma: total.gamma * qty,
      vega: total.vega * qty,
      theta: total.theta * qty,
      rho: total.rho * qty,
    };
  }

  static aggregateModelGreeks(input: {
    positions: OptionPosition[];
    underlying_price: number;
    evaluation_time: string;
    dividend_yield: number | null;
    qty: number;
  }): Greeks {
    return OptionStrategy.prepare({
      positions: input.positions,
      qty: input.qty,
      evaluation_time: input.evaluation_time,
      entry_cost: 0,
      dividend_yield: input.dividend_yield,
    }).greeksAt(input.underlying_price);
  }
}

export function strategyPnl(input: StrategyPnlInput): number {
  return OptionStrategy.prepare(input).pnlAt(input.underlying_price);
}

export function strategyBreakEvenPoints(input: StrategyBreakEvenInput): number[] {
  return OptionStrategy.prepare(input).breakEvenPoints(input);
}
