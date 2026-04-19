import { fail } from './error';
import { fromUtcRfc3339String } from './internal';
import type { NyDateString } from './types';

export function dateObjectToNyDate(date: Date): NyDateString {
  if (!(date instanceof Date) || Number.isNaN(date.getTime())) {
    fail('invalid_date_object', 'invalid Date object');
  }
  return fromUtcRfc3339String(date.toISOString()).slice(0, 10);
}
