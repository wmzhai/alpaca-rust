import assert from 'node:assert/strict';
import test from 'node:test';

import { probability } from '../src/index.ts';

const spot = 100;
const days = 365;
const volatility = 0.2;

function island(low: number, high: number): number {
  const lower = probability.chanceOfTarget({ spot, target: low, days, volatility }).below;
  const upper = probability.chanceOfTarget({ spot, target: high, days, volatility }).below;
  return upper - lower;
}

test('one breakeven uses the side named by its direction', () => {
  const above = probability.chanceOfProfit({
    spot,
    days,
    volatility,
    breakevens: [{ at: 105, direction: 1 }],
    maxProfit: 100,
    minPnl: -50,
    pnlWithoutBreakEven: -10,
  });
  const below = probability.chanceOfTarget({ spot, target: 105, days, volatility }).above;
  assert.ok(Math.abs(above - below) < 1e-12);

  const under = probability.chanceOfProfit({
    spot,
    days,
    volatility,
    breakevens: [{ at: 105, direction: -1 }],
    maxProfit: 100,
    minPnl: -50,
    pnlWithoutBreakEven: 10,
  });
  assert.ok(Math.abs(under - (1 - below)) < 1e-12);
});

test('two profit islands add both lognormal masses', () => {
  const breakevens = [
    { at: 80, direction: 1 as const },
    { at: 90, direction: -1 as const },
    { at: 110, direction: 1 as const },
    { at: 130, direction: -1 as const },
  ];
  const actual = probability.chanceOfProfit({
    spot,
    days,
    volatility,
    breakevens,
    maxProfit: Number.POSITIVE_INFINITY,
    minPnl: Number.NEGATIVE_INFINITY,
    pnlWithoutBreakEven: -1,
  });
  const expected = island(80, 90) + island(110, 130);
  assert.ok(Math.abs(actual - expected) < 1e-12, `${actual} vs ${expected}`);
});

test('no way to profit is zero and no way to lose is one', () => {
  const breakevens = [{ at: 90, direction: 1 as const }, { at: 110, direction: -1 as const }];
  assert.equal(probability.chanceOfProfit({
    spot,
    days,
    volatility,
    breakevens,
    maxProfit: 0,
    minPnl: -10,
    pnlWithoutBreakEven: -1,
  }), 0);
  assert.equal(probability.chanceOfProfit({
    spot,
    days,
    volatility,
    breakevens,
    maxProfit: 10,
    minPnl: 0,
    pnlWithoutBreakEven: 1,
  }), 1);
});

test('sampled sign changes keep a direction for every root', () => {
  const roots = probability.directedBreakEvens({
    prices: [70, 85, 100, 115, 140],
    pnls: [-5, 5, -5, 5, -5],
  });
  assert.deepEqual(roots.map((root) => root.direction), [1, -1, 1, -1]);
  assert.deepEqual(roots.map((root) => root.at), [77.5, 92.5, 107.5, 127.5]);
});

test('polished root lands on a linear zero', () => {
  const roots = probability.polishedDirectedBreakEvens({
    prices: [70, 140],
    evaluate: (price) => price - 90,
    tolerance: 1e-8,
    maxIterations: 40,
  });
  assert.equal(roots.length, 1);
  assert.equal(roots[0]?.direction, 1);
  assert.ok(Math.abs((roots[0]?.at ?? 0) - 90) < 1e-6);
});
