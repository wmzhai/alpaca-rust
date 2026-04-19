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

function solverResidual(input: {
  optionRight: OptionRight;
  targetPrice: number;
  forward: number;
  strike: number;
  years: number;
  rate: number;
  normalVolatility: number;
}): number {
  const intrinsicPrice = discount(input.rate, input.years) * intrinsic(input.optionRight, input.forward, input.strike);
  if (input.optionRight === 'call' && input.forward > input.strike) {
    return intrinsicPrice + priceCore({ ...input, optionRight: 'put' }) - input.targetPrice;
  }
  if (input.optionRight === 'put' && input.forward < input.strike) {
    return intrinsicPrice + priceCore({ ...input, optionRight: 'call' }) - input.targetPrice;
  }
  return priceCore(input) - input.targetPrice;
}

function validateInput(input: {
  forward: number;
  strike: number;
  years: number;
  rate: number;
  normalVolatility: number;
  optionRight: OptionRight;
}): OptionRight {
  ensureFinite('forward', input.forward);
  ensureFinite('strike', input.strike);
  ensurePositive('years', input.years);
  ensureFinite('rate', input.rate);
  ensureNonNegative('normalVolatility', input.normalVolatility);
  return validateOptionRight(input.optionRight);
}

function dValue(input: {
  forward: number;
  strike: number;
  years: number;
  normalVolatility: number;
}): number {
  return (input.forward - input.strike) / (input.normalVolatility * Math.sqrt(input.years));
}

function priceCore(input: {
  forward: number;
  strike: number;
  years: number;
  rate: number;
  normalVolatility: number;
  optionRight: OptionRight;
}): number {
  const discountRate = discount(input.rate, input.years);
  if (input.normalVolatility === 0) {
    return discountRate * intrinsic(input.optionRight, input.forward, input.strike);
  }

  const sign = optionSign(input.optionRight);
  const spread = input.forward - input.strike;
  const stdDev = input.normalVolatility * Math.sqrt(input.years);
  const d = spread / stdDev;

  return discountRate * (sign * spread * normalCdf(sign * d) + stdDev * normalPdf(d));
}

export function price(input: {
  forward: number;
  strike: number;
  years: number;
  rate: number;
  normalVolatility: number;
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
  normalVolatility: number;
  optionRight: OptionRight;
}): Greeks {
  const optionRight = validateInput(input);
  ensurePositive('normalVolatility', input.normalVolatility);

  const discountRate = discount(input.rate, input.years);
  const sqrtYears = Math.sqrt(input.years);
  const d = dValue(input);
  const pdfD = normalPdf(d);
  const optionPrice = priceCore({ ...input, optionRight });

  const delta = optionRight === 'call'
    ? discountRate * normalCdf(d)
    : discountRate * (normalCdf(d) - 1);
  const gamma = discountRate * pdfD / (input.normalVolatility * sqrtYears);
  const vega = discountRate * sqrtYears * pdfD;
  const theta = input.rate * optionPrice - discountRate * input.normalVolatility * pdfD / (2 * sqrtYears);
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
  validateInput({ ...input, normalVolatility: 0 });

  const intrinsicPrice = discount(input.rate, input.years) * intrinsic(input.optionRight, input.forward, input.strike);
  if (input.targetPrice < intrinsicPrice) {
    fail('invalid_math_input', `targetPrice is below discounted intrinsic value: ${input.targetPrice} < ${intrinsicPrice}`);
  }

  const lowerBound = input.lowerBound ?? 0;
  const upperBound = input.upperBound ?? 20;
  ensureNonNegative('lowerBound', lowerBound);
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
  let lowerValue = solverResidual({ ...input, normalVolatility: lower });
  let upperValue = solverResidual({ ...input, normalVolatility: upper });
  if (lowerValue === 0) {
    return lower;
  }
  if (upperValue === 0) {
    return upper;
  }
  if (lowerValue * upperValue > 0) {
    fail('root_not_bracketed', `root is not bracketed: f(${lower})=${lowerValue}, f(${upper})=${upperValue}`);
  }

  let normalVolatility = (lower + upper) / 2;
  for (let index = 0; index < maxIterations; index += 1) {
    const value = solverResidual({ ...input, normalVolatility });
    if (value === 0) {
      return normalVolatility;
    }
    if (value < 0) {
      lower = normalVolatility;
      lowerValue = value;
    } else {
      upper = normalVolatility;
      upperValue = value;
    }
    if (lowerValue === 0) {
      return lower;
    }
    if (upperValue === 0) {
      return upper;
    }

    normalVolatility = (lower + upper) / 2;
    if (Math.abs(upper - lower) <= tolerance) {
      return normalVolatility;
    }
  }

  return normalVolatility;
}
