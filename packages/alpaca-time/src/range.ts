import { addTradingDays, isTradingDate } from './calendar';
import { parseDate } from './clock';
import { fail } from './error';
import {
  addUtcDays,
  datePartsToUtcDate,
  formatDateParts,
  parseDateParts,
  utcDateToDateParts,
} from './internal';
import type { DateRange, NyDateString, WeekdayCode } from './types';

function toUtcDate(date: NyDateString): Date {
  return datePartsToUtcDate(parseDateParts(date));
}

function toDateString(date: Date): NyDateString {
  return formatDateParts(utcDateToDateParts(date));
}

function mondayStart(date: Date): Date {
  const daysSinceMonday = (date.getUTCDay() + 6) % 7;
  return addUtcDays(date, -daysSinceMonday);
}

function isoWeekInfo(date: Date): { year: number; week: number } {
  const thursday = addUtcDays(date, 3 - ((date.getUTCDay() + 6) % 7));
  const isoYear = thursday.getUTCFullYear();
  const jan4 = new Date(Date.UTC(isoYear, 0, 4));
  const week1Monday = mondayStart(jan4);
  const diffDays = Math.trunc((mondayStart(date).getTime() - week1Monday.getTime()) / 86_400_000);
  return { year: isoYear, week: Math.floor(diffDays / 7) + 1 };
}

function weekdayNumber(weekday: WeekdayCode): number {
  switch (weekday) {
    case 'mon':
      return 1;
    case 'tue':
      return 2;
    case 'wed':
      return 3;
    case 'thu':
      return 4;
    case 'fri':
      return 5;
    case 'sat':
      return 6;
    case 'sun':
      return 0;
    default:
      fail('invalid_weekday', `invalid weekday: ${weekday}`);
  }
}

export function addDays(date: NyDateString, days: number): NyDateString {
  if (!Number.isInteger(days)) {
    fail('invalid_days', `days must be integer: ${days}`);
  }

  return toDateString(addUtcDays(toUtcDate(parseDate(date)), days));
}

export function dates(startDate: NyDateString, endDate: NyDateString): NyDateString[] {
  const start = toUtcDate(parseDate(startDate));
  const end = toUtcDate(parseDate(endDate));

  if (start.getTime() > end.getTime()) {
    fail('invalid_date_range', `start_date must be <= end_date: ${startDate} > ${endDate}`);
  }

  const values: NyDateString[] = [];
  for (let current = start; current.getTime() <= end.getTime(); current = addUtcDays(current, 1)) {
    values.push(toDateString(current));
  }
  return values;
}

export function tradingDates(startDate: NyDateString, endDate: NyDateString): NyDateString[] {
  return dates(startDate, endDate).filter((date) => isTradingDate(date));
}

export function nthWeekday(
  year: number,
  month: number,
  weekday: WeekdayCode,
  nth: number,
): NyDateString {
  if (!Number.isInteger(year) || !Number.isInteger(month) || month < 1 || month > 12 || !Number.isInteger(nth) || nth < 1) {
    fail('invalid_weekday_selection', `invalid nth weekday input: year=${year}, month=${month}, nth=${nth}`);
  }

  let current = new Date(Date.UTC(year, month - 1, 1));
  const targetWeekday = weekdayNumber(weekday);
  while (current.getUTCDay() !== targetWeekday) {
    current = addUtcDays(current, 1);
  }
  current = addUtcDays(current, (nth - 1) * 7);
  if (current.getUTCMonth() !== month - 1) {
    fail('invalid_weekday_selection', `weekday ${weekday} #${nth} does not exist in ${year}-${month}`);
  }
  return toDateString(current);
}

export function isLastTradingDateOfWeek(date: string): boolean {
  try {
    const canonicalDate = parseDate(date);
    if (!isTradingDate(canonicalDate)) {
      return false;
    }

    const current = toUtcDate(canonicalDate);
    const next = toUtcDate(addTradingDays(canonicalDate, 1));
    return mondayStart(current).getTime() !== mondayStart(next).getTime();
  } catch {
    return false;
  }
}

export function weeklyLastTradingDates(startDate: string, endDate: string): NyDateString[] {
  return tradingDates(startDate, endDate).filter((date) => isLastTradingDateOfWeek(date));
}

export function lastTradingDateOfWeek(date: NyDateString): NyDateString {
  const week = calendarWeekRange(date);
  const values = weeklyLastTradingDates(week.start_date, week.end_date);
  if (values.length === 0) {
    fail('missing_last_trading_date_of_week', `no trading day found for week containing ${date}`);
  }
  return values[0];
}

export function calendarWeekRange(date: string): DateRange {
  const current = toUtcDate(parseDate(date));
  const start = mondayStart(current);
  const end = addUtcDays(start, 6);
  return {
    start_date: toDateString(start),
    end_date: toDateString(end),
  };
}

export function isoWeekRange(year: number, week: number): DateRange {
  if (!Number.isInteger(year) || !Number.isInteger(week) || week < 1 || week > 53) {
    fail('invalid_iso_week', `invalid iso week: year=${year}, week=${week}`);
  }

  const jan4 = new Date(Date.UTC(year, 0, 4));
  const week1Monday = mondayStart(jan4);
  const start = addUtcDays(week1Monday, (week - 1) * 7);
  const info = isoWeekInfo(start);
  if (info.year !== year || info.week !== week) {
    fail('invalid_iso_week', `invalid iso week: year=${year}, week=${week}`);
  }

  return {
    start_date: toDateString(start),
    end_date: toDateString(addUtcDays(start, 6)),
  };
}
