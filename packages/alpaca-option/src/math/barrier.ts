import { fail } from '../error';
import { normalCdf } from '../numeric';
import type { OptionRight } from '../types';
import { roundToFixtureYears } from './shared';

type BarrierType = 'down_in' | 'down_out' | 'up_in' | 'up_out';

function ensureFinite(name: string, value: number): void {
  if (!Number.isFinite(value)) {
    fail('invalid_math_input', `${name} must be finite: ${value}`);
  }
}

function ensurePositive(name: string, value: number): void {
  ensureFinite(name, value);
  if (value <= 0) {
    fail('invalid_math_input', `${name} must be greater than zero: ${value}`);
  }
}

function ensureNonNegative(name: string, value: number): void {
  ensureFinite(name, value);
  if (value < 0) {
    fail('invalid_math_input', `${name} must be non-negative: ${value}`);
  }
}

function validateOptionRight(optionRight: OptionRight): OptionRight {
  if (optionRight !== 'call' && optionRight !== 'put') {
    fail('invalid_math_input', `invalid option right: ${optionRight}`);
  }
  return optionRight;
}

function validateBarrierType(barrierType: BarrierType): BarrierType {
  if (
    barrierType !== 'down_in'
    && barrierType !== 'down_out'
    && barrierType !== 'up_in'
    && barrierType !== 'up_out'
  ) {
    fail('invalid_math_input', `invalid barrier_type: ${barrierType}`);
  }
  return barrierType;
}

function isTriggered(spot: number, barrier: number, barrierType: BarrierType): boolean {
  return barrierType === 'down_in' || barrierType === 'down_out'
    ? spot <= barrier
    : spot >= barrier;
}

function safeScaledTerm(powerTerm: number, probabilityTerm: number): number {
  return probabilityTerm === 0 ? 0 : powerTerm * probabilityTerm;
}

class BarrierKernel {
  readonly spot: number;

  readonly strike: number;

  readonly barrier: number;

  readonly rebate: number;

  readonly years: number;

  readonly rate: number;

  readonly dividendYield: number;

  readonly volatility: number;

  readonly optionRight: OptionRight;

  readonly barrierType: BarrierType;

  constructor(input: {
    spot: number;
    strike: number;
    barrier: number;
    rebate: number;
    years: number;
    rate: number;
    dividendYield: number;
    volatility: number;
    optionRight: OptionRight;
    barrierType: BarrierType;
  }) {
    this.spot = input.spot;
    this.strike = input.strike;
    this.barrier = input.barrier;
    this.rebate = input.rebate;
    this.years = input.years;
    this.rate = input.rate;
    this.dividendYield = input.dividendYield;
    this.volatility = input.volatility;
    this.optionRight = input.optionRight;
    this.barrierType = input.barrierType;
  }

  stdDeviation(): number {
    return this.volatility * Math.sqrt(this.years);
  }

  riskFreeDiscount(): number {
    return Math.exp(-this.rate * this.years);
  }

  dividendDiscount(): number {
    return Math.exp(-this.dividendYield * this.years);
  }

  mu(): number {
    return (this.rate - this.dividendYield) / (this.volatility * this.volatility) - 0.5;
  }

  muSigma(): number {
    return (1 + this.mu()) * this.stdDeviation();
  }

  a(phi: number): number {
    const x1 = Math.log(this.spot / this.strike) / this.stdDeviation() + this.muSigma();
    const n1 = normalCdf(phi * x1);
    const n2 = normalCdf(phi * (x1 - this.stdDeviation()));
    return phi * (
      this.spot * this.dividendDiscount() * n1
        - this.strike * this.riskFreeDiscount() * n2
    );
  }

  b(phi: number): number {
    const x2 = Math.log(this.spot / this.barrier) / this.stdDeviation() + this.muSigma();
    const n1 = normalCdf(phi * x2);
    const n2 = normalCdf(phi * (x2 - this.stdDeviation()));
    return phi * (
      this.spot * this.dividendDiscount() * n1
        - this.strike * this.riskFreeDiscount() * n2
    );
  }

  c(eta: number, phi: number): number {
    const hs = this.barrier / this.spot;
    const powHs0 = hs ** (2 * this.mu());
    const powHs1 = powHs0 * hs * hs;
    const y1 = Math.log(this.barrier * hs / this.strike) / this.stdDeviation() + this.muSigma();
    const n1 = normalCdf(eta * y1);
    const n2 = normalCdf(eta * (y1 - this.stdDeviation()));
    return phi * (
      this.spot * this.dividendDiscount() * safeScaledTerm(powHs1, n1)
        - this.strike * this.riskFreeDiscount() * safeScaledTerm(powHs0, n2)
    );
  }

  d(eta: number, phi: number): number {
    const hs = this.barrier / this.spot;
    const powHs0 = hs ** (2 * this.mu());
    const powHs1 = powHs0 * hs * hs;
    const y2 = Math.log(this.barrier / this.spot) / this.stdDeviation() + this.muSigma();
    const n1 = normalCdf(eta * y2);
    const n2 = normalCdf(eta * (y2 - this.stdDeviation()));
    return phi * (
      this.spot * this.dividendDiscount() * safeScaledTerm(powHs1, n1)
        - this.strike * this.riskFreeDiscount() * safeScaledTerm(powHs0, n2)
    );
  }

  e(eta: number): number {
    if (this.rebate <= 0) {
      return 0;
    }

    const powHs0 = (this.barrier / this.spot) ** (2 * this.mu());
    const x2 = Math.log(this.spot / this.barrier) / this.stdDeviation() + this.muSigma();
    const y2 = Math.log(this.barrier / this.spot) / this.stdDeviation() + this.muSigma();
    const n1 = normalCdf(eta * (x2 - this.stdDeviation()));
    const n2 = normalCdf(eta * (y2 - this.stdDeviation()));
    return this.rebate * this.riskFreeDiscount() * (n1 - safeScaledTerm(powHs0, n2));
  }

  f(eta: number): number {
    if (this.rebate <= 0) {
      return 0;
    }

    const mu = this.mu();
    const lambda = Math.sqrt(mu * mu + (2 * this.rate) / (this.volatility * this.volatility));
    const hs = this.barrier / this.spot;
    const powPlus = hs ** (mu + lambda);
    const powMinus = hs ** (mu - lambda);
    const sigmaSqrtT = this.stdDeviation();
    const z = Math.log(this.barrier / this.spot) / sigmaSqrtT + lambda * sigmaSqrtT;
    const n1 = normalCdf(eta * z);
    const n2 = normalCdf(eta * (z - 2 * lambda * sigmaSqrtT));
    return this.rebate * (safeScaledTerm(powPlus, n1) + safeScaledTerm(powMinus, n2));
  }

  value(): number {
    if (this.optionRight === 'call') {
      switch (this.barrierType) {
        case 'down_in':
          return this.strike >= this.barrier
            ? this.c(1, 1) + this.e(1)
            : this.a(1) - this.b(1) + this.d(1, 1) + this.e(1);
        case 'up_in':
          return this.strike >= this.barrier
            ? this.a(1) + this.e(-1)
            : this.b(1) - this.c(-1, 1) + this.d(-1, 1) + this.e(-1);
        case 'down_out':
          return this.strike >= this.barrier
            ? this.a(1) - this.c(1, 1) + this.f(1)
            : this.b(1) - this.d(1, 1) + this.f(1);
        case 'up_out':
          return this.strike >= this.barrier
            ? this.f(-1)
            : this.a(1) - this.b(1) + this.c(-1, 1) - this.d(-1, 1) + this.f(-1);
        default:
          return 0;
      }
    }

    switch (this.barrierType) {
      case 'down_in':
        return this.strike >= this.barrier
          ? this.b(-1) - this.c(1, -1) + this.d(1, -1) + this.e(1)
          : this.a(-1) + this.e(1);
      case 'up_in':
        return this.strike >= this.barrier
          ? this.a(-1) - this.b(-1) + this.d(-1, -1) + this.e(-1)
          : this.c(-1, -1) + this.e(-1);
      case 'down_out':
        return this.strike >= this.barrier
          ? this.a(-1) - this.b(-1) + this.c(1, -1) - this.d(1, -1) + this.f(1)
          : this.f(1);
      case 'up_out':
        return this.strike >= this.barrier
          ? this.b(-1) - this.d(-1, -1) + this.f(-1)
          : this.a(-1) - this.c(-1, -1) + this.f(-1);
      default:
        return 0;
    }
  }
}

export function price(input: {
  spot: number;
  strike: number;
  barrier: number;
  rebate: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  optionRight: OptionRight;
  barrierType: BarrierType;
}): number {
  ensurePositive('spot', input.spot);
  ensurePositive('strike', input.strike);
  ensurePositive('barrier', input.barrier);
  ensureNonNegative('rebate', input.rebate);
  ensurePositive('years', input.years);
  ensureFinite('rate', input.rate);
  ensureFinite('dividendYield', input.dividendYield);
  ensurePositive('volatility', input.volatility);
  const optionRight = validateOptionRight(input.optionRight);
  const barrierType = validateBarrierType(input.barrierType);
  const years = roundToFixtureYears(input.years);

  if (isTriggered(input.spot, input.barrier, barrierType)) {
    fail('invalid_math_input', 'barrier touched or crossed at valuation spot');
  }

  return new BarrierKernel({
    spot: input.spot,
    strike: input.strike,
    barrier: input.barrier,
    rebate: input.rebate,
    years,
    rate: input.rate,
    dividendYield: input.dividendYield,
    volatility: input.volatility,
    optionRight,
    barrierType,
  }).value();
}
