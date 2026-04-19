import {
  formatTimestampParts,
  fromUtcRfc3339String,
  localTimestampDiffSeconds,
  pad2,
  parseDateParts,
  parseHhmmParts,
  parseTimestampParts,
  toUtcRfc3339String,
} from './internal';
import { fail } from './error';
import type {
  ComparableTimeInput,
  HhmmString,
  NyDateString,
  NyTimestampString,
  TimestampParts,
} from './types';

export function now(): NyTimestampString {
  return fromUtcRfc3339String(new Date().toISOString());
}

export function today(): NyDateString {
  return now().slice(0, 10);
}

export function parseDate(input: string): NyDateString {
  const parts = parseDateParts(input);
  return `${parts.year.toString().padStart(4, '0')}-${pad2(parts.month)}-${pad2(parts.day)}`;
}

export function parseTimestamp(input: string): NyTimestampString {
  return formatTimestampParts(parseTimestampParts(input));
}

export function parts(input = now()): TimestampParts {
  const canonical = parseTimestamp(input);
  const parsed = parseTimestampParts(canonical);
  const date = canonical.slice(0, 10);
  return {
    date,
    timestamp: canonical,
    year: parsed.year,
    month: parsed.month,
    day: parsed.day,
    hour: parsed.hour,
    minute: parsed.minute,
    second: parsed.second,
    hhmm: parsed.hour * 100 + parsed.minute,
    hhmm_string: `${pad2(parsed.hour)}:${pad2(parsed.minute)}`,
    weekday_from_sunday: new Date(`${date}T00:00:00Z`).getUTCDay(),
  };
}

export function parseDateOrTimestamp(input: string): ComparableTimeInput {
  try {
    return parseDate(input);
  } catch {
    return parseTimestamp(input);
  }
}

export function firstDateOrTimestamp(
  inputs: Array<string | null | undefined>,
): ComparableTimeInput | null {
  for (const input of inputs) {
    if (typeof input !== 'string') {
      continue;
    }

    const trimmed = input.trim();
    if (!trimmed) {
      continue;
    }

    try {
      return parseDateOrTimestamp(trimmed);
    } catch {
      continue;
    }
  }

  return null;
}

export function toUtcRfc3339(input: string): string {
  return toUtcRfc3339String(input);
}

export function fromUnixSeconds(seconds: number): NyTimestampString {
  if (seconds === 0) {
    return '';
  }
  if (!Number.isInteger(seconds)) {
    return '';
  }

  const date = new Date(seconds * 1000);
  return Number.isNaN(date.getTime()) ? '' : fromUtcRfc3339String(date.toISOString());
}

export function truncateToMinute(input: string): string {
  try {
    const parts = parseTimestampParts(input);
    return formatTimestampParts({ ...parts, second: 0 });
  } catch {
    return input.length >= 16 ? `${input.slice(0, 16)}:00` : input;
  }
}

export function minuteKey(input: string): string {
  const parts = parseTimestampParts(input);
  return `${parseDate(input.slice(0, 10))} ${pad2(parts.hour)}:${pad2(parts.minute)}`;
}

export function hhmmStringFromParts(hour: number, minute: number): HhmmString {
  if (!Number.isInteger(hour) || !Number.isInteger(minute) || hour < 0 || hour > 23 || minute < 0 || minute > 59) {
    fail('invalid_hhmm_parts', `invalid time parts: ${hour}:${minute}`);
  }

  return `${pad2(hour)}:${pad2(minute)}`;
}

export function minutesFromHhmm(input: string): number {
  const { hour, minute } = parseHhmmParts(input);
  return hour * 60 + minute;
}

function parsedDateOrTimestampParts(input: string): {
  date: NyDateString;
  timestamp: NyTimestampString | null;
} {
  const timestamp = (() => {
    try {
      return parseTimestamp(input);
    } catch {
      return null;
    }
  })();

  if (timestamp) {
    return { date: timestamp.slice(0, 10), timestamp };
  }

  return { date: parseDate(input), timestamp: null };
}

export function compareDateOrTimestamp(left: string, right: string): number {
  const leftRaw = left.trim();
  const rightRaw = right.trim();

  if (!leftRaw || !rightRaw) {
    return leftRaw.localeCompare(rightRaw);
  }

  try {
    const leftParts = parsedDateOrTimestampParts(leftRaw);
    const rightParts = parsedDateOrTimestampParts(rightRaw);
    const dateCompare = leftParts.date.localeCompare(rightParts.date);
    if (dateCompare !== 0) {
      return dateCompare < 0 ? -1 : 1;
    }

    if (leftParts.timestamp && rightParts.timestamp) {
      const timestampCompare = leftParts.timestamp.localeCompare(rightParts.timestamp);
      if (timestampCompare !== 0) {
        return timestampCompare < 0 ? -1 : 1;
      }
    }

    return 0;
  } catch {
    const fallback = leftRaw.localeCompare(rightRaw);
    if (fallback === 0) {
      return 0;
    }
    return fallback < 0 ? -1 : 1;
  }
}

function comparableInputToTimestamp(input: ComparableTimeInput): NyTimestampString {
  return input.length === 10 ? `${parseDate(input)} 00:00:00` : parseTimestamp(input);
}

export function fractionalDaysBetween(start: string, end: string): number | null {
  try {
    return localTimestampDiffSeconds(comparableInputToTimestamp(start), comparableInputToTimestamp(end)) / 86_400;
  } catch {
    return null;
  }
}

export function fractionalDaysUntil(target: string): number | null {
  return fractionalDaysBetween(now(), target);
}

export function fractionalDaysSince(input: string): number | null {
  return fractionalDaysBetween(input, now());
}
