import assert from 'node:assert/strict';
import test from 'node:test';

import * as alpacaTime from '../src/index.ts';

test('clock helpers preserve granularity and canonicalize RFC3339 UTC input', () => {
  assert.equal(alpacaTime.clock.parseDateOrTimestamp('2025-01-02'), '2025-01-02');
  assert.equal(
    alpacaTime.clock.parseDateOrTimestamp('2025-01-02T14:30:15Z'),
    '2025-01-02 09:30:15',
  );
  assert.equal(
    alpacaTime.clock.firstDateOrTimestamp([null, '', '2025-01-02T14:30:15Z']),
    '2025-01-02 09:30:15',
  );
});

test('compareDateOrTimestamp does not invent intraday order for date-only inputs', () => {
  assert.equal(
    alpacaTime.clock.compareDateOrTimestamp('2025-01-02', '2025-01-02 23:59:59'),
    0,
  );
  assert.equal(
    alpacaTime.clock.compareDateOrTimestamp('2025-01-02 09:30:00', '2025-01-02 09:31:00'),
    -1,
  );
});

test('display and browser helpers keep documented fallback behavior', () => {
  assert.equal(alpacaTime.display.compact('2025-01-02', 'yy-mm-dd hh:mm'), '25-01-02');
  assert.equal(alpacaTime.display.time('2025-01-02', 'minute', 'yymmdd'), '250102');
  assert.equal(alpacaTime.display.compact('not-a-date', 'mm-dd'), 'not-a-date');
  assert.equal(
    alpacaTime.browser.dateObjectToNyDate(new Date('2025-01-02T04:30:00Z')),
    '2025-01-01',
  );
});

test('range helpers expose canonical week boundaries', () => {
  assert.deepEqual(alpacaTime.range.calendarWeekRange('2025-01-08'), {
    start_date: '2025-01-06',
    end_date: '2025-01-12',
  });
  assert.deepEqual(alpacaTime.range.isoWeekRange(2025, 2), {
    start_date: '2025-01-06',
    end_date: '2025-01-12',
  });
});
