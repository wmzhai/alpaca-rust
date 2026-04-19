import { clock as timeClock, range as timeRange } from '@alpaca/time';

import { fail } from './error';

export function nearestWeeklyExpiration(anchorDate: string): string {
  const anchor = timeClock.parseDate(anchorDate);
  const candidates = timeRange.weeklyLastTradingDates(anchor, timeRange.addDays(anchor, 14));
  if (candidates.length === 0) {
    fail('missing_weekly_expiration', `no weekly expiration found on or after ${anchorDate}`);
  }
  return candidates[0];
}

export function weeklyExpirationsBetween(startDateInput: string, endDateInput: string): string[] {
  const startDate = timeClock.parseDate(startDateInput);
  const endDate = timeClock.parseDate(endDateInput);
  return timeRange.weeklyLastTradingDates(startDate, endDate);
}

export function standardMonthlyExpiration(year: number, month: number): string {
  return timeRange.lastTradingDateOfWeek(timeRange.nthWeekday(year, month, 'fri', 3));
}
