import { dateDiffDays, dayCountDenominator, localTimestampDiffSeconds } from './internal';
import { isTradingDate, marketHoursForDate } from './calendar';
import { now, parseDate, parseTimestamp } from './clock';
import { fail } from './error';
import type { DayCountBasis, NyDateString, NyTimestampString } from './types';

export function close(expirationDate: string): NyTimestampString {
  const canonicalDate = parseDate(expirationDate);
  if (!isTradingDate(canonicalDate)) {
    fail('invalid_expiration_date', `expiration date must be a trading date: ${expirationDate}`);
  }

  const hours = marketHoursForDate(canonicalDate);
  if (!hours.regular_close) {
    fail('invalid_market_hours', 'missing regular close');
  }

  return `${canonicalDate} ${hours.regular_close}:00`;
}

export function calendarDays(expirationDate: string, at: NyTimestampString = now()): number {
  const canonicalTimestamp = parseTimestamp(at);
  const canonicalDate = parseDate(expirationDate);
  const dayDiff = dateDiffDays(canonicalTimestamp.slice(0, 10), canonicalDate);
  if (dayDiff !== 0) {
    return dayDiff;
  }

  return canonicalTimestamp <= close(canonicalDate) ? 0 : -1;
}

export function days(expirationDate: string, at: NyTimestampString = now()): number {
  return localTimestampDiffSeconds(parseTimestamp(at), close(expirationDate)) / 86_400;
}

export function years(
  expirationDate: string,
  at: NyTimestampString = now(),
  basis?: DayCountBasis,
): number {
  try {
    return Math.max(days(expirationDate, at) / dayCountDenominator(basis), 0);
  } catch {
    return 0;
  }
}

export function yearsBetweenDates(
  startDate: NyDateString,
  endDate: NyDateString,
  basis?: DayCountBasis,
): number {
  return dateDiffDays(startDate, endDate) / dayCountDenominator(basis ?? 'ACT/365');
}
