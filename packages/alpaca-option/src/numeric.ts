import { fail } from './error';

const SQRT_2 = Math.sqrt(2);
const SQRT_2PI = Math.sqrt(2 * Math.PI);
const SQRT_PI = Math.sqrt(Math.PI);
const ERF_EPSILON = 1e-18;
const ERF_MAX_ITERATIONS = 200;

function ensureFinite(name: string, value: number): void {
  if (!Number.isFinite(value)) {
    fail('invalid_numeric_input', `${name} must be finite: ${value}`);
  }
}

export type RangeExtrema = {
  minSpot: number;
  minValue: number;
  maxSpot: number;
  maxValue: number;
};

function erfSeries(x: number): number {
  const sign = x < 0 ? -1 : 1;
  const absX = Math.abs(x);
  let term = absX;
  let total = absX;

  for (let n = 1; n < ERF_MAX_ITERATIONS; n += 1) {
    term *= -(absX * absX) / n;
    const delta = term / (2 * n + 1);
    total += delta;
    if (Math.abs(delta) < ERF_EPSILON) {
      break;
    }
  }

  return sign * 2 * total / SQRT_PI;
}

function normalCdfTail(x: number): number {
  let fraction = 0;
  for (let k = ERF_MAX_ITERATIONS; k >= 1; k -= 1) {
    fraction = k / (x + fraction);
  }
  return normalPdf(x) / (x + fraction);
}

export function normalCdf(x: number): number {
  if (x === 0) {
    return 0.5;
  }

  if (Math.abs(x) <= 4) {
    return 0.5 * (1 + erfSeries(x / SQRT_2));
  }

  const tail = normalCdfTail(Math.abs(x));
  return x > 0 ? 1 - tail : tail;
}

export function normalPdf(x: number): number {
  return Math.exp(-0.5 * x * x) / SQRT_2PI;
}

// Positive half of the 16-point Gauss-Legendre rule on [-1, 1].
// Keep these identical to the Rust implementation in crates/alpaca-option.
const GAUSS_16_NODE = [
  0.09501250983763769,
  0.2816035507792589,
  0.4580167776572271,
  0.6178762444026439,
  0.755404408355003,
  0.8656312023878316,
  0.9445750230732324,
  0.9894009349916503,
];
const GAUSS_16_WEIGHT = [
  0.18945061045506847,
  0.18260341504492356,
  0.16915651939500262,
  0.14959598881657668,
  0.12462897125553392,
  0.09515851168249283,
  0.06225352393864823,
  0.02715245941175309,
];

function bivariateAngleDensity(theta: number, product: number, halfSquare: number): number {
  const cosine = Math.cos(theta);
  const denominator = cosine * cosine;
  if (denominator <= 1e-18) {
    return 0;
  }
  return Math.exp((Math.sin(theta) * product - halfSquare) / denominator);
}

// P(X <= x, Y <= y) for standard normals with correlation rho.
// Drezner-Wesolowsky arcsine integral, same 16-point rule as the Rust crate.
export function bivariateNormalCdf(x: number, y: number, rho: number): number {
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(rho)) {
    return Number.NaN;
  }
  if (rho >= 0.999999) {
    return normalCdf(Math.min(x, y));
  }
  if (rho <= -0.999999) {
    return Math.max(normalCdf(x) - normalCdf(-y), 0);
  }
  if (x <= -8 || y <= -8) {
    return 0;
  }
  if (x >= 8 && y >= 8) {
    return 1;
  }

  const angle = Math.asin(rho);
  const halfSquare = 0.5 * (x * x + y * y);
  const product = x * y;
  let sum = 0;
  for (let index = 0; index < GAUSS_16_NODE.length; index += 1) {
    const node = GAUSS_16_NODE[index];
    const weight = GAUSS_16_WEIGHT[index];
    sum += weight * bivariateAngleDensity(angle * (node + 1) * 0.5, product, halfSquare);
    sum += weight * bivariateAngleDensity(angle * (1 - node) * 0.5, product, halfSquare);
  }
  const value = normalCdf(x) * normalCdf(y) + (angle * sum) / (4 * Math.PI);
  return Math.min(Math.max(value, 0), 1);
}

export function round(value: number, decimals: number): number {
  ensureFinite('value', value);
  if (!Number.isInteger(decimals) || decimals < 0) {
    fail('invalid_numeric_input', `decimals must be a non-negative integer: ${decimals}`);
  }
  const multiplier = 10 ** decimals;
  return Math.round(value * multiplier) / multiplier;
}

export function linspace(start: number, end: number, count: number): number[] {
  ensureFinite('start', start);
  ensureFinite('end', end);
  if (!Number.isInteger(count) || count <= 0) {
    fail('invalid_numeric_input', `count must be a positive integer: ${count}`);
  }
  if (count === 1) {
    return [start];
  }

  const step = (end - start) / (count - 1);
  return Array.from({ length: count }, (_, index) => start + step * index);
}

export function brentSolve(
  lowerBound: number,
  upperBound: number,
  evaluate: (x: number) => number,
  toleranceInput?: number,
  maxIterationsInput?: number,
): number {
  ensureFinite('lowerBound', lowerBound);
  ensureFinite('upperBound', upperBound);
  if (lowerBound >= upperBound) {
    fail('invalid_numeric_input', `lowerBound must be less than upperBound: ${lowerBound} >= ${upperBound}`);
  }

  const tolerance = toleranceInput ?? 1e-10;
  if (!Number.isFinite(tolerance) || tolerance <= 0) {
    fail('invalid_numeric_input', `tolerance must be positive: ${tolerance}`);
  }

  const maxIterations = maxIterationsInput ?? 100;
  if (!Number.isInteger(maxIterations) || maxIterations <= 0) {
    fail('invalid_numeric_input', `maxIterations must be a positive integer: ${maxIterations}`);
  }

  let a = lowerBound;
  let b = upperBound;
  let fa = evaluate(a);
  let fb = evaluate(b);
  ensureFinite('f(lowerBound)', fa);
  ensureFinite('f(upperBound)', fb);

  if (Math.abs(fa) <= tolerance) {
    return a;
  }
  if (Math.abs(fb) <= tolerance) {
    return b;
  }
  if (fa * fb > 0) {
    fail('root_not_bracketed', `root is not bracketed: f(${a})=${fa}, f(${b})=${fb}`);
  }

  if (Math.abs(fa) < Math.abs(fb)) {
    [a, b] = [b, a];
    [fa, fb] = [fb, fa];
  }

  let c = a;
  let fc = fa;
  let d = b - a;
  let mflag = true;

  for (let iteration = 0; iteration < maxIterations; iteration += 1) {
    let s = (fa !== fc && fb !== fc)
      ? (a * fb * fc) / ((fa - fb) * (fa - fc))
        + (b * fa * fc) / ((fb - fa) * (fb - fc))
        + (c * fa * fb) / ((fc - fa) * (fc - fb))
      : b - (fb * (b - a)) / (fb - fa);

    const lowerWindow = (3 * a + b) / 4;
    const outsideWindow = a < b
      ? s <= lowerWindow || s >= b
      : s >= lowerWindow || s <= b;
    const cond2 = mflag && Math.abs(s - b) >= Math.abs(b - c) / 2;
    const cond3 = !mflag && Math.abs(s - b) >= Math.abs(c - d) / 2;
    const cond4 = mflag && Math.abs(b - c) < tolerance;
    const cond5 = !mflag && Math.abs(c - d) < tolerance;

    if (outsideWindow || cond2 || cond3 || cond4 || cond5) {
      s = (a + b) / 2;
      mflag = true;
    } else {
      mflag = false;
    }

    const fs = evaluate(s);
    ensureFinite('f(candidate)', fs);

    d = c;
    c = b;
    fc = fb;

    if (fa * fs < 0) {
      b = s;
      fb = fs;
    } else {
      a = s;
      fa = fs;
    }

    if (Math.abs(fa) < Math.abs(fb)) {
      [a, b] = [b, a];
      [fa, fb] = [fb, fa];
    }

    if (Math.abs(fb) <= tolerance || Math.abs(b - a) <= tolerance) {
      return b;
    }
  }

  fail('root_not_converged', `root solver did not converge in ${maxIterations} iterations`);
}

export function refineBracketedRoot(
  lowerBound: number,
  upperBound: number,
  evaluate: (x: number) => number,
  toleranceInput?: number,
  maxIterationsInput?: number,
): number {
  return brentSolve(lowerBound, upperBound, evaluate, toleranceInput, maxIterationsInput);
}

export function evaluatePoints(points: number[], evaluate: (x: number) => number): number[] {
  return points.map((point) => {
    ensureFinite('point', point);
    const value = evaluate(point);
    ensureFinite('f(point)', value);
    return value;
  });
}

export function scanRangeExtrema(
  lowerBound: number,
  upperBound: number,
  stepInput: number | undefined,
  evaluate: (x: number) => number,
): RangeExtrema {
  ensureFinite('lowerBound', lowerBound);
  ensureFinite('upperBound', upperBound);
  if (lowerBound > upperBound) {
    fail('invalid_numeric_input', `lowerBound must be less than or equal to upperBound: ${lowerBound} > ${upperBound}`);
  }

  const step = stepInput ?? 1;
  if (!Number.isFinite(step) || step <= 0) {
    fail('invalid_numeric_input', `step must be positive: ${step}`);
  }

  let spot = lowerBound;
  let value = evaluate(spot);
  ensureFinite('f(lowerBound)', value);

  const extrema: RangeExtrema = {
    minSpot: spot,
    minValue: value,
    maxSpot: spot,
    maxValue: value,
  };

  while (spot < upperBound) {
    spot = Math.min(spot + step, upperBound);
    value = evaluate(spot);
    ensureFinite('f(point)', value);
    if (value < extrema.minValue) {
      extrema.minValue = value;
      extrema.minSpot = spot;
    }
    if (value > extrema.maxValue) {
      extrema.maxValue = value;
      extrema.maxSpot = spot;
    }
  }

  return extrema;
}
