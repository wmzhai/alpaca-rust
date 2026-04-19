import test from 'node:test';
import assert from 'node:assert/strict';

import { numeric } from '../src/index';

test('evaluatePoints returns function values in order', () => {
  const values = numeric.evaluatePoints([1, 2, 3.5], (spot) => spot * spot);
  assert.deepEqual(values, [1, 4, 12.25]);
});

test('refineBracketedRoot solves sign-change interval', () => {
  const root = numeric.refineBracketedRoot(1, 2, (spot) => spot * spot - 2, 1e-9, 100);
  assert.ok(Math.abs(root - Math.sqrt(2)) < 1e-7, `root=${root}`);
});

test('scanRangeExtrema finds min and max points', () => {
  const extrema = numeric.scanRangeExtrema(90, 110, 5, (spot) => (spot - 100) * 2);
  assert.equal(extrema.minSpot, 90);
  assert.equal(extrema.minValue, -20);
  assert.equal(extrema.maxSpot, 110);
  assert.equal(extrema.maxValue, 20);
});
