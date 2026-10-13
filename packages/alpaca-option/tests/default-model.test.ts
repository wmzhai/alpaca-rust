import assert from 'node:assert/strict';
import test from 'node:test';

import { pricing, probability } from '../src/index.ts';

const sample = {
  spot: 100,
  strike: 100,
  years: 0.75,
  rate: 0.05,
  dividendYield: 0,
  volatility: 0.25,
  optionRight: 'put' as const,
};

test('zero dividend call matches european and put matches the rust price', () => {
  const call = { ...sample, optionRight: 'call' as const };
  assert.ok(Math.abs(pricing.priceDefault(call) - pricing.priceBlackScholes(call)) < 1e-8);
  const put = pricing.priceDefault(sample);
  assert.ok(put + 1e-8 >= pricing.priceBlackScholes(sample));
  assert.ok(Math.abs(put - 7.103512) < 1e-4, String(put));
  const greeks = pricing.greeksDefault(sample);
  assert.ok(Number.isFinite(greeks.delta));
  assert.ok(Number.isFinite(greeks.rho));
});

test('probability ignores rate and dividend', () => {
  const base = probability.expiryProbabilityInRange({
    spot: 100,
    lowerPrice: 80,
    upperPrice: 120,
    years: 0.5,
    rate: 0.01,
    dividendYield: 0,
    volatility: 0.2,
  });
  const shifted = probability.expiryProbabilityInRange({
    spot: 100,
    lowerPrice: 80,
    upperPrice: 120,
    years: 0.5,
    rate: 0.08,
    dividendYield: 0.03,
    volatility: 0.2,
  });
  assert.ok(Math.abs(base - shifted) < 1e-12);
});
