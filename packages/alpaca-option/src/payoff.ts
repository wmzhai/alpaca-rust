import { fail } from './error';
import type { PayoffLegInput } from './types';

const ROOT_EPSILON = 1e-9;

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

function validateLeg(leg: PayoffLegInput): void {
  ensurePositive('invalid_payoff_input', 'strike', leg.strike);
  ensureFinite('invalid_payoff_input', 'premium', leg.premium);
  if (!Number.isInteger(leg.quantity) || leg.quantity <= 0) {
    fail('invalid_payoff_input', `quantity must be a positive integer: ${leg.quantity}`);
  }
}

function legIntrinsic(leg: PayoffLegInput, underlyingPriceAtExpiry: number): number {
  return leg.optionRight === 'call'
    ? Math.max(underlyingPriceAtExpiry - leg.strike, 0)
    : Math.max(leg.strike - underlyingPriceAtExpiry, 0);
}

function signedLegPayoff(leg: PayoffLegInput, underlyingPriceAtExpiry: number): number {
  const intrinsic = legIntrinsic(leg, underlyingPriceAtExpiry);
  return leg.positionSide === 'long'
    ? leg.quantity * (intrinsic - leg.premium)
    : leg.quantity * (leg.premium - intrinsic);
}

function legSlope(leg: PayoffLegInput, underlyingPriceAtExpiry: number): number {
  if (leg.optionRight === 'call') {
    if (underlyingPriceAtExpiry > leg.strike) {
      return leg.positionSide === 'long' ? leg.quantity : -leg.quantity;
    }
    return 0;
  }

  if (underlyingPriceAtExpiry < leg.strike) {
    return leg.positionSide === 'long' ? -leg.quantity : leg.quantity;
  }
  return 0;
}

function maybePushRoot(
  legs: PayoffLegInput[],
  start: number,
  end: number,
  sample: number,
  roots: number[],
): void {
  const slope = legs.reduce((total, leg) => total + legSlope(leg, sample), 0);
  const value = strategyPayoffAtExpiry({ legs, underlyingPriceAtExpiry: sample });
  if (Math.abs(slope) <= ROOT_EPSILON) {
    return;
  }

  const root = sample - value / slope;
  if (!Number.isFinite(root) || root < 0) {
    return;
  }

  const startOk = root + ROOT_EPSILON >= start;
  const endOk = Number.isFinite(end) ? root - ROOT_EPSILON <= end : true;
  if (startOk && endOk) {
    roots.push(root);
  }
}

export function singleLegPayoffAtExpiry(input: {
  optionRight: PayoffLegInput['optionRight'];
  positionSide: PayoffLegInput['positionSide'];
  strike: number;
  premium: number;
  quantity: number;
  underlyingPriceAtExpiry: number;
}): number {
  return strategyPayoffAtExpiry({
    legs: [{
      optionRight: input.optionRight,
      positionSide: input.positionSide,
      strike: input.strike,
      premium: input.premium,
      quantity: input.quantity,
    }],
    underlyingPriceAtExpiry: input.underlyingPriceAtExpiry,
  });
}

export function strategyPayoffAtExpiry(input: {
  legs: PayoffLegInput[];
  underlyingPriceAtExpiry: number;
}): number {
  ensureFinite('invalid_payoff_input', 'underlyingPriceAtExpiry', input.underlyingPriceAtExpiry);
  if (input.underlyingPriceAtExpiry < 0) {
    fail('invalid_payoff_input', `underlyingPriceAtExpiry must be non-negative: ${input.underlyingPriceAtExpiry}`);
  }

  let total = 0;
  for (const leg of input.legs) {
    validateLeg(leg);
    total += signedLegPayoff(leg, input.underlyingPriceAtExpiry);
  }
  return total;
}

export function breakEvenPoints(input: { legs: PayoffLegInput[] }): number[] {
  for (const leg of input.legs) {
    validateLeg(leg);
  }
  if (input.legs.length === 0) {
    return [];
  }

  const strikes = [...new Set(input.legs.map((leg) => leg.strike).sort((a, b) => a - b))];
  const roots: number[] = [];
  let intervalStart = 0;

  strikes.forEach((boundary, index) => {
    const sample = index === 0 ? Math.max(boundary / 2, 0) : (intervalStart + boundary) / 2;
    maybePushRoot(input.legs, intervalStart, boundary, sample, roots);
    intervalStart = boundary;
  });

  maybePushRoot(input.legs, intervalStart, Number.POSITIVE_INFINITY, intervalStart + Math.max(intervalStart, 1), roots);
  return [...new Set(roots.map((value) => Math.round(value * 1e8) / 1e8))].sort((a, b) => a - b);
}
