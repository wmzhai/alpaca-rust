import { clock as timeClock, expiration as timeExpiration } from '@alpaca/time';

import { canonicalContract } from './contract';
import { fail } from './error';
import { refineBracketedRoot } from './numeric';
import { greeksBlackScholes, intrinsicValue, priceBlackScholes } from './pricing';
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
  StrategyPnlInput,
} from './types';

const CONTRACT_MULTIPLIER = 100;
const DEFAULT_RISK_FREE_RATE = 0.0368;

type PreparedStrategyLeg = {
  optionRight: OptionRight;
  strike: number;
  quantity: number;
  years: number;
  impliedVolatility: number | null;
};

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

function strategyEntryCost(positions: OptionPosition[], entryCost: number | null): number {
  if (entryCost != null) {
    ensureFinite('invalid_strategy_payoff_input', 'entryCost', entryCost);
    return entryCost;
  }

  let total = 0;
  for (const position of positions) {
    total += Number(position.avg_cost) * position.qty * CONTRACT_MULTIPLIER;
  }
  return total;
}

function prepareStrategyContext(input: {
  positions: OptionPosition[];
  evaluation_time: string;
  entry_cost: number | null;
  dividend_yield: number | null;
  long_volatility_shift: number | null;
}): { prepared: PreparedStrategyLeg[]; entryCost: number; dividendYield: number } {
  const dividendYield = input.dividend_yield ?? 0;
  ensureFinite('invalid_strategy_payoff_input', 'dividendYield', dividendYield);
  if (input.long_volatility_shift != null) {
    ensureFinite('invalid_strategy_payoff_input', 'longVolatilityShift', input.long_volatility_shift);
  }
  timeClock.parseTimestamp(input.evaluation_time);

  const entryCost = strategyEntryCost(input.positions, input.entry_cost);
  const prepared: PreparedStrategyLeg[] = [];
  for (const position of input.positions) {
    const contract = strategyPositionContract(position);
    validateStrategyPosition(position, contract);
    const years = valuationYears(contract.expiration_date, input.evaluation_time);
    const impliedVolatility = years <= 0
      ? null
      : (() => {
          if (position.snapshot?.implied_volatility == null) {
            fail(
              'invalid_strategy_payoff_input',
              `impliedVolatility is required before expiration: ${position.contract}`,
            );
          }
          const value = position.qty > 0
            ? position.snapshot.implied_volatility + (input.long_volatility_shift ?? 0)
            : position.snapshot.implied_volatility;
          ensurePositive('invalid_strategy_payoff_input', 'impliedVolatility', value);
          return value;
        })();
    prepared.push({
      optionRight: contract.option_right,
      strike: contract.strike,
      quantity: position.qty,
      years,
      impliedVolatility,
    });
  }

  return { prepared, entryCost, dividendYield };
}

function strategyMarkValuePrepared(input: {
  prepared: PreparedStrategyLeg[];
  underlying_price: number;
  rate: number;
  dividend_yield: number;
}): number {
  ensureFinite('invalid_strategy_payoff_input', 'underlyingPrice', input.underlying_price);
  if (input.underlying_price < 0) {
    fail('invalid_strategy_payoff_input', `underlyingPrice must be non-negative: ${input.underlying_price}`);
  }
  ensureFinite('invalid_strategy_payoff_input', 'rate', input.rate);

  let total = 0;
  for (const position of input.prepared) {
    const optionValue = position.years <= 0
      ? intrinsicValue(input.underlying_price, position.strike, position.optionRight)
      : priceBlackScholes({
          spot: input.underlying_price,
          strike: position.strike,
          years: position.years,
          rate: input.rate,
          dividendYield: input.dividend_yield,
          volatility: position.impliedVolatility ?? 0,
          optionRight: position.optionRight,
        });
    total += optionValue * position.quantity * CONTRACT_MULTIPLIER;
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
  prepared: PreparedStrategyLeg[];
  underlying_price: number;
  rate: number;
  dividend_yield: number;
}): Greeks {
  ensurePositive('invalid_strategy_payoff_input', 'underlyingPrice', input.underlying_price);
  ensureFinite('invalid_strategy_payoff_input', 'rate', input.rate);

  const total = zeroGreeks();
  for (const position of input.prepared) {
    const greeks = position.years <= 0
      ? expiryIntrinsicGreeks(input.underlying_price, position.strike, position.optionRight)
      : greeksBlackScholes({
          spot: input.underlying_price,
          strike: position.strike,
          years: position.years,
          rate: input.rate,
          dividendYield: input.dividend_yield,
          volatility: position.impliedVolatility ?? 0,
          optionRight: position.optionRight,
        });

    total.delta += greeks.delta * position.quantity * CONTRACT_MULTIPLIER;
    total.gamma += greeks.gamma * position.quantity * CONTRACT_MULTIPLIER;
    total.vega += greeks.vega * position.quantity * CONTRACT_MULTIPLIER;
    total.theta += greeks.theta * position.quantity * CONTRACT_MULTIPLIER;
    total.rho += greeks.rho * position.quantity * CONTRACT_MULTIPLIER;
  }

  return total;
}

function pushUniqueRoot(roots: number[], root: number, tolerance: number): void {
  if (roots.some((existing) => Math.abs(existing - root) <= tolerance)) {
    return;
  }
  roots.push(root);
}

function validateStrategyQuantity(strategyQuantity: number): number {
  ensurePositive('invalid_strategy_payoff_input', 'strategyQuantity', strategyQuantity);
  return strategyQuantity;
}

function zeroGreeks(): Greeks {
  return { delta: 0, gamma: 0, vega: 0, theta: 0, rho: 0 };
}

export class OptionStrategy {
  private constructor(
    private readonly strategyPositions: OptionPosition[],
    private readonly prepared: PreparedStrategyLeg[],
    private readonly entryCost: number,
    private readonly rate: number,
    private readonly dividendYield: number,
  ) {}

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
      evaluation_time: evaluationTime,
      entry_cost: input.entry_cost,
      rate: input.rate ?? DEFAULT_RISK_FREE_RATE,
      dividend_yield: input.dividend_yield ?? null,
      long_volatility_shift: input.long_volatility_shift ?? null,
    });
  }

  static prepare(input: OptionStrategyInput & { evaluation_time: string }): OptionStrategy {
    const rate = input.rate ?? DEFAULT_RISK_FREE_RATE;
    ensureFinite('invalid_strategy_payoff_input', 'rate', rate);
    const context = prepareStrategyContext({
      positions: input.positions,
      evaluation_time: input.evaluation_time,
      entry_cost: input.entry_cost,
      dividend_yield: input.dividend_yield ?? null,
      long_volatility_shift: input.long_volatility_shift ?? null,
    });
    return new OptionStrategy([...input.positions], context.prepared, context.entryCost, rate, context.dividendYield);
  }

  markValueAt(underlyingPrice: number): number {
    return strategyMarkValuePrepared({
      prepared: this.prepared,
      underlying_price: underlyingPrice,
      rate: this.rate,
      dividend_yield: this.dividendYield,
    });
  }

  pnlAt(underlyingPrice: number): number {
    return this.markValueAt(underlyingPrice) - this.entryCost;
  }

  greeksAt(underlyingPrice: number, strategyQuantity: number): Greeks {
    const quantity = validateStrategyQuantity(strategyQuantity);
    const total = strategyGreeksPrepared({
      prepared: this.prepared,
      underlying_price: underlyingPrice,
      rate: this.rate,
      dividend_yield: this.dividendYield,
    });

    return {
      delta: total.delta * quantity,
      gamma: total.gamma * quantity,
      vega: total.vega * quantity,
      theta: total.theta * quantity,
      rho: total.rho * quantity,
    };
  }

  positions(): OptionPosition[] {
    return [...this.strategyPositions];
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
        pnl: markValue - this.entryCost,
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

  breakEvenPoints(input: Omit<StrategyBreakEvenInput, 'positions' | 'evaluation_time' | 'entry_cost' | 'rate' | 'dividend_yield' | 'long_volatility_shift'>): number[] {
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

  static aggregateSnapshotGreeks(input: {
    positions: OptionPosition[];
    strategyQuantity: number;
  }): Greeks {
    const strategyQuantity = validateStrategyQuantity(input.strategyQuantity);
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
      delta: total.delta * strategyQuantity,
      gamma: total.gamma * strategyQuantity,
      vega: total.vega * strategyQuantity,
      theta: total.theta * strategyQuantity,
      rho: total.rho * strategyQuantity,
    };
  }

  static aggregateModelGreeks(input: {
    positions: OptionPosition[];
    underlying_price: number;
    evaluation_time: string;
    rate: number;
    dividend_yield: number | null;
    long_volatility_shift: number | null;
    strategyQuantity: number;
  }): Greeks {
    return OptionStrategy.prepare({
      positions: input.positions,
      evaluation_time: input.evaluation_time,
      entry_cost: 0,
      rate: input.rate,
      dividend_yield: input.dividend_yield,
      long_volatility_shift: input.long_volatility_shift,
    }).greeksAt(input.underlying_price, input.strategyQuantity);
  }
}

export function strategyPnl(input: StrategyPnlInput): number {
  return OptionStrategy.prepare(input).pnlAt(input.underlying_price);
}

export function strategyBreakEvenPoints(input: StrategyBreakEvenInput): number[] {
  return OptionStrategy.prepare(input).breakEvenPoints(input);
}
