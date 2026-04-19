import { fail } from '../error';
import { normalCdf } from '../numeric';
import type { OptionRight } from '../types';

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

export function price(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  optionRight: OptionRight;
  averageStyle: 'continuous';
}): number {
  ensurePositive('spot', input.spot);
  ensurePositive('strike', input.strike);
  ensurePositive('years', input.years);
  ensureFinite('rate', input.rate);
  ensureFinite('dividendYield', input.dividendYield);
  ensureNonNegative('volatility', input.volatility);
  const optionRight = validateOptionRight(input.optionRight);

  if (input.averageStyle !== 'continuous') {
    fail('unsupported_math_input', `unsupported average_style: ${input.averageStyle}`);
  }

  const sign = optionRight === 'call' ? 1 : -1;
  const variance = input.volatility * input.volatility * input.years / 3;

  if (variance === 0) {
    const meanLevel = input.spot * Math.exp((input.rate - input.dividendYield) * input.years / 2);
    return Math.exp(-input.rate * input.years) * Math.max(sign * (meanLevel - input.strike), 0);
  }

  const meanLn = Math.log(input.spot)
    + (input.rate - input.dividendYield - 0.5 * input.volatility * input.volatility) * input.years / 2;
  const stdDev = Math.sqrt(variance);
  const d1 = (meanLn - Math.log(input.strike) + variance) / stdDev;
  const d2 = d1 - stdDev;
  const expectedAverage = Math.exp(meanLn + 0.5 * variance);

  return Math.exp(-input.rate * input.years) * sign * (
    expectedAverage * normalCdf(sign * d1) - input.strike * normalCdf(sign * d2)
  );
}
