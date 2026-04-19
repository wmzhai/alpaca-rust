import { marketHoursForDate } from './calendar';
import { minutesFromHhmm, now, parts } from './clock';
import type { HhmmString, MarketSession, TimestampParts } from './types';

function timestampParts(timestamp: string): TimestampParts {
  return parts(timestamp);
}

export function marketSessionAt(timestamp: string): MarketSession {
  const parsed = timestampParts(timestamp);
  const hours = marketHoursForDate(parsed.date);

  if (!hours.is_trading_date) {
    return 'closed';
  }

  const minutes = minutesFromHhmm(parsed.hhmm_string);
  const premarketOpen = minutesFromHhmm(hours.premarket_open!);
  const regularOpen = minutesFromHhmm(hours.regular_open!);
  const regularClose = minutesFromHhmm(hours.regular_close!);
  const afterHoursClose = minutesFromHhmm(hours.after_hours_close!);

  if (minutes >= premarketOpen && minutes < regularOpen) {
    return 'premarket';
  }
  if (minutes >= regularOpen && minutes < regularClose) {
    return 'regular';
  }
  if (minutes >= regularClose && minutes < afterHoursClose) {
    return 'after_hours';
  }
  return 'closed';
}

export function isPremarketAt(timestamp: string): boolean {
  try {
    return marketSessionAt(timestamp) === 'premarket';
  } catch {
    return false;
  }
}

export function isRegularSessionAt(timestamp: string): boolean {
  try {
    return marketSessionAt(timestamp) === 'regular';
  } catch {
    return false;
  }
}

export function isAfterHoursAt(timestamp: string): boolean {
  try {
    return marketSessionAt(timestamp) === 'after_hours';
  } catch {
    return false;
  }
}

export function isInWindow(timestamp: string, start: HhmmString, end: HhmmString): boolean {
  try {
    const current = minutesFromHhmm(timestampParts(timestamp).hhmm_string);
    const startMinutes = minutesFromHhmm(start);
    const endMinutes = minutesFromHhmm(end);

    if (endMinutes < startMinutes) {
      return false;
    }

    return current >= startMinutes && current < endMinutes;
  } catch {
    return false;
  }
}

export function isOvernightWindow(timestamp: string): boolean {
  try {
    const parsed = timestampParts(timestamp);
    return parsed.hour >= 20 || parsed.hour < 4;
  } catch {
    return false;
  }
}

export function isRegularSessionNow(): boolean {
  return isRegularSessionAt(now());
}

export function isOvernightNow(): boolean {
  return isOvernightWindow(now());
}

export function isInWindowNow(start: HhmmString, end: HhmmString): boolean {
  return isInWindow(now(), start, end);
}
