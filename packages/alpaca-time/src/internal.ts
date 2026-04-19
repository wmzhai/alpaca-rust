import { fail } from './error';
import type { DayCountBasis, DurationParts, DurationSign, WeekdayCode } from './types';

export interface DateParts {
  year: number;
  month: number;
  day: number;
}

export interface TimestampParts extends DateParts {
  hour: number;
  minute: number;
  second: number;
}

const DATE_RE = /^(\d{4})-(\d{2})-(\d{2})$/;
const TIMESTAMP_RE = /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})$/;
const HHMM_RE = /^(\d{2}):(\d{2})$/;
const RFC3339_RE =
  /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2}))$/;

const NY_FORMATTER = new Intl.DateTimeFormat('en-CA', {
  timeZone: 'America/New_York',
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hourCycle: 'h23',
});

export function pad2(value: number): string {
  return String(value).padStart(2, '0');
}

export function pad4(value: number): string {
  return String(value).padStart(4, '0');
}

export function utcDate(
  year: number,
  month: number,
  day: number,
  hour = 0,
  minute = 0,
  second = 0,
): Date {
  return new Date(Date.UTC(year, month - 1, day, hour, minute, second));
}

function assertValidDate(year: number, month: number, day: number, input: string): void {
  const date = utcDate(year, month, day);
  if (
    date.getUTCFullYear() !== year ||
    date.getUTCMonth() + 1 !== month ||
    date.getUTCDate() !== day
  ) {
    fail('invalid_date', `invalid date: ${input}`);
  }
}

export function parseDateParts(input: string): DateParts {
  const match = DATE_RE.exec(input);
  if (!match) {
    fail('invalid_date', `invalid date: ${input}`);
  }

  const [, yearText, monthText, dayText] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);

  assertValidDate(year, month, day, input);
  return { year, month, day };
}

export function parseTimestampParts(input: string): TimestampParts {
  if (RFC3339_RE.test(input)) {
    return parseTimestampParts(fromUtcRfc3339String(input));
  }

  const match = TIMESTAMP_RE.exec(input);
  if (!match) {
    fail('invalid_timestamp', `invalid timestamp: ${input}`);
  }

  const [, yearText, monthText, dayText, hourText, minuteText, secondText] = match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const hour = Number(hourText);
  const minute = Number(minuteText);
  const second = Number(secondText);

  assertValidDate(year, month, day, input);
  if (hour > 23 || minute > 59 || second > 59) {
    fail('invalid_timestamp', `invalid timestamp: ${input}`);
  }

  return { year, month, day, hour, minute, second };
}

export function parseHhmmParts(input: string): { hour: number; minute: number } {
  const match = HHMM_RE.exec(input);
  if (!match) {
    fail('invalid_hhmm', `invalid hhmm: ${input}`);
  }

  const [, hourText, minuteText] = match;
  const hour = Number(hourText);
  const minute = Number(minuteText);

  if (hour > 23 || minute > 59) {
    fail('invalid_hhmm', `invalid hhmm: ${input}`);
  }

  return { hour, minute };
}

export function formatDateParts(parts: DateParts): string {
  return `${pad4(parts.year)}-${pad2(parts.month)}-${pad2(parts.day)}`;
}

export function formatTimestampParts(parts: TimestampParts): string {
  return `${formatDateParts(parts)} ${pad2(parts.hour)}:${pad2(parts.minute)}:${pad2(parts.second)}`;
}

export function datePartsToUtcDate(parts: DateParts): Date {
  return utcDate(parts.year, parts.month, parts.day);
}

export function timestampPartsToUtcLikeDate(parts: TimestampParts): Date {
  return utcDate(parts.year, parts.month, parts.day, parts.hour, parts.minute, parts.second);
}

export function utcDateToDateParts(date: Date): DateParts {
  return {
    year: date.getUTCFullYear(),
    month: date.getUTCMonth() + 1,
    day: date.getUTCDate(),
  };
}

export function addUtcDays(date: Date, days: number): Date {
  const next = new Date(date.getTime());
  next.setUTCDate(next.getUTCDate() + days);
  return next;
}

export function weekdayCodeFromDate(date: Date): WeekdayCode {
  switch (date.getUTCDay()) {
    case 0:
      return 'sun';
    case 1:
      return 'mon';
    case 2:
      return 'tue';
    case 3:
      return 'wed';
    case 4:
      return 'thu';
    case 5:
      return 'fri';
    default:
      return 'sat';
  }
}

export function dayCountDenominator(basis?: string): number {
  switch (basis) {
    case 'ACT/365':
      return 365;
    case 'ACT/360':
      return 360;
    default:
      return 365.25;
  }
}

export function durationPartsFromSeconds(totalSecondsInput: number): DurationParts {
  const sign: DurationSign =
    totalSecondsInput > 0 ? 1 : totalSecondsInput < 0 ? -1 : 0;
  const total_seconds = Math.abs(Math.trunc(totalSecondsInput));

  return {
    sign,
    total_seconds,
    days: Math.floor(total_seconds / 86_400),
    hours: Math.floor((total_seconds % 86_400) / 3_600),
    minutes: Math.floor((total_seconds % 3_600) / 60),
    seconds: total_seconds % 60,
  };
}

function nyPartsFromUtcDate(date: Date): TimestampParts {
  const partMap = new Map<string, string>();
  for (const part of NY_FORMATTER.formatToParts(date)) {
    if (part.type !== 'literal') {
      partMap.set(part.type, part.value);
    }
  }

  return {
    year: Number(partMap.get('year')),
    month: Number(partMap.get('month')),
    day: Number(partMap.get('day')),
    hour: Number(partMap.get('hour')),
    minute: Number(partMap.get('minute')),
    second: Number(partMap.get('second')),
  };
}

function sameTimestampParts(left: TimestampParts, right: TimestampParts): boolean {
  return (
    left.year === right.year &&
    left.month === right.month &&
    left.day === right.day &&
    left.hour === right.hour &&
    left.minute === right.minute &&
    left.second === right.second
  );
}

function nyOffsetMinutesAt(date: Date): number {
  const parts = nyPartsFromUtcDate(date);
  const localMillis = Date.UTC(
    parts.year,
    parts.month - 1,
    parts.day,
    parts.hour,
    parts.minute,
    parts.second,
  );
  const utcMillis = Math.trunc(date.getTime() / 1000) * 1000;
  return (localMillis - utcMillis) / 60_000;
}

export function fromUtcRfc3339String(input: string): string {
  if (!RFC3339_RE.test(input)) {
    fail('invalid_rfc3339', `invalid rfc3339: ${input}`);
  }

  const date = new Date(input);
  if (Number.isNaN(date.getTime())) {
    fail('invalid_rfc3339', `invalid rfc3339: ${input}`);
  }

  return formatTimestampParts(nyPartsFromUtcDate(date));
}

export function toUtcRfc3339String(input: string): string {
  const parts = parseTimestampParts(input);
  const localMillis = Date.UTC(
    parts.year,
    parts.month - 1,
    parts.day,
    parts.hour,
    parts.minute,
    parts.second,
  );

  let current = new Date(localMillis);
  let previousMillis: number | null = null;

  for (let index = 0; index < 6; index += 1) {
    const offsetMinutes = nyOffsetMinutesAt(current);
    const next = new Date(localMillis - offsetMinutes * 60_000);

    if (next.getTime() === current.getTime() || next.getTime() === previousMillis) {
      current = next;
      break;
    }

    previousMillis = current.getTime();
    current = next;
  }

  if (!sameTimestampParts(nyPartsFromUtcDate(current), parts)) {
    fail('invalid_ny_local_time', `cannot localize NY time: ${input}`);
  }

  return current.toISOString().replace(/\.000Z$/, 'Z');
}

export function localTimestampDiffSeconds(start: string, end: string): number {
  const startDate = timestampPartsToUtcLikeDate(parseTimestampParts(start));
  const endDate = timestampPartsToUtcLikeDate(parseTimestampParts(end));
  return Math.trunc((endDate.getTime() - startDate.getTime()) / 1000);
}

export function dateDiffDays(startDate: string, endDate: string): number {
  const start = datePartsToUtcDate(parseDateParts(startDate));
  const end = datePartsToUtcDate(parseDateParts(endDate));
  return Math.trunc((end.getTime() - start.getTime()) / 86_400_000);
}

export type { DayCountBasis };
