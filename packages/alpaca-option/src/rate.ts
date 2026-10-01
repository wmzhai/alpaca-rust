export type RiskFreeRatePoint = {
  years: number;
  rate: number;
};

export const DEFAULT_RISK_FREE_RATE = 0.0368;

// Treasury par yield curve for 2026-09-30, the latest complete official curve.
// Live pricing replaces this after Universe publishes a newer row.
export const DEFAULT_RISK_FREE_RATE_CURVE: readonly RiskFreeRatePoint[] = Object.freeze([
  { years: 1 / 12, rate: 0.0402 },
  { years: 1.5 / 12, rate: 0.0413 },
  { years: 2 / 12, rate: 0.0416 },
  { years: 3 / 12, rate: 0.0420 },
  { years: 4 / 12, rate: 0.0429 },
  { years: 6 / 12, rate: 0.0433 },
  { years: 1, rate: 0.0454 },
  { years: 2, rate: 0.0488 },
  { years: 3, rate: 0.0500 },
  { years: 5, rate: 0.0509 },
  { years: 7, rate: 0.0519 },
  { years: 10, rate: 0.0529 },
  { years: 20, rate: 0.0568 },
  { years: 30, rate: 0.0564 },
]);

let installedCurve: readonly RiskFreeRatePoint[] | null = null;

export function installRiskFreeRateCurve(points: readonly RiskFreeRatePoint[]): void {
  if (points.length < 2) {
    throw new Error('risk-free curve must contain at least two points');
  }

  let previousYears = Number.NEGATIVE_INFINITY;
  const curve = points.map((point) => {
    if (!Number.isFinite(point.years) || point.years <= previousYears) {
      throw new Error(`curve years must be finite and strictly increasing: ${point.years}`);
    }
    if (!Number.isFinite(point.rate) || point.rate <= 0 || point.rate >= 1) {
      throw new Error(`curve rate must be in (0, 1): ${point.rate}`);
    }
    previousYears = point.years;
    return { years: point.years, rate: point.rate };
  });
  installedCurve = Object.freeze(curve);
}

export function clearRiskFreeRateCurve(): void {
  installedCurve = null;
}

export function activeRiskFreeRateCurve(): readonly RiskFreeRatePoint[] {
  return installedCurve ?? DEFAULT_RISK_FREE_RATE_CURVE;
}

export function riskFreeRateForYears(years: number): number {
  return rateOnCurve(activeRiskFreeRateCurve(), years);
}

export function rateOnCurve(curve: readonly RiskFreeRatePoint[], years: number): number {
  if (!Number.isFinite(years) || curve.length === 0) {
    return DEFAULT_RISK_FREE_RATE;
  }

  const first = curve[0]!;
  if (years <= first.years) {
    return first.rate;
  }

  for (let index = 1; index < curve.length; index += 1) {
    const left = curve[index - 1]!;
    const right = curve[index]!;
    if (years <= right.years) {
      const span = right.years - left.years;
      if (span <= 0) {
        return right.rate;
      }
      const weight = (years - left.years) / span;
      return left.rate + (right.rate - left.rate) * weight;
    }
  }

  return curve[curve.length - 1]!.rate;
}
