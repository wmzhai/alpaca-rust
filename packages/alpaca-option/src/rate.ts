export type RiskFreeRatePoint = {
  years: number;
  rate: number;
};

export const DEFAULT_RISK_FREE_RATE = 0.0368;

export const DEFAULT_RISK_FREE_RATE_CURVE: readonly RiskFreeRatePoint[] = Object.freeze([
  { years: 1 / 12, rate: 0.0370 },
  { years: 1.5 / 12, rate: 0.0370 },
  { years: 2 / 12, rate: 0.0371 },
  { years: 3 / 12, rate: 0.0380 },
  { years: 4 / 12, rate: 0.0379 },
  { years: 6 / 12, rate: 0.0383 },
  { years: 1, rate: 0.0385 },
  { years: 2, rate: 0.0415 },
  { years: 3, rate: 0.0421 },
  { years: 5, rate: 0.0429 },
  { years: 7, rate: 0.0442 },
  { years: 10, rate: 0.0456 },
  { years: 20, rate: 0.0505 },
  { years: 30, rate: 0.0503 },
]);

export function riskFreeRateForYears(years: number): number {
  if (!Number.isFinite(years)) {
    return DEFAULT_RISK_FREE_RATE;
  }

  const first = DEFAULT_RISK_FREE_RATE_CURVE[0];
  if (years <= first.years) {
    return first.rate;
  }

  for (let index = 1; index < DEFAULT_RISK_FREE_RATE_CURVE.length; index += 1) {
    const left = DEFAULT_RISK_FREE_RATE_CURVE[index - 1]!;
    const right = DEFAULT_RISK_FREE_RATE_CURVE[index]!;
    if (years <= right.years) {
      const weight = (years - left.years) / (right.years - left.years);
      return left.rate + (right.rate - left.rate) * weight;
    }
  }

  return DEFAULT_RISK_FREE_RATE_CURVE[DEFAULT_RISK_FREE_RATE_CURVE.length - 1]!.rate;
}
