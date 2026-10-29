import { range } from '@alpaca/time';

export const IV_LOOKBACK_DAYS = 365;
export const YEAR_DAYS = 365;

export type IvWindowMetrics = {
  current_iv: number;
  iv_percentile: number;
  iv_rank: number;
  iv_max: number;
  iv_min: number;
  sample_size: number;
  as_of_date: string;
};

export type TermIvQuality = 'observed' | 'interpolated' | 'extrapolated' | 'unavailable';

export type TermIvValue = {
  iv: number | null;
  quality: TermIvQuality;
};

export function ivWindowMetrics(
  points: ReadonlyArray<readonly [string, number]>,
  windowEnd: string,
  lookbackDays: number,
): IvWindowMetrics | null {
  if (lookbackDays < 0 || windowEnd.length !== 10) return null;
  let cutoff: string;
  try {
    cutoff = range.addDays(windowEnd, -lookbackDays);
  } catch {
    return null;
  }
  const filtered = points
    .filter(([date]) => date.length === 10 && date >= cutoff && date <= windowEnd)
    .slice()
    .sort((left, right) => (left[0] < right[0] ? -1 : left[0] > right[0] ? 1 : 0));
  if (filtered.length === 0) return null;
  const ivs = filtered.map((point) => point[1]);
  const current = ivs[ivs.length - 1];
  const ivMax = Math.max(0, ...ivs);
  const ivMin = Math.max(0, Math.min(...ivs));
  const below = ivs.filter((iv) => iv < current).length;
  return {
    current_iv: current,
    iv_percentile: below / ivs.length,
    iv_rank: ivMax > ivMin ? (current - ivMin) / (ivMax - ivMin) : 0.5,
    iv_max: ivMax,
    iv_min: ivMin,
    sample_size: ivs.length,
    as_of_date: filtered[filtered.length - 1][0],
  };
}

export function empiricalPercentile(samples: readonly number[], reference: number): number | null {
  if (samples.length === 0 || !Number.isFinite(reference)) return null;
  if (samples.some((value) => !Number.isFinite(value))) return null;
  const below = samples.filter((value) => value < reference).length;
  return below / samples.length;
}

export function roundIv(value: number): number {
  return Math.round(value * 1_000_000) / 1_000_000;
}

export function interpolateTermIv(points: ReadonlyArray<readonly [number, number]>, days: number): TermIvValue {
  if (points.length === 0) return { iv: null, quality: 'unavailable' };
  const nodes = points
    .slice()
    .sort((left, right) => left[0] - right[0])
    .filter((point, index, all) => index === 0 || point[0] !== all[index - 1][0]);
  const exact = nodes.find((point) => point[0] === days);
  if (exact != null) return { iv: roundIv(exact[1]), quality: 'observed' };
  if (days < 7) return { iv: null, quality: 'unavailable' };
  const first = nodes[0];
  const last = nodes[nodes.length - 1];
  if (days < first[0]) return { iv: roundIv(first[1]), quality: 'extrapolated' };
  if (days > last[0]) {
    if (nodes.length >= 2) {
      const previous = nodes[nodes.length - 2];
      const slope = (last[1] - previous[1]) / (last[0] - previous[0]);
      return { iv: roundIv(Math.max(0, last[1] + slope * (days - last[0]))), quality: 'extrapolated' };
    }
    return { iv: roundIv(last[1]), quality: 'extrapolated' };
  }
  for (let index = 0; index < nodes.length - 1; index += 1) {
    const left = nodes[index];
    const right = nodes[index + 1];
    if (days > left[0] && days < right[0]) {
      const ratio = (days - left[0]) / (right[0] - left[0]);
      return { iv: roundIv(left[1] + ratio * (right[1] - left[1])), quality: 'interpolated' };
    }
  }
  return { iv: null, quality: 'unavailable' };
}

export function termRatio(sigmaShort: number, sigmaLong: number): number | null {
  if (Number.isFinite(sigmaShort) && Number.isFinite(sigmaLong) && sigmaShort > 0 && sigmaLong > 0) {
    return sigmaShort / sigmaLong;
  }
  return null;
}

export function expectedMove(spot: number, sigma: number, horizonDays: number): number | null {
  if (Number.isFinite(spot) && Number.isFinite(sigma) && Number.isFinite(horizonDays) && spot > 0 && sigma > 0 && horizonDays > 0) {
    return spot * sigma * Math.sqrt(horizonDays / YEAR_DAYS);
  }
  return null;
}

export function sigma30MoveScale(sigma30: number): number | null {
  if (Number.isFinite(sigma30) && sigma30 > 0) return sigma30 * Math.sqrt(30 / YEAR_DAYS);
  return null;
}

export function ivMomentum(sigma: number, ema: number): number | null {
  if (Number.isFinite(sigma) && Number.isFinite(ema) && ema > 0) return (sigma - ema) / ema;
  return null;
}

export function priceDistanceAtr(spot: number, ema: number, atr: number): number | null {
  if (Number.isFinite(spot) && Number.isFinite(ema) && Number.isFinite(atr) && atr > 0) return (spot - ema) / atr;
  return null;
}

export function drawdownFromHigh(high: number, spot: number): number | null {
  if (Number.isFinite(high) && Number.isFinite(spot) && high > 0) return (high - spot) / high;
  return null;
}

export function emaLast(values: number[], period: number): number | null {
  if (!(period > 0) || values.length < period || values.some((value) => !Number.isFinite(value))) return null;
  let ema = values.slice(0, period).reduce((sum, value) => sum + value, 0) / period;
  const k = 2 / (period + 1);
  for (const value of values.slice(period)) ema = value * k + ema * (1 - k);
  return Number.isFinite(ema) ? ema : null;
}

export function wilderAtr(highs: number[], lows: number[], closes: number[], period: number): number | null {
  if (!(period > 0) || highs.length !== lows.length || highs.length !== closes.length || highs.length < period + 1) return null;
  const ranges: number[] = [];
  for (let index = 1; index < highs.length; index += 1) {
    const values = [highs[index], lows[index], closes[index], closes[index - 1]];
    if (values.some((value) => !Number.isFinite(value))) return null;
    ranges.push(Math.max(highs[index] - lows[index], Math.abs(highs[index] - closes[index - 1]), Math.abs(lows[index] - closes[index - 1])));
  }
  if (ranges.length < period) return null;
  let atr = ranges.slice(0, period).reduce((sum, value) => sum + value, 0) / period;
  for (const range of ranges.slice(period)) atr = (atr * (period - 1) + range) / period;
  return Number.isFinite(atr) && atr > 0 ? atr : null;
}

export type TierName = 'tier1' | 'tier2' | 'tier3' | 'low' | 'unknown';

export function instantTier(delta: number | null): TierName {
  if (delta == null || !Number.isFinite(delta) || delta > 1) return 'unknown';
  if (delta <= 0.05) return 'low';
  if (delta < 0.4) return 'tier3';
  if (delta < 0.62) return 'tier2';
  return 'tier1';
}

export type StableTier = {
  label: TierName;
  pending: TierName | null;
  confirms: number;
};

export function advanceStableTier(
  state: StableTier | null,
  instant: TierName,
  hysteresis: number,
  delta: number | null,
  confirmSamples: number,
): StableTier {
  const required = Math.max(1, confirmSamples);
  if (instant === 'unknown' || delta == null || !Number.isFinite(delta)) {
    return { label: state?.label && state.label !== 'unknown' ? state.label : 'unknown', pending: null, confirms: 0 };
  }
  if (state == null || state.label === 'unknown') {
    return { label: instant, pending: null, confirms: 0 };
  }
  if (instant === state.label) {
    return { label: state.label, pending: null, confirms: 0 };
  }
  if (!crossedHysteresis(state.label, instant, delta, hysteresis)) {
    return { label: state.label, pending: null, confirms: 0 };
  }
  const confirms = state.pending === instant ? state.confirms + 1 : 1;
  if (confirms >= required) {
    return { label: instant, pending: null, confirms: 0 };
  }
  return { label: state.label, pending: instant, confirms };
}

function crossedHysteresis(from: TierName, to: TierName, delta: number, hysteresis: number): boolean {
  const band = Math.max(0, hysteresis);
  if (from === 'tier2' && to === 'tier1') return delta >= 0.62 + band;
  if (from === 'tier2' && to === 'tier3') return delta < 0.4 - band;
  if (from === 'tier1' && to === 'tier2') return delta < 0.62 - band;
  if (from === 'tier3' && to === 'tier2') return delta >= 0.4 + band;
  if (from === 'tier3' && to === 'low') return delta <= 0.05 - band;
  if (from === 'low' && to === 'tier3') return delta > 0.05 + band;
  if (from === 'tier1' && to === 'tier3') return delta < 0.4 - band;
  if (from === 'tier3' && to === 'tier1') return delta >= 0.62 + band;
  return true;
}

export function pairDeltaShares(longQty: number, longDelta: number, shortQty: number, shortDelta: number): number | null {
  if (![longQty, longDelta, shortQty, shortDelta].every((value) => Number.isFinite(value))) return null;
  if (longQty < 0 || shortQty < 0) return null;
  return 100 * (longQty * longDelta - shortQty * shortDelta);
}

export type LegEfficiency = {
  omega: number | null;
  gammaElasticity: number | null;
  thetaBurdenDaily: number | null;
  thetaCostPerDelta: number | null;
  vegaBurdenPerPoint: number | null;
  extrinsic: number | null;
  extrinsicRatio: number | null;
  reason: string | null;
};

export function legEfficiency(input: {
  spot: number;
  strike: number;
  premium: number;
  delta: number;
  gamma: number;
  thetaDaily: number;
  vegaPerPoint: number;
}): LegEfficiency {
  const { spot, strike, premium, delta, gamma, thetaDaily, vegaPerPoint } = input;
  const invalid = ![spot, strike, premium, delta, gamma, thetaDaily, vegaPerPoint].every((value) => Number.isFinite(value))
    || !(premium > 0) || !(spot > 0) || !(delta > 0);
  if (invalid) {
    return {
      omega: null,
      gammaElasticity: null,
      thetaBurdenDaily: null,
      thetaCostPerDelta: null,
      vegaBurdenPerPoint: null,
      extrinsic: null,
      extrinsicRatio: null,
      reason: '缺少有效报价或 Delta',
    };
  }
  const extrinsic = premium - Math.max(spot - strike, 0);
  return {
    omega: (spot * delta) / premium,
    gammaElasticity: (spot * spot * gamma) / premium,
    thetaBurdenDaily: Math.max(0, -thetaDaily) / premium,
    thetaCostPerDelta: Math.max(0, -thetaDaily) / delta,
    vegaBurdenPerPoint: Math.abs(vegaPerPoint) / premium,
    extrinsic,
    extrinsicRatio: extrinsic / premium,
    reason: extrinsic < 0 ? '外在价值为负' : null,
  };
}
