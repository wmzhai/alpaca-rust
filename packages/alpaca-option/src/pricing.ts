import * as optionContract from './contract';
import { fail } from './error';
import { bjerksundStensland2002Price } from './math/american';
import { brentSolve, normalCdf, normalPdf } from './numeric';
import type {
  BlackScholesImpliedVolatilityInput,
  BlackScholesInput,
  Greeks,
  OptionRight,
} from './types';

type ContractInput = optionContract.ContractInput;

function ensureFinite(name: string, value: number): void {
  if (!Number.isFinite(value)) {
    fail('invalid_pricing_input', `${name} must be finite: ${value}`);
  }
}

function ensurePositive(name: string, value: number): void {
  ensureFinite(name, value);
  if (value <= 0) {
    fail('invalid_pricing_input', `${name} must be greater than zero: ${value}`);
  }
}

function validateOptionRight(optionRight: OptionRight): OptionRight {
  if (optionRight !== 'call' && optionRight !== 'put') {
    fail('invalid_pricing_input', `invalid option right: ${optionRight}`);
  }
  return optionRight;
}

function validateBlackScholesInput(input: BlackScholesInput): OptionRight {
  ensurePositive('spot', input.spot);
  ensurePositive('strike', input.strike);
  ensurePositive('years', input.years);
  ensureFinite('rate', input.rate);
  ensureFinite('dividendYield', input.dividendYield);
  ensurePositive('volatility', input.volatility);
  return validateOptionRight(input.optionRight);
}

function d1d2(input: BlackScholesInput): { d1: number; d2: number } {
  const sqrtYears = Math.sqrt(input.years);
  const sigmaSqrtT = input.volatility * sqrtYears;
  const d1 = (Math.log(input.spot / input.strike)
    + (input.rate - input.dividendYield + 0.5 * input.volatility * input.volatility) * input.years)
    / sigmaSqrtT;
  return { d1, d2: d1 - sigmaSqrtT };
}

function priceBlackScholesCore(input: BlackScholesInput): number {
  const { d1, d2 } = d1d2(input);
  const discountSpot = Math.exp(-input.dividendYield * input.years);
  const discountStrike = Math.exp(-input.rate * input.years);

  if (input.optionRight === 'call') {
    return input.spot * discountSpot * normalCdf(d1)
      - input.strike * discountStrike * normalCdf(d2);
  }
  return input.strike * discountStrike * normalCdf(-d2)
    - input.spot * discountSpot * normalCdf(-d1);
}

function discountedForwardMinusStrike(input: Pick<BlackScholesInput, 'spot' | 'strike' | 'years' | 'rate' | 'dividendYield'>): number {
  return input.spot * Math.exp(-input.dividendYield * input.years)
    - input.strike * Math.exp(-input.rate * input.years);
}

function europeanNoArbitrageLowerBound(input: Pick<BlackScholesInput, 'spot' | 'strike' | 'years' | 'rate' | 'dividendYield' | 'optionRight'>): number {
  const parity = discountedForwardMinusStrike(input);
  return input.optionRight === 'call' ? Math.max(parity, 0) : Math.max(-parity, 0);
}

export function priceBlackScholes(input: BlackScholesInput): number {
  const optionRight = validateBlackScholesInput(input);
  return priceBlackScholesCore({ ...input, optionRight });
}

export function priceDefault(input: BlackScholesInput): number {
  validateBlackScholesInput(input);
  return bjerksundStensland2002Price(input);
}

export function greeksBlackScholes(input: BlackScholesInput): Greeks {
  const optionRight = validateBlackScholesInput(input);
  const { d1, d2 } = d1d2(input);
  const sqrtYears = Math.sqrt(input.years);
  const sigmaSqrtT = input.volatility * sqrtYears;
  const expMinusQt = Math.exp(-input.dividendYield * input.years);
  const expMinusRt = Math.exp(-input.rate * input.years);
  const nd1 = normalCdf(d1);
  const nd2 = normalCdf(d2);
  const nMinusD1 = normalCdf(-d1);
  const nMinusD2 = normalCdf(-d2);
  const phiD1 = normalPdf(d1);

  const delta = optionRight === 'call' ? expMinusQt * nd1 : -expMinusQt * nMinusD1;
  const gamma = expMinusQt * phiD1 / (input.spot * sigmaSqrtT);
  const vega = input.spot * expMinusQt * phiD1 * sqrtYears / 100;
  const thetaAnnual = optionRight === 'call'
    ? -input.spot * expMinusQt * phiD1 * input.volatility / (2 * sqrtYears)
      - input.rate * input.strike * expMinusRt * nd2
      + input.dividendYield * input.spot * expMinusQt * nd1
    : -input.spot * expMinusQt * phiD1 * input.volatility / (2 * sqrtYears)
      + input.rate * input.strike * expMinusRt * nMinusD2
      - input.dividendYield * input.spot * expMinusQt * nMinusD1;
  const theta = thetaAnnual / 365;
  const rho = optionRight === 'call'
    ? input.strike * input.years * expMinusRt * nd2
    : -input.strike * input.years * expMinusRt * nMinusD2;

  return { delta, gamma, vega, theta, rho };
}

function defaultPriceOrIntrinsic(
  input: BlackScholesInput,
  spot: number,
  years: number,
  volatility: number,
  rate: number,
): number {
  if (years <= 0) {
    return intrinsicValue(spot, input.strike, input.optionRight);
  }
  return priceDefault({ ...input, spot, years, volatility, rate });
}

export type GreekName = 'delta' | 'gamma' | 'vega' | 'theta' | 'rho';

export function greekDefault(input: BlackScholesInput, name: GreekName): number {
  if (!(input.years > 0)) {
    if (name !== 'delta') {
      return 0;
    }
    if (input.optionRight === 'call') {
      if (input.spot > input.strike) return 1;
      if (input.spot < input.strike) return 0;
      return 0.5;
    }
    if (input.spot < input.strike) return -1;
    if (input.spot > input.strike) return 0;
    return -0.5;
  }
  validateBlackScholesInput(input);
  if (name === 'delta' || name === 'gamma') {
    const spotStep = 0.01;
    const up = defaultPriceOrIntrinsic(input, input.spot + spotStep, input.years, input.volatility, input.rate);
    const down = defaultPriceOrIntrinsic(input, input.spot - spotStep, input.years, input.volatility, input.rate);
    if (name === 'delta') {
      return (up - down) / (2 * spotStep);
    }
    const mid = defaultPriceOrIntrinsic(input, input.spot, input.years, input.volatility, input.rate);
    return (up - 2 * mid + down) / (spotStep * spotStep);
  }
  if (name === 'theta') {
    const days = input.years * 365;
    const dayStep = Math.min(days / 24, 1 / 24);
    const yearStep = dayStep / 365;
    const sooner = defaultPriceOrIntrinsic(input, input.spot, input.years - yearStep, input.volatility, input.rate);
    const later = defaultPriceOrIntrinsic(input, input.spot, input.years + yearStep, input.volatility, input.rate);
    return (sooner - later) / (2 * dayStep);
  }
  if (name === 'vega') {
    const volStep = 0.01;
    const volUp = defaultPriceOrIntrinsic(input, input.spot, input.years, input.volatility + volStep, input.rate);
    const volDown = defaultPriceOrIntrinsic(input, input.spot, input.years, input.volatility - volStep, input.rate);
    return (volUp - volDown) / 2;
  }
  const rateStep = 0.001;
  const rateUp = defaultPriceOrIntrinsic(input, input.spot, input.years, input.volatility, input.rate + rateStep);
  const rateDown = defaultPriceOrIntrinsic(input, input.spot, input.years, input.volatility, input.rate - rateStep);
  return (rateUp - rateDown) / 2 / 0.1;
}

export function greeksDefault(input: BlackScholesInput): Greeks {
  validateBlackScholesInput(input);
  return {
    delta: greekDefault(input, 'delta'),
    gamma: greekDefault(input, 'gamma'),
    vega: greekDefault(input, 'vega'),
    theta: greekDefault(input, 'theta'),
    rho: greekDefault(input, 'rho'),
  };
}

export function intrinsicValue(spot: number, strike: number, optionRight: OptionRight): number {
  ensureFinite('spot', spot);
  ensurePositive('strike', strike);
  validateOptionRight(optionRight);
  return optionRight === 'call'
    ? Math.max(spot - strike, 0)
    : Math.max(strike - spot, 0);
}

export function extrinsicValue(
  optionPrice: number,
  spot: number,
  strike: number,
  optionRight: OptionRight,
): number {
  ensureFinite('optionPrice', optionPrice);
  return Math.max(optionPrice - intrinsicValue(spot, strike, optionRight), 0);
}

export function contractExtrinsicValue(
  optionPriceInput: number | string | null,
  spotInput: number | string | null,
  contractInput?: ContractInput,
): number | null {
  const optionPrice = typeof optionPriceInput === 'number' ? optionPriceInput : Number(optionPriceInput);
  const spot = typeof spotInput === 'number' ? spotInput : Number(spotInput);
  const contract = optionContract.canonicalContract(contractInput);
  if (!Number.isFinite(optionPrice) || !Number.isFinite(spot) || !contract) {
    return null;
  }

  return extrinsicValue(optionPrice, spot, contract.strike, contract.option_right);
}

export function impliedVolatilityBlackScholes(input: BlackScholesImpliedVolatilityInput): number {
  ensureFinite('targetPrice', input.targetPrice);
  validateBlackScholesInput({ ...input, volatility: 0.2 });

  const minimumPrice = europeanNoArbitrageLowerBound({
    spot: input.spot,
    strike: input.strike,
    years: input.years,
    rate: input.rate,
    dividendYield: input.dividendYield,
    optionRight: input.optionRight,
  });
  if (input.targetPrice + 1e-12 < minimumPrice) {
    fail(
      'invalid_pricing_input',
      `targetPrice is below discounted no-arbitrage lower bound: ${input.targetPrice} < ${minimumPrice}`,
    );
  }

  const lowerBound = input.lowerBound ?? 0.0001;
  const upperBound = input.upperBound ?? 5.0;
  ensurePositive('lowerBound', lowerBound);
  ensurePositive('upperBound', upperBound);
  if (lowerBound >= upperBound) {
    fail('invalid_pricing_input', `lowerBound must be less than upperBound: ${lowerBound} >= ${upperBound}`);
  }

  return brentSolve(
    lowerBound,
    upperBound,
    (volatility) => priceBlackScholesCore({ ...input, volatility }) - input.targetPrice,
    input.tolerance,
    input.maxIterations,
  );
}

export function impliedVolatilityFromPrice(input: BlackScholesImpliedVolatilityInput): number {
  ensureFinite('targetPrice', input.targetPrice);
  validateBlackScholesInput({ ...input, volatility: 0.2 });

  const minimumPrice = Math.max(
    europeanNoArbitrageLowerBound({
      spot: input.spot,
      strike: input.strike,
      years: input.years,
      rate: input.rate,
      dividendYield: input.dividendYield,
      optionRight: input.optionRight,
    }),
    intrinsicValue(input.spot, input.strike, input.optionRight),
  );
  if (input.targetPrice + 1e-12 < minimumPrice) {
    fail(
      'invalid_pricing_input',
      `targetPrice is below discounted no-arbitrage lower bound: ${input.targetPrice} < ${minimumPrice}`,
    );
  }

  const lowerBound = input.lowerBound ?? 0.0001;
  const upperBound = input.upperBound ?? 20.0;
  ensurePositive('lowerBound', lowerBound);
  ensurePositive('upperBound', upperBound);
  if (lowerBound >= upperBound) {
    fail('invalid_pricing_input', `lowerBound must be less than upperBound: ${lowerBound} >= ${upperBound}`);
  }

  return brentSolve(
    lowerBound,
    upperBound,
    (volatility) => priceDefault({ ...input, volatility }) - input.targetPrice,
    input.tolerance,
    input.maxIterations,
  );
}
