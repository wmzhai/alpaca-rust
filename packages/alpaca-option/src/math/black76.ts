import { fail } from '../error';
import { normalCdf, normalPdf } from '../numeric';
import type { Greeks, OptionRight } from '../types';

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

function optionSign(optionRight: OptionRight): number {
  return optionRight === 'call' ? 1 : -1;
}

function discount(rate: number, years: number): number {
  return Math.exp(-rate * years);
}

function intrinsic(optionRight: OptionRight, forward: number, strike: number): number {
  return optionRight === 'call'
    ? Math.max(forward - strike, 0)
    : Math.max(strike - forward, 0);
}

function validateInput(input: {
  forward: number;
  strike: number;
  years: number;
  rate: number;
  volatility: number;
  optionRight: OptionRight;
}): OptionRight {
  ensurePositive('forward', input.forward);
  ensurePositive('strike', input.strike);
  ensurePositive('years', input.years);
  ensureFinite('rate', input.rate);
  ensureNonNegative('volatility', input.volatility);
  return validateOptionRight(input.optionRight);
}

function d1d2(input: {
  forward: number;
  strike: number;
  years: number;
  volatility: number;
}): { d1: number; d2: number } {
  const sqrtYears = Math.sqrt(input.years);
  const sigmaSqrtT = input.volatility * sqrtYears;
  const d1 = (Math.log(input.forward / input.strike) + 0.5 * input.volatility * input.volatility * input.years)
    / sigmaSqrtT;
  return { d1, d2: d1 - sigmaSqrtT };
}

function priceCore(input: {
  forward: number;
  strike: number;
  years: number;
  rate: number;
  volatility: number;
  optionRight: OptionRight;
}): number {
  const discountRate = discount(input.rate, input.years);
  if (input.volatility === 0) {
    return discountRate * intrinsic(input.optionRight, input.forward, input.strike);
  }

  const sign = optionSign(input.optionRight);
  const { d1, d2 } = d1d2(input);
  return discountRate * sign * (
    input.forward * normalCdf(sign * d1) - input.strike * normalCdf(sign * d2)
  );
}

export function price(input: {
  forward: number;
  strike: number;
  years: number;
  rate: number;
  volatility: number;
  optionRight: OptionRight;
}): number {
  const optionRight = validateInput(input);
  return priceCore({ ...input, optionRight });
}

export function greeks(input: {
  forward: number;
  strike: number;
  years: number;
  rate: number;
  volatility: number;
  optionRight: OptionRight;
}): Greeks {
  const optionRight = validateInput(input);
  ensurePositive('volatility', input.volatility);

  const discountRate = discount(input.rate, input.years);
  const sqrtYears = Math.sqrt(input.years);
  const sigmaSqrtT = input.volatility * sqrtYears;
  const { d1 } = d1d2(input);
  const pdfD1 = normalPdf(d1);
  const optionPrice = priceCore({ ...input, optionRight });

  const delta = optionRight === 'call'
    ? discountRate * normalCdf(d1)
    : discountRate * (normalCdf(d1) - 1);
  const gamma = discountRate * pdfD1 / (input.forward * sigmaSqrtT);
  const vega = discountRate * input.forward * pdfD1 * sqrtYears;
  const theta = input.rate * optionPrice - discountRate * input.forward * pdfD1 * input.volatility / (2 * sqrtYears);
  const rho = -input.years * optionPrice;

  return { delta, gamma, vega, theta, rho };
}

export function impliedVolatilityFromPrice(input: {
  targetPrice: number;
  forward: number;
  strike: number;
  years: number;
  rate: number;
  optionRight: OptionRight;
  lowerBound?: number;
  upperBound?: number;
  tolerance?: number;
  maxIterations?: number;
}): number {
  ensureFinite('targetPrice', input.targetPrice);
  validateInput({ ...input, volatility: 0.2 });

  const intrinsicPrice = discount(input.rate, input.years) * intrinsic(input.optionRight, input.forward, input.strike);
  if (input.targetPrice < intrinsicPrice) {
    fail('invalid_math_input', `targetPrice is below discounted intrinsic value: ${input.targetPrice} < ${intrinsicPrice}`);
  }

  const lowerBound = input.lowerBound ?? 0.0001;
  const upperBound = input.upperBound ?? 5.0;
  ensurePositive('lowerBound', lowerBound);
  ensurePositive('upperBound', upperBound);
  if (lowerBound >= upperBound) {
    fail('invalid_math_input', `lowerBound must be less than upperBound: ${lowerBound} >= ${upperBound}`);
  }

  const tolerance = input.tolerance ?? 1e-10;
  ensurePositive('tolerance', tolerance);
  const maxIterations = input.maxIterations ?? 128;
  if (!Number.isInteger(maxIterations) || maxIterations <= 0) {
    fail('invalid_math_input', `maxIterations must be a positive integer: ${maxIterations}`);
  }

  let lower = lowerBound;
  let upper = upperBound;
  let lowerValue = priceCore({ ...input, volatility: lower }) - input.targetPrice;
  let upperValue = priceCore({ ...input, volatility: upper }) - input.targetPrice;
  if (Math.abs(lowerValue) <= tolerance) {
    return lower;
  }
  if (Math.abs(upperValue) <= tolerance) {
    return upper;
  }
  if (lowerValue * upperValue > 0) {
    fail('root_not_bracketed', `root is not bracketed: f(${lower})=${lowerValue}, f(${upper})=${upperValue}`);
  }

  let volatility = (lower + upper) / 2;
  for (let index = 0; index < maxIterations; index += 1) {
    const value = priceCore({ ...input, volatility }) - input.targetPrice;
    const sqrtYears = Math.sqrt(input.years);
    const { d1 } = d1d2({ forward: input.forward, strike: input.strike, years: input.years, volatility });
    const vega = discount(input.rate, input.years) * input.forward * normalPdf(d1) * sqrtYears;
    const stepEstimate = Number.isFinite(vega) && vega > 0 ? Math.abs(value / vega) : Number.POSITIVE_INFINITY;
    if ((Math.abs(value) <= tolerance && stepEstimate <= tolerance) || Math.abs(upper - lower) <= tolerance) {
      return volatility;
    }
    if (value < 0) {
      lower = volatility;
      lowerValue = value;
    } else {
      upper = volatility;
      upperValue = value;
    }

    const newtonCandidate = Number.isFinite(vega) && vega > 0 ? volatility - value / vega : Number.NaN;
    if (Number.isFinite(newtonCandidate) && newtonCandidate > lower && newtonCandidate < upper) {
      volatility = newtonCandidate;
    } else if (Math.abs(lowerValue) < Math.abs(upperValue)) {
      volatility = (lower + upper) / 2;
    } else {
      volatility = (lower + upper) / 2;
    }
  }

  return volatility;
}
