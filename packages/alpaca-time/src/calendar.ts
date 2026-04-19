import { fail } from './error';
import { now, parts, today } from './clock';
import {
  addUtcDays,
  datePartsToUtcDate,
  formatDateParts,
  parseDateParts,
  utcDate,
  utcDateToDateParts,
} from './internal';
import type { MarketHours, NyDateString, NyTimestampString, TradingDayInfo } from './types';

const MONDAY = 1;
const THURSDAY = 4;
const SATURDAY = 6;
const SUNDAY = 0;

const holidayCache = new Map<number, Set<string>>();
const earlyCloseCache = new Map<number, Set<string>>();

function observedFixedHoliday(year: number, month: number, day: number): Date {
  const date = utcDate(year, month, day);
  const weekday = date.getUTCDay();
  if (weekday === SATURDAY) {
    return addUtcDays(date, -1);
  }
  if (weekday === SUNDAY) {
    return addUtcDays(date, 1);
  }
  return date;
}

function nthWeekday(year: number, month: number, weekday: number, nth: number): Date {
  let current = utcDate(year, month, 1);
  while (current.getUTCDay() !== weekday) {
    current = addUtcDays(current, 1);
  }
  return addUtcDays(current, (nth - 1) * 7);
}

function lastWeekday(year: number, month: number, weekday: number): Date {
  const nextMonth = month === 12 ? utcDate(year + 1, 1, 1) : utcDate(year, month + 1, 1);
  let current = addUtcDays(nextMonth, -1);
  while (current.getUTCDay() !== weekday) {
    current = addUtcDays(current, -1);
  }
  return current;
}

function easterDate(year: number): Date {
  const a = year % 19;
  const b = Math.floor(year / 100);
  const c = year % 100;
  const d = Math.floor(b / 4);
  const e = b % 4;
  const f = Math.floor((b + 8) / 25);
  const g = Math.floor((b - f + 1) / 3);
  const h = (19 * a + b - d - g + 15) % 30;
  const i = Math.floor(c / 4);
  const k = c % 4;
  const l = (32 + 2 * e + 2 * i - h - k) % 7;
  const m = Math.floor((a + 11 * h + 22 * l) / 451);
  const month = Math.floor((h + l - 7 * m + 114) / 31);
  const day = ((h + l - 7 * m + 114) % 31) + 1;
  return utcDate(year, month, day);
}

function isWeekday(date: Date): boolean {
  const weekday = date.getUTCDay();
  return weekday !== SATURDAY && weekday !== SUNDAY;
}

function holidayDatesForYear(year: number): Set<string> {
  const cached = holidayCache.get(year);
  if (cached) {
    return cached;
  }

  const dates = new Set<string>([
    formatDateParts(utcDateToDateParts(observedFixedHoliday(year, 1, 1))),
    formatDateParts(utcDateToDateParts(nthWeekday(year, 1, MONDAY, 3))),
    formatDateParts(utcDateToDateParts(nthWeekday(year, 2, MONDAY, 3))),
    formatDateParts(utcDateToDateParts(addUtcDays(easterDate(year), -2))),
    formatDateParts(utcDateToDateParts(lastWeekday(year, 5, MONDAY))),
    formatDateParts(utcDateToDateParts(observedFixedHoliday(year, 6, 19))),
    formatDateParts(utcDateToDateParts(observedFixedHoliday(year, 7, 4))),
    formatDateParts(utcDateToDateParts(nthWeekday(year, 9, MONDAY, 1))),
    formatDateParts(utcDateToDateParts(nthWeekday(year, 11, THURSDAY, 4))),
    formatDateParts(utcDateToDateParts(observedFixedHoliday(year, 12, 25))),
  ]);

  holidayCache.set(year, dates);
  return dates;
}

function isMarketHolidayDate(date: Date): boolean {
  return holidayDatesForYear(date.getUTCFullYear()).has(formatDateParts(utcDateToDateParts(date)));
}

function isTradingDateDate(date: Date): boolean {
  return isWeekday(date) && !isMarketHolidayDate(date);
}

function addTradingDaysFrom(date: Date, days: number): Date {
  let current = new Date(date.getTime());
  const step = days === 0 ? 0 : days > 0 ? 1 : -1;
  let remaining = Math.abs(days);
  if (step === 0) {
    return current;
  }
  while (remaining > 0) {
    current = addUtcDays(current, step);
    if (isTradingDateDate(current)) {
      remaining -= 1;
    }
  }
  return current;
}

function earlyCloseDatesForYear(year: number): Set<string> {
  const cached = earlyCloseCache.get(year);
  if (cached) {
    return cached;
  }

  const dates = new Set<string>();
  const thanksgiving = nthWeekday(year, 11, THURSDAY, 4);
  const blackFriday = addUtcDays(thanksgiving, 1);
  if (isWeekday(blackFriday) && !isMarketHolidayDate(blackFriday)) {
    dates.add(formatDateParts(utcDateToDateParts(blackFriday)));
  }

  const independenceHoliday = observedFixedHoliday(year, 7, 4);
  const independenceEve = addTradingDaysFrom(independenceHoliday, -1);
  if (independenceEve.getUTCFullYear() === year) {
    dates.add(formatDateParts(utcDateToDateParts(independenceEve)));
  }

  const christmasEve = utcDate(year, 12, 24);
  if (isWeekday(christmasEve) && !isMarketHolidayDate(christmasEve)) {
    dates.add(formatDateParts(utcDateToDateParts(christmasEve)));
  }

  earlyCloseCache.set(year, dates);
  return dates;
}

function isEarlyCloseDate(date: Date): boolean {
  return earlyCloseDatesForYear(date.getUTCFullYear()).has(formatDateParts(utcDateToDateParts(date)));
}

function validateDays(days: number): void {
  if (!Number.isInteger(days)) {
    fail('invalid_days', `days must be integer: ${days}`);
  }
}

export function tradingDayInfo(date: string): TradingDayInfo {
  const parsed = datePartsToUtcDate(parseDateParts(date));
  const canonicalDate = formatDateParts(utcDateToDateParts(parsed));
  const is_trading_date = isTradingDateDate(parsed);
  const is_market_holiday = isMarketHolidayDate(parsed);
  const is_early_close = is_trading_date && isEarlyCloseDate(parsed);
  const market_hours = marketHoursForDate(canonicalDate);

  return {
    date: canonicalDate,
    is_trading_date,
    is_market_holiday,
    is_early_close,
    market_hours,
  };
}

export function isTradingDate(date: string): boolean {
  try {
    return isTradingDateDate(datePartsToUtcDate(parseDateParts(date)));
  } catch {
    return false;
  }
}

export function isTradingToday(): boolean {
  return isTradingDate(today());
}

export function marketHoursForDate(date: string): MarketHours {
  const parsed = datePartsToUtcDate(parseDateParts(date));
  const canonicalDate = formatDateParts(utcDateToDateParts(parsed));
  const is_trading_date = isTradingDateDate(parsed);
  const is_early_close = is_trading_date && isEarlyCloseDate(parsed);

  if (!is_trading_date) {
    return {
      date: canonicalDate,
      is_trading_date: false,
      is_early_close: false,
      premarket_open: null,
      regular_open: null,
      regular_close: null,
      after_hours_close: null,
    };
  }

  return {
    date: canonicalDate,
    is_trading_date: true,
    is_early_close,
    premarket_open: '04:00',
    regular_open: '09:30',
    regular_close: is_early_close ? '13:00' : '16:00',
    after_hours_close: '20:00',
  };
}

export function lastCompletedTradingDate(atTimestamp?: NyTimestampString): NyDateString {
  const parsed = parts(atTimestamp ?? now());
  const hours = marketHoursForDate(parsed.date);

  if (!hours.is_trading_date) {
    return addTradingDays(parsed.date, -1);
  }

  if (!hours.regular_close) {
    fail('invalid_market_hours', 'missing regular close');
  }

  const currentMinutes = parsed.hour * 60 + parsed.minute;
  const closeMinutes = Number(hours.regular_close.slice(0, 2)) * 60 + Number(hours.regular_close.slice(3, 5));

  return currentMinutes >= closeMinutes ? parsed.date : addTradingDays(parsed.date, -1);
}

export function addTradingDays(date: string, days: number): NyDateString {
  validateDays(days);
  const parsed = datePartsToUtcDate(parseDateParts(date));
  return formatDateParts(utcDateToDateParts(addTradingDaysFrom(parsed, days)));
}
