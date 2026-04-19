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
  const d2Lower = (Math.log(input.spot / input.lowerPrice)
    + (input.rate - input.dividendYield - 0.5 * input.volatility * input.volatility) * input.years)
    / sigmaSqrtT;
  const d2Upper = (Math.log(input.spot / input.upperPrice)
    + (input.rate - input.dividendYield - 0.5 * input.volatility * input.volatility) * input.years)
    / sigmaSqrtT;

  return normalCdf(d2Lower) - normalCdf(d2Upper);
}
