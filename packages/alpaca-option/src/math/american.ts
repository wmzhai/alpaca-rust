import { fail } from '../error';
import { normalCdf, normalPdf } from '../numeric';
import type { OptionRight } from '../types';
import { roundToFixtureYears } from './shared';

export type CashDividend = {
  time: number;
  amount: number;
};

export type CashDividendModel = 'spot' | 'escrowed';

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

function validateOptionRight(optionRight: OptionRight): OptionRight {
  if (optionRight !== 'call' && optionRight !== 'put') {
    fail('invalid_math_input', `invalid option right: ${optionRight}`);
  }
  return optionRight;
}

function validateInput(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  optionRight: OptionRight;
}): OptionRight {
  ensurePositive('spot', input.spot);
  ensurePositive('strike', input.strike);
  ensurePositive('years', input.years);
  ensureFinite('rate', input.rate);
  ensureFinite('dividendYield', input.dividendYield);
  ensurePositive('volatility', input.volatility);
  return validateOptionRight(input.optionRight);
}

function carry(rate: number, dividendYield: number): number {
  return rate - dividendYield;
}

function discount(rate: number, years: number): number {
  return Math.exp(-rate * years);
}

function intrinsic(optionRight: OptionRight, spot: number, strike: number): number {
  return optionRight === 'call'
    ? Math.max(spot - strike, 0)
    : Math.max(strike - spot, 0);
}

function gbsD1D2(input: {
  spot: number;
  strike: number;
  years: number;
  carry: number;
  volatility: number;
}): { d1: number; d2: number } {
  const sqrtYears = Math.sqrt(input.years);
  const sigmaSqrtT = input.volatility * sqrtYears;
  const d1 = (Math.log(input.spot / input.strike) + (input.carry + 0.5 * input.volatility * input.volatility) * input.years)
    / sigmaSqrtT;
  return { d1, d2: d1 - sigmaSqrtT };
}

function gbsPrice(input: {
  optionRight: OptionRight;
  spot: number;
  strike: number;
  years: number;
  rate: number;
  carry: number;
  volatility: number;
}): number {
  const dividendDiscount = Math.exp((input.carry - input.rate) * input.years);
  const riskFreeDiscount = discount(input.rate, input.years);
  if (input.volatility === 0) {
    const forward = input.spot * Math.exp(input.carry * input.years);
    return riskFreeDiscount * intrinsic(input.optionRight, forward, input.strike);
  }

  const sign = input.optionRight === 'call' ? 1 : -1;
  const { d1, d2 } = gbsD1D2(input);
  return sign * (
    input.spot * dividendDiscount * normalCdf(sign * d1)
      - input.strike * riskFreeDiscount * normalCdf(sign * d2)
  );
}

function criticalPrice(input: {
  optionRight: OptionRight;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  tolerance: number;
}): number {
  const variance = input.volatility * input.volatility * input.years;
  const riskFreeDiscount = discount(input.rate, input.years);
  if (riskFreeDiscount > 1 + 1e-12) {
    fail('unsupported_math_input', 'american approximation does not support negative rates');
  }
  const dividendDiscount = Math.exp(-input.dividendYield * input.years);
  const sqrtVariance = Math.sqrt(variance);
  const n = 2 * Math.log(dividendDiscount / riskFreeDiscount) / variance;
  const m = -2 * Math.log(riskFreeDiscount) / variance;
  const carryTerm = Math.log(dividendDiscount / riskFreeDiscount);

  let si: number;
  if (input.optionRight === 'call') {
    const qu = (-(n - 1) + Math.sqrt((n - 1) * (n - 1) + 4 * m)) / 2;
    const su = input.strike / (1 - 1 / qu);
    const h = -(carryTerm + 2 * sqrtVariance) * input.strike / (su - input.strike);
    si = input.strike + (su - input.strike) * (1 - Math.exp(h));
  } else {
    const qu = (-(n - 1) - Math.sqrt((n - 1) * (n - 1) + 4 * m)) / 2;
    const su = input.strike / (1 - 1 / qu);
    const h = (carryTerm - 2 * sqrtVariance) * input.strike / (input.strike - su);
    si = su + (input.strike - su) * Math.exp(h);
  }

  const kappa = Math.abs(riskFreeDiscount - 1) > 1e-12
    ? -2 * Math.log(riskFreeDiscount) / (variance * (1 - riskFreeDiscount))
    : 2 / variance;

  let forwardSi = si * dividendDiscount / riskFreeDiscount;
  let d1 = Math.log(forwardSi / input.strike) / sqrtVariance + 0.5 * sqrtVariance;
  let temp = gbsPrice({
    optionRight: input.optionRight,
    spot: si,
    strike: input.strike,
    years: input.years,
    rate: input.rate,
    carry: carry(input.rate, input.dividendYield),
    volatility: input.volatility,
  });

  if (input.optionRight === 'call') {
    const q = (-(n - 1) + Math.sqrt((n - 1) * (n - 1) + 4 * kappa)) / 2;
    let lhs = si - input.strike;
    let rhs = temp + (1 - dividendDiscount * normalCdf(d1)) * si / q;
    let bi = dividendDiscount * normalCdf(d1) * (1 - 1 / q)
      + (1 - dividendDiscount * normalPdf(d1) / sqrtVariance) / q;
    while (Math.abs(lhs - rhs) / input.strike > input.tolerance) {
      si = (input.strike + rhs - bi * si) / (1 - bi);
      forwardSi = si * dividendDiscount / riskFreeDiscount;
      d1 = Math.log(forwardSi / input.strike) / sqrtVariance + 0.5 * sqrtVariance;
      lhs = si - input.strike;
      temp = gbsPrice({
        optionRight: input.optionRight,
        spot: si,
        strike: input.strike,
        years: input.years,
        rate: input.rate,
        carry: carry(input.rate, input.dividendYield),
        volatility: input.volatility,
      });
      rhs = temp + (1 - dividendDiscount * normalCdf(d1)) * si / q;
      bi = dividendDiscount * normalCdf(d1) * (1 - 1 / q)
        + (1 - dividendDiscount * normalPdf(d1) / sqrtVariance) / q;
    }
  } else {
    const q = (-(n - 1) - Math.sqrt((n - 1) * (n - 1) + 4 * kappa)) / 2;
    let lhs = input.strike - si;
    let rhs = temp - (1 - dividendDiscount * normalCdf(-d1)) * si / q;
    let bi = -dividendDiscount * normalCdf(-d1) * (1 - 1 / q)
      - (1 + dividendDiscount * normalPdf(d1) / sqrtVariance) / q;
    while (Math.abs(lhs - rhs) / input.strike > input.tolerance) {
      si = (input.strike - rhs + bi * si) / (1 + bi);
      forwardSi = si * dividendDiscount / riskFreeDiscount;
      d1 = Math.log(forwardSi / input.strike) / sqrtVariance + 0.5 * sqrtVariance;
      lhs = input.strike - si;
      temp = gbsPrice({
        optionRight: input.optionRight,
        spot: si,
        strike: input.strike,
        years: input.years,
        rate: input.rate,
        carry: carry(input.rate, input.dividendYield),
        volatility: input.volatility,
      });
      rhs = temp - (1 - dividendDiscount * normalCdf(-d1)) * si / q;
      bi = -dividendDiscount * normalCdf(-d1) * (1 - 1 / q)
        - (1 + dividendDiscount * normalPdf(d1) / sqrtVariance) / q;
    }
  }

  return si;
}

function bs1993Phi(input: {
  spot: number;
  years: number;
  gamma: number;
  h: number;
  i: number;
  rate: number;
  carry: number;
  volatility: number;
}): number {
  const sqrtYears = Math.sqrt(input.years);
  const sigmaSq = input.volatility * input.volatility;
  const lambda = (-input.rate + input.gamma * input.carry + 0.5 * input.gamma * (input.gamma - 1) * sigmaSq) * input.years;
  const d = -(Math.log(input.spot / input.h) + (input.carry + (input.gamma - 0.5) * sigmaSq) * input.years)
    / (input.volatility * sqrtYears);
  const kappa = 2 * input.carry / sigmaSq + 2 * input.gamma - 1;
  return Math.exp(lambda) * (input.spot ** input.gamma) * (
    normalCdf(d)
      - ((input.i / input.spot) ** kappa) * normalCdf(d - 2 * Math.log(input.i / input.spot) / (input.volatility * sqrtYears))
  );
}

function bs1993CallPrice(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  carry: number;
  volatility: number;
}): number {
  if (input.carry >= input.rate) {
    return gbsPrice({ ...input, optionRight: 'call' });
  }

  const sigmaSq = input.volatility * input.volatility;
  const beta = (0.5 - input.carry / sigmaSq)
    + Math.sqrt((input.carry / sigmaSq - 0.5) ** 2 + 2 * input.rate / sigmaSq);
  const bInfinity = beta / (beta - 1) * input.strike;
  const b0 = Math.max(input.strike, input.rate / (input.rate - input.carry) * input.strike);
  const ht = -(input.carry * input.years + 2 * input.volatility * Math.sqrt(input.years)) * b0 / (bInfinity - b0);
  const i = b0 + (bInfinity - b0) * (1 - Math.exp(ht));
  const alpha = (i - input.strike) * (i ** (-beta));
  if (input.spot >= i) {
    return input.spot - input.strike;
  }

  return alpha * (input.spot ** beta)
    - alpha * bs1993Phi({ spot: input.spot, years: input.years, gamma: beta, h: i, i, rate: input.rate, carry: input.carry, volatility: input.volatility })
    + bs1993Phi({ spot: input.spot, years: input.years, gamma: 1, h: i, i, rate: input.rate, carry: input.carry, volatility: input.volatility })
    - bs1993Phi({ spot: input.spot, years: input.years, gamma: 1, h: input.strike, i, rate: input.rate, carry: input.carry, volatility: input.volatility })
    - input.strike * bs1993Phi({ spot: input.spot, years: input.years, gamma: 0, h: i, i, rate: input.rate, carry: input.carry, volatility: input.volatility })
    + input.strike * bs1993Phi({ spot: input.spot, years: input.years, gamma: 0, h: input.strike, i, rate: input.rate, carry: input.carry, volatility: input.volatility });
}

function juQuadraticPriceInner(input: {
  optionRight: OptionRight;
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
}): number {
  const dividendDiscount = Math.exp(-input.dividendYield * input.years);
  if (dividendDiscount >= 1 && input.optionRight === 'call') {
    return gbsPrice({ ...input, carry: carry(input.rate, input.dividendYield) });
  }

  const variance = input.volatility * input.volatility * input.years;
  const sqrtVariance = Math.sqrt(variance);
  const riskFreeDiscount = discount(input.rate, input.years);
  if (riskFreeDiscount > 1 + 1e-12) {
    fail('unsupported_math_input', 'ju quadratic approximation does not support negative rates');
  }

  const europeanPrice = gbsPrice({ ...input, carry: carry(input.rate, input.dividendYield) });
  const sk = criticalPrice({
    optionRight: input.optionRight,
    strike: input.strike,
    years: input.years,
    rate: input.rate,
    dividendYield: input.dividendYield,
    volatility: input.volatility,
    tolerance: 1e-6,
  });
  if (Math.abs(1 - riskFreeDiscount) < 1e-12 || Math.abs(Math.log(riskFreeDiscount)) < 1e-12) {
    return baroneAdesiWhaleyPrice(input);
  }

  const forwardPrice = input.spot * dividendDiscount / riskFreeDiscount;
  const forwardSk = sk * dividendDiscount / riskFreeDiscount;
  const alpha = -2 * Math.log(riskFreeDiscount) / variance;
  const beta = 2 * Math.log(dividendDiscount / riskFreeDiscount) / variance;
  const h = 1 - riskFreeDiscount;
  const phi = input.optionRight === 'call' ? 1 : -1;
  const tempRoot = Math.sqrt((beta - 1) * (beta - 1) + (4 * alpha) / h);
  const lambda = (-(beta - 1) + phi * tempRoot) / 2;
  const lambdaPrime = -phi * alpha / (h * h * tempRoot);
  const blackSk = gbsPrice({
    optionRight: input.optionRight,
    spot: sk,
    strike: input.strike,
    years: input.years,
    rate: input.rate,
    carry: carry(input.rate, input.dividendYield),
    volatility: input.volatility,
  });
  const hA = phi * (sk - input.strike) - blackSk;
  const d1Sk = (Math.log(forwardSk / input.strike) + 0.5 * variance) / sqrtVariance;
  const d2Sk = d1Sk - sqrtVariance;
  const vEh = forwardSk * normalPdf(d1Sk) / (alpha * sqrtVariance)
    - phi * forwardSk * normalCdf(phi * d1Sk) * Math.log(dividendDiscount) / Math.log(riskFreeDiscount)
    + phi * input.strike * normalCdf(phi * d2Sk);
  const denominator = 2 * lambda + beta - 1;
  const b = (1 - h) * alpha * lambdaPrime / (2 * denominator);
  const c = -((1 - h) * alpha / denominator) * (vEh / hA + 1 / h + lambdaPrime / denominator);
  const spotRatio = Math.log(input.spot / sk);
  const chi = spotRatio * (b * spotRatio + c);

  if (phi * (sk - input.spot) > 0) {
    return europeanPrice + hA * ((input.spot / sk) ** lambda) / (1 - chi);
  }
  return phi * (input.spot - input.strike);
}

export function treePrice(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  optionRight: OptionRight;
  steps?: number;
  useRichardson?: boolean;
}): number {
  const optionRight = validateInput(input);
  if (optionRight === 'call' && input.dividendYield <= 0) {
    return gbsPrice({ ...input, optionRight, carry: carry(input.rate, input.dividendYield) });
  }

  const treeOnce = (steps: number): number => {
    if (!Number.isInteger(steps) || steps < 2) {
      fail('invalid_math_input', `steps must be at least 2: ${steps}`);
    }

    const dt = input.years / steps;
    const growth = Math.exp((input.rate - input.dividendYield) * dt);
    const up = Math.exp(input.volatility * Math.sqrt(dt));
    const down = 1 / up;
    const probability = (growth - down) / (up - down);
    if (!(probability > 0 && probability < 1)) {
      fail('invalid_math_input', `tree probability is out of bounds: ${probability}`);
    }

    const discountStep = Math.exp(-input.rate * dt);
    const values = Array.from({ length: steps + 1 }, (_, index) => {
      const stock = input.spot * (down ** (steps - index)) * (up ** index);
      return intrinsic(optionRight, stock, input.strike);
    });

    for (let level = steps; level >= 1; level -= 1) {
      for (let index = 0; index < level; index += 1) {
        const continuation = discountStep * (probability * values[index + 1] + (1 - probability) * values[index]);
        const stock = input.spot * (down ** (level - 1 - index)) * (up ** index);
        values[index] = Math.max(continuation, intrinsic(optionRight, stock, input.strike));
      }
    }

    return values[0];
  };

  let baseSteps = Math.max(input.steps ?? 4000, 200);
  if (baseSteps % 2 === 1) {
    baseSteps += 1;
  }

  const coarse = treeOnce(baseSteps);
  if (input.useRichardson === false) {
    return coarse;
  }
  const fine = treeOnce(baseSteps * 2);
  return 2 * fine - coarse;
}

export function baroneAdesiWhaleyPrice(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  optionRight: OptionRight;
}): number {
  const optionRight = validateInput(input);
  const assetCarry = carry(input.rate, input.dividendYield);
  if (optionRight === 'call' && assetCarry >= input.rate) {
    return gbsPrice({ ...input, optionRight, carry: assetCarry });
  }

  const variance = input.volatility * input.volatility * input.years;
  const sqrtVariance = Math.sqrt(variance);
  const riskFreeDiscount = discount(input.rate, input.years);
  if (riskFreeDiscount > 1 + 1e-12) {
    fail('unsupported_math_input', 'barone-adesi-whaley does not support negative rates');
  }
  const dividendDiscount = Math.exp(-input.dividendYield * input.years);
  const sk = criticalPrice({ ...input, optionRight, tolerance: 1e-6 });
  const forwardSk = sk * dividendDiscount / riskFreeDiscount;
  const d1 = Math.log(forwardSk / input.strike) / sqrtVariance + 0.5 * sqrtVariance;
  const n = 2 * Math.log(dividendDiscount / riskFreeDiscount) / variance;
  const kappa = Math.abs(riskFreeDiscount - 1) > 1e-12
    ? -2 * Math.log(riskFreeDiscount) / (variance * (1 - riskFreeDiscount))
    : 2 / variance;

  if (optionRight === 'call') {
    const q = (-(n - 1) + Math.sqrt((n - 1) * (n - 1) + 4 * kappa)) / 2;
    const a = (sk / q) * (1 - dividendDiscount * normalCdf(d1));
    return input.spot < sk
      ? gbsPrice({ ...input, optionRight, carry: assetCarry }) + a * ((input.spot / sk) ** q)
      : input.spot - input.strike;
  }

  const q = (-(n - 1) - Math.sqrt((n - 1) * (n - 1) + 4 * kappa)) / 2;
  const a = -(sk / q) * (1 - dividendDiscount * normalCdf(-d1));
  return input.spot > sk
    ? gbsPrice({ ...input, optionRight, carry: assetCarry }) + a * ((input.spot / sk) ** q)
    : input.strike - input.spot;
}

export function bjerksundStensland1993Price(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  optionRight: OptionRight;
}): number {
  const optionRight = validateInput(input);
  const assetCarry = carry(input.rate, input.dividendYield);
  return optionRight === 'call'
    ? bs1993CallPrice({
        spot: input.spot,
        strike: input.strike,
        years: input.years,
        rate: input.rate,
        carry: assetCarry,
        volatility: input.volatility,
      })
    : bs1993CallPrice({
        spot: input.strike,
        strike: input.spot,
        years: input.years,
        rate: input.dividendYield,
        carry: -assetCarry,
        volatility: input.volatility,
      });
}

export function juQuadraticPrice(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  optionRight: OptionRight;
}): number {
  const optionRight = validateInput(input);
  return juQuadraticPriceInner({ ...input, optionRight });
}

const DISCRETE_DIVIDEND_TIME_STEPS_PER_YEAR = 300;
const DISCRETE_DIVIDEND_SPACE_STEPS = 300;

function validateCashDividendModel(model: CashDividendModel): CashDividendModel {
  if (model !== 'spot' && model !== 'escrowed') {
    fail('invalid_math_input', `invalid cash_dividend_model: ${model}`);
  }
  return model;
}

function validateCashDividends(dividends: CashDividend[], years: number): CashDividend[] {
  if (dividends.length === 0) {
    fail('invalid_math_input', 'dividends must not be empty');
  }

  const normalized = [...dividends].sort((left, right) => left.time - right.time);
  const rounded: CashDividend[] = [];

  for (const dividend of normalized) {
    ensurePositive('dividend.time', dividend.time);
    ensurePositive('dividend.amount', dividend.amount);
    const roundedTime = roundToFixtureYears(dividend.time);
    if (roundedTime > years) {
      fail('invalid_math_input', `dividend.time exceeds years: ${roundedTime} > ${years}`);
    }

    const last = rounded[rounded.length - 1];
    if (last != null && Math.abs(last.time - roundedTime) <= 1e-12) {
      last.amount += dividend.amount;
      continue;
    }

    rounded.push({
      time: roundedTime,
      amount: dividend.amount,
    });
  }

  return rounded;
}

function remainingEscrowBalance(dividends: CashDividend[], rate: number, time: number): number {
  return dividends
    .filter((dividend) => dividend.time + 1e-12 >= time)
    .reduce((sum, dividend) => sum + dividend.amount * Math.exp(-rate * (dividend.time - time)), 0);
}

function interpolateLinear(xGrid: number[], yGrid: number[], x: number): number {
  if (x <= xGrid[0]) {
    return yGrid[0];
  }
  if (x >= xGrid[xGrid.length - 1]) {
    return yGrid[yGrid.length - 1];
  }

  let upper = 0;
  while (upper < xGrid.length && xGrid[upper] <= x) {
    upper += 1;
  }
  const lower = upper - 1;
  const x0 = xGrid[lower];
  const x1 = xGrid[upper];
  const y0 = yGrid[lower];
  const y1 = yGrid[upper];
  return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
}

function actualSpot(
  stateSpot: number,
  rate: number,
  time: number,
  dividends: CashDividend[],
  model: CashDividendModel,
): number {
  return model === 'spot'
    ? stateSpot
    : stateSpot + remainingEscrowBalance(dividends, rate, time);
}

function dividendStepValue(input: {
  xGrid: number[];
  valuesAfter: number[];
  strike: number;
  optionRight: OptionRight;
  rate: number;
  dividendTime: number;
  dividendAmount: number;
  dividends: CashDividend[];
  model: CashDividendModel;
}): number[] {
  const balance = remainingEscrowBalance(input.dividends, input.rate, input.dividendTime);
  return input.xGrid.map((stateSpot) => {
    const continuation = input.model === 'spot'
      ? interpolateLinear(input.xGrid, input.valuesAfter, Math.max(stateSpot - input.dividendAmount, input.xGrid[0]))
      : interpolateLinear(input.xGrid, input.valuesAfter, stateSpot);
    const exercise = intrinsic(
      input.optionRight,
      input.model === 'spot' ? stateSpot : stateSpot + balance,
      input.strike,
    );
    return Math.max(continuation, exercise);
  });
}

function implicitCrankNicolsonStep(input: {
  xGrid: number[];
  values: number[];
  dt: number;
  strike: number;
  optionRight: OptionRight;
  rate: number;
  volatility: number;
  dividends: CashDividend[];
  model: CashDividendModel;
  time: number;
}): number[] {
  const dx = input.xGrid[1] - input.xGrid[0];
  const last = input.xGrid.length - 1;
  const balance = input.model === 'spot' ? 0 : remainingEscrowBalance(input.dividends, input.rate, input.time);
  const lowerBoundary = intrinsic(
    input.optionRight,
    input.model === 'spot' ? 0 : balance,
    input.strike,
  );
  const upperBoundary = intrinsic(input.optionRight, input.xGrid[last] + balance, input.strike);

  const lower = Array.from({ length: last - 1 }, () => 0);
  const diag = Array.from({ length: last - 1 }, () => 0);
  const upper = Array.from({ length: last - 1 }, () => 0);
  const rhs = Array.from({ length: last - 1 }, () => 0);

  for (let index = 1; index < last; index += 1) {
    const stateSpot = input.xGrid[index];
    const diffusion = 0.5 * input.volatility * input.volatility * stateSpot * stateSpot / (dx * dx);
    const drift = 0.5 * input.rate * stateSpot / dx;
    const left = diffusion - drift;
    const center = -2 * diffusion - input.rate;
    const right = diffusion + drift;

    const row = index - 1;
    lower[row] = -0.5 * input.dt * left;
    diag[row] = 1 - 0.5 * input.dt * center;
    upper[row] = -0.5 * input.dt * right;
    rhs[row] = (1 + 0.5 * input.dt * center) * input.values[index]
      + 0.5 * input.dt * (left * input.values[index - 1] + right * input.values[index + 1]);
  }

  rhs[0] -= lower[0] * lowerBoundary;
  rhs[last - 2] -= upper[last - 2] * upperBoundary;

  for (let row = 1; row < last - 1; row += 1) {
    const weight = lower[row] / diag[row - 1];
    diag[row] -= weight * upper[row - 1];
    rhs[row] -= weight * rhs[row - 1];
  }

  const next = Array.from({ length: input.xGrid.length }, () => 0);
  next[0] = lowerBoundary;
  next[last] = upperBoundary;
  next[last - 1] = rhs[last - 2] / diag[last - 2];

  for (let row = last - 3; row >= 0; row -= 1) {
    next[row + 1] = (rhs[row] - upper[row] * next[row + 2]) / diag[row];
  }

  for (let index = 1; index < last; index += 1) {
    const exercise = intrinsic(
      input.optionRight,
      actualSpot(input.xGrid[index], input.rate, input.time, input.dividends, input.model),
      input.strike,
    );
    next[index] = Math.max(next[index], exercise);
  }

  return next;
}

export function discreteDividendPrice(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  volatility: number;
  optionRight: OptionRight;
  cashDividendModel: CashDividendModel;
  dividends: CashDividend[];
}): number {
  ensurePositive('spot', input.spot);
  ensurePositive('strike', input.strike);
  ensurePositive('years', input.years);
  ensureFinite('rate', input.rate);
  ensurePositive('volatility', input.volatility);
  const optionRight = validateOptionRight(input.optionRight);
  const cashDividendModel = validateCashDividendModel(input.cashDividendModel);
  const years = roundToFixtureYears(input.years);
  const dividends = validateCashDividends(input.dividends, years);

  const initialBalance = cashDividendModel === 'spot' ? 0 : remainingEscrowBalance(dividends, input.rate, 0);
  const stateSpot = input.spot - initialBalance;
  if (stateSpot <= 0) {
    fail('invalid_math_input', 'spot minus escrowed dividends must remain positive');
  }

  const referenceSpot = Math.max(
    input.spot,
    input.strike,
    stateSpot + remainingEscrowBalance(dividends, input.rate, 0),
  );
  const stateUpper = Math.max(referenceSpot * 4, 1);
  const dx = stateUpper / DISCRETE_DIVIDEND_SPACE_STEPS;
  const xGrid = Array.from({ length: DISCRETE_DIVIDEND_SPACE_STEPS + 1 }, (_, index) => index * dx);

  let values = xGrid.map((state) => intrinsic(optionRight, state, input.strike));

  const eventTimes = Array.from(new Set(
    dividends
      .filter((dividend) => dividend.time > 0 && dividend.time < years)
      .map((dividend) => dividend.time.toFixed(12)),
  ))
    .map((value) => Number(value))
    .sort((left, right) => left - right);

  const timeline = [0, ...eventTimes, years];

  for (let segment = timeline.length - 2; segment >= 0; segment -= 1) {
    const start = timeline[segment];
    const end = timeline[segment + 1];

    if (segment + 1 < timeline.length - 1) {
      const dividend = dividends.find((item) => Math.abs(item.time - end) <= 1e-12);
      if (dividend == null) {
        fail('invalid_math_input', `timeline dividend should exist at ${end}`);
      }
      values = dividendStepValue({
        xGrid,
        valuesAfter: values,
        strike: input.strike,
        optionRight,
        rate: input.rate,
        dividendTime: end,
        dividendAmount: dividend.amount,
        dividends,
        model: cashDividendModel,
      });
    }

    const subSteps = Math.max(Math.round((end - start) * DISCRETE_DIVIDEND_TIME_STEPS_PER_YEAR), 1);
    const dt = (end - start) / subSteps;

    for (let step = 0; step < subSteps; step += 1) {
      const time = end - (step + 1) * dt;
      values = implicitCrankNicolsonStep({
        xGrid,
        values,
        dt,
        strike: input.strike,
        optionRight,
        rate: input.rate,
        volatility: input.volatility,
        dividends,
        model: cashDividendModel,
        time,
      });
    }
  }

  return interpolateLinear(xGrid, values, stateSpot);
}
