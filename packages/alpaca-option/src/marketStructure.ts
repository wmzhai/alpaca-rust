import { clock as timeClock } from '@alpaca/time';

import type {
  MarketStructureAnalysis,
  MarketStructureFilters,
  MarketStructureLevel,
  MarketStructureOptionRecord,
} from './types';

type LevelAccumulator = {
  strike: number;
  call_open_interest: number;
  put_open_interest: number;
  call_gamma_exposure: number;
  put_gamma_exposure: number;
  call_volume: number;
  put_volume: number;
};

export function gammaExposure(
  record: MarketStructureOptionRecord,
  underlyingPrice: number,
): number | null {
  const gamma = finite(record.gamma);
  const openInterest = finite(record.open_interest);
  const multiplier = finite(record.multiplier);
  const spot = finite(underlyingPrice);

  if (
    gamma == null ||
    openInterest == null ||
    multiplier == null ||
    spot == null ||
    openInterest < 0 ||
    multiplier <= 0 ||
    spot <= 0
  ) {
    return null;
  }

  const exposure = gamma * openInterest * multiplier * spot * spot * 0.01;
  return finite(record.option_right === 'put' ? -exposure : exposure);
}

export function filterMarketStructureRecords(
  records: MarketStructureOptionRecord[],
  filters: MarketStructureFilters,
): MarketStructureOptionRecord[] {
  return records.filter(
    (record) =>
      matchesExpiration(record, filters) &&
      matchesOptionRight(record, filters) &&
      matchesStrike(record, filters) &&
      matchesDte(record, filters) &&
      matchesOpenInterest(record, filters),
  );
}

export function analyzeMarketStructure(
  records: MarketStructureOptionRecord[],
): MarketStructureAnalysis {
  const recordsCount = records.length;
  const underlyingPrice =
    records
      .map((record) => finitePositive(record.underlying_price))
      .find((value) => value != null) ?? null;
  const openInterestCount = records.filter(
    (record) => finite(record.open_interest) != null,
  ).length;
  const warnings: string[] = [];

  if (records.length === 0) {
    warnings.push('no_records');
  }
  if (underlyingPrice == null) {
    warnings.push('missing_underlying_price');
  }
  if (openInterestCount < recordsCount) {
    warnings.push('incomplete_open_interest');
  }

  const accumulators: LevelAccumulator[] = [];
  if (underlyingPrice != null) {
    for (const record of records) {
      const strike = finite(record.strike);
      if (strike == null) {
        continue;
      }

      const openInterest = Math.max(finite(record.open_interest) ?? 0, 0);
      const volume = activityVolume(record);
      const exposure = gammaExposure(record, underlyingPrice) ?? 0;
      let level = accumulators.find((candidate) => sameStrike(candidate.strike, strike));
      if (!level) {
        level = {
          strike,
          call_open_interest: 0,
          put_open_interest: 0,
          call_gamma_exposure: 0,
          put_gamma_exposure: 0,
          call_volume: 0,
          put_volume: 0,
        };
        accumulators.push(level);
      }

      if (record.option_right === 'call') {
        level.call_open_interest += openInterest;
        level.call_gamma_exposure += exposure;
        level.call_volume += volume;
      } else {
        level.put_open_interest += openInterest;
        level.put_gamma_exposure += exposure;
        level.put_volume += volume;
      }
    }
  }

  const levels = accumulators
    .sort((left, right) => left.strike - right.strike)
    .map((level) => toLevel(level, []));

  if (levels.length === 0 && recordsCount > 0) {
    warnings.push('no_gamma_exposure_records');
  }

  const callWallStrike =
    maxBy(
      levels.filter((level) => level.call_gamma_exposure > 0),
      (level) => level.call_gamma_exposure,
    )?.strike ?? null;
  const putWallStrike =
    minBy(
      levels.filter((level) => level.put_gamma_exposure < 0),
      (level) => level.put_gamma_exposure,
    )?.strike ?? null;
  const absoluteWallStrike =
    maxBy(
      levels.filter((level) => level.absolute_gamma_exposure > 0),
      (level) => level.absolute_gamma_exposure,
    )?.strike ?? null;

  for (const level of levels) {
    if (callWallStrike != null && sameStrike(level.strike, callWallStrike)) {
      level.labels.push('call_wall');
    }
    if (putWallStrike != null && sameStrike(level.strike, putWallStrike)) {
      level.labels.push('put_wall');
    }
    if (absoluteWallStrike != null && sameStrike(level.strike, absoluteWallStrike)) {
      level.labels.push('absolute_gamma_exposure_wall');
    }
  }

  const netGammaExposure = levels.reduce((sum, level) => sum + level.net_gamma_exposure, 0);
  const absoluteGammaExposure = levels.reduce(
    (sum, level) => sum + level.absolute_gamma_exposure,
    0,
  );

  return {
    underlying_price: underlyingPrice,
    records_count: recordsCount,
    open_interest_coverage: recordsCount === 0 ? 0 : openInterestCount / recordsCount,
    call_wall: levelByStrike(levels, callWallStrike),
    put_wall: levelByStrike(levels, putWallStrike),
    absolute_gamma_exposure_wall: levelByStrike(levels, absoluteWallStrike),
    net_gamma_exposure: netGammaExposure,
    absolute_gamma_exposure: absoluteGammaExposure,
    levels,
    warnings,
  };
}

function toLevel(level: LevelAccumulator, labels: string[]): MarketStructureLevel {
  const totalOpenInterest = level.call_open_interest + level.put_open_interest;
  const netGammaExposure = level.call_gamma_exposure + level.put_gamma_exposure;
  const absoluteGammaExposure =
    Math.abs(level.call_gamma_exposure) + Math.abs(level.put_gamma_exposure);
  const totalVolume = level.call_volume + level.put_volume;

  return {
    strike: level.strike,
    call_open_interest: level.call_open_interest,
    put_open_interest: level.put_open_interest,
    total_open_interest: totalOpenInterest,
    call_gamma_exposure: level.call_gamma_exposure,
    put_gamma_exposure: level.put_gamma_exposure,
    net_gamma_exposure: netGammaExposure,
    absolute_gamma_exposure: absoluteGammaExposure,
    call_volume: level.call_volume,
    put_volume: level.put_volume,
    total_volume: totalVolume,
    labels,
  };
}

function matchesExpiration(
  record: MarketStructureOptionRecord,
  filters: MarketStructureFilters,
): boolean {
  return !filters.expiration_date || record.expiration_date === filters.expiration_date;
}

function matchesOptionRight(
  record: MarketStructureOptionRecord,
  filters: MarketStructureFilters,
): boolean {
  return !filters.option_right || record.option_right === filters.option_right;
}

function matchesStrike(
  record: MarketStructureOptionRecord,
  filters: MarketStructureFilters,
): boolean {
  const strike = finite(record.strike);
  if (strike == null) {
    return false;
  }

  const min = finite(filters.strike_price_gte);
  if (min != null && strike < min) {
    return false;
  }
  const max = finite(filters.strike_price_lte);
  if (max != null && strike > max) {
    return false;
  }
  return true;
}

function matchesDte(
  record: MarketStructureOptionRecord,
  filters: MarketStructureFilters,
): boolean {
  if (filters.dte_min == null && filters.dte_max == null) {
    return true;
  }

  const dte = daysToExpiration(record);
  if (dte == null) {
    return false;
  }
  const min = finite(filters.dte_min);
  if (min != null && dte < min) {
    return false;
  }
  const max = finite(filters.dte_max);
  if (max != null && dte > max) {
    return false;
  }
  return true;
}

function matchesOpenInterest(
  record: MarketStructureOptionRecord,
  filters: MarketStructureFilters,
): boolean {
  if (!filters.require_open_interest) {
    return true;
  }
  const openInterest = finite(record.open_interest);
  return openInterest != null && openInterest > 0;
}

function daysToExpiration(record: MarketStructureOptionRecord): number | null {
  const asOfDate = record.as_of.slice(0, 10) || record.as_of;
  return finite(timeClock.fractionalDaysBetween(asOfDate, record.expiration_date));
}

function activityVolume(record: MarketStructureOptionRecord): number {
  return record.daily_volume ?? record.minute_volume ?? record.latest_trade_size ?? 0;
}

function levelByStrike(
  levels: MarketStructureLevel[],
  strike: number | null,
): MarketStructureLevel | null {
  if (strike == null) {
    return null;
  }
  return levels.find((level) => sameStrike(level.strike, strike)) ?? null;
}

function finite(value: number | null | undefined): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function finitePositive(value: number | null | undefined): number | null {
  const number = finite(value);
  return number != null && number > 0 ? number : null;
}

function sameStrike(left: number, right: number): boolean {
  return Math.abs(left - right) <= 1e-8;
}

function maxBy<T>(values: T[], score: (value: T) => number): T | undefined {
  let best: T | undefined;
  let bestScore = Number.NEGATIVE_INFINITY;
  for (const value of values) {
    const current = score(value);
    if (current > bestScore) {
      best = value;
      bestScore = current;
    }
  }
  return best;
}

function minBy<T>(values: T[], score: (value: T) => number): T | undefined {
  let best: T | undefined;
  let bestScore = Number.POSITIVE_INFINITY;
  for (const value of values) {
    const current = score(value);
    if (current < bestScore) {
      best = value;
      bestScore = current;
    }
  }
  return best;
}
