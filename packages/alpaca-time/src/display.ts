import { minutesFromHhmm, parseDate, parseHhmm, parseTimestamp } from './clock';
import { fail } from './error';
import {
  datePartsToUtcDate,
  durationPartsFromSeconds,
  parseDateParts,
  localTimestampDiffSeconds,
  weekdayCodeFromDate,
} from './internal';
import type {
  ComparableTimeInput,
  DurationParts,
  HhmmString,
  NyDateString,
  NyTimestampString,
  WeekdayCode,
} from './types';

type CompactStyle =
  | 'mm-dd'
  | 'yy-mm-dd'
  | 'yymmdd'
  | 'mm-dd hh:mm'
  | 'yy-mm-dd hh:mm'
  | 'yyyy-mm-dd hh:mm';

type DateCompactStyle = 'mm-dd' | 'yy-mm-dd' | 'yymmdd';
type TimestampCompactStyle = 'mm-dd hh:mm' | 'yy-mm-dd hh:mm' | 'yyyy-mm-dd hh:mm';

function isTimestampCompactStyle(style: CompactStyle): style is TimestampCompactStyle {
  return style.includes('hh:mm');
}

function compactDate(date: string, style: DateCompactStyle): string {
  const canonical = parseDate(date);
  if (style === 'mm-dd') {
    return canonical.slice(5, 10);
  }
  if (style === 'yy-mm-dd') {
    return canonical.slice(2, 10);
  }
  if (style === 'yymmdd') {
    return `${canonical.slice(2, 4)}${canonical.slice(5, 7)}${canonical.slice(8, 10)}`;
  }
  fail('invalid_compact_date_style', `invalid compact date style: ${style}`);
}

function compactTimestamp(timestamp: string, style: TimestampCompactStyle): string {
  const canonical = parseTimestamp(timestamp);
  if (style === 'mm-dd hh:mm') {
    return `${canonical.slice(5, 10)} ${canonical.slice(11, 16)}`;
  }
  if (style === 'yy-mm-dd hh:mm') {
    return `${canonical.slice(2, 10)} ${canonical.slice(11, 16)}`;
  }
  if (style === 'yyyy-mm-dd hh:mm') {
    return canonical.slice(0, 16);
  }
  fail('invalid_compact_timestamp_style', `invalid compact timestamp style: ${style}`);
}

function timeOnly(timestamp: string, precision: 'minute' | 'second' = 'minute'): string {
  const canonical = parseTimestamp(timestamp);
  if (precision === 'minute') {
    return canonical.slice(11, 16);
  }
  if (precision === 'second') {
    return canonical.slice(11, 19);
  }
  fail('invalid_time_only_precision', `invalid time precision: ${precision}`);
}

export function compact(input: ComparableTimeInput, style: CompactStyle): string {
  try {
    if (isTimestampCompactStyle(style)) {
      if (input.length === 10) {
        const canonical = parseDate(input);
        if (style === 'mm-dd hh:mm') {
          return compactDate(canonical, 'mm-dd');
        }
        if (style === 'yy-mm-dd hh:mm') {
          return compactDate(canonical, 'yy-mm-dd');
        }
        return canonical;
      }

      return compactTimestamp(input, style);
    }

    return compactDate(input.length === 10 ? input : parseTimestamp(input).slice(0, 10), style);
  } catch {
    return input;
  }
}

export function time(
  input: ComparableTimeInput,
  precision: 'minute' | 'second' = 'minute',
  dateStyle: DateCompactStyle = 'mm-dd',
): string {
  try {
    if (input.length === 10) {
      return compactDate(input, dateStyle);
    }

    return timeOnly(input, precision);
  } catch {
    return input;
  }
}

export function hhmm(input: string): HhmmString {
  return parseHhmm(input);
}

export function duration(start: HhmmString, end: HhmmString): string {
  try {
    const startMinutes = minutesFromHhmm(start);
    const endMinutes = minutesFromHhmm(end);
    const diffMinutes = Math.max(endMinutes - startMinutes, 0);
    const hours = Math.floor(diffMinutes / 60);
    const minutes = diffMinutes % 60;
    return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
  } catch {
    return '-';
  }
}

export function weekdayCode(date: NyDateString): WeekdayCode {
  return weekdayCodeFromDate(datePartsToUtcDate(parseDateParts(date)));
}

export function weekdayShortZh(date: NyDateString): '一' | '二' | '三' | '四' | '五' | '六' | '日' {
  switch (weekdayCode(date)) {
    case 'mon':
      return '一';
    case 'tue':
      return '二';
    case 'wed':
      return '三';
    case 'thu':
      return '四';
    case 'fri':
      return '五';
    case 'sat':
      return '六';
    default:
      return '日';
  }
}

export function durationParts(seconds: number): DurationParts {
  return durationPartsFromSeconds(seconds);
}

export function relativeDurationParts(from: NyTimestampString, to: NyTimestampString): DurationParts {
  return durationPartsFromSeconds(localTimestampDiffSeconds(from, to));
}

export function compactDuration(parts: DurationParts, style: 'hm' | 'dhm' = 'hm'): string {
  const sign = parts.sign < 0 ? '-' : '';

  if (style === 'hm') {
    const totalHours = parts.days * 24 + parts.hours;
    const body = totalHours > 0 ? `${totalHours}h ${parts.minutes}m` : `${parts.minutes}m`;
    return `${sign}${body}`;
  }

  if (style === 'dhm') {
    if (parts.days > 0) {
      return `${sign}${parts.days}d ${parts.hours}h ${parts.minutes}m`;
    }
    if (parts.hours > 0) {
      return `${sign}${parts.hours}h ${parts.minutes}m`;
    }
    return `${sign}${parts.minutes}m`;
  }

  fail('invalid_compact_duration_style', `invalid compact duration style: ${style}`);
}

export function compactDaysUntil(days: number): string {
  if (Math.abs(days) < 1) {
    return `${(days * 24).toFixed(1)}h`;
  }
  return `${days.toFixed(1)}D`;
}
