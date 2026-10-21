import assert from 'node:assert/strict';
import test from 'node:test';

import { OptionStrategy, pricing, probability } from '../src/index.ts';

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

test('spot zero is the american intrinsic value', () => {
  const call = pricing.priceDefault({ ...sample, spot: 0, optionRight: 'call' });
  assert.equal(call, 0);
  const put = pricing.priceDefault({ ...sample, spot: 0 });
  assert.equal(put, sample.strike);
  assert.throws(() => pricing.priceDefault({ ...sample, spot: -1 }));
  assert.throws(() => pricing.priceBlackScholes({ ...sample, spot: 0 }));
});

test('several expirations scan five moves and price zero', () => {
  assert.deepEqual(probability.profitLossScanWindow({ spot: 160, impliedMove: 20, strikes: [] }), {
    lower: 60,
    upper: 260,
  });
  assert.deepEqual(probability.profitLossScanWindow({ spot: 100, impliedMove: 40, strikes: [80] }), {
    lower: 0,
    upper: 300,
  });
  assert.deepEqual(probability.profitLossScanWindow({ spot: 100, impliedMove: 1, strikes: [5, 400] }), {
    lower: 5,
    upper: 400,
  });

  const seen: number[] = [];
  const bounds = probability.profitLossBounds({
    evaluate: (price) => {
      seen.push(price);
      return price === 0 ? -4153 : -268;
    },
    spot: 160,
    strikes: [115, 150],
    singleExpiration: false,
    impliedMove: 80,
    samples: 20,
  });
  assert.equal(bounds.minPnl, -4153);
  assert.ok(seen.includes(0));
  assert.ok(seen.some((price) => price >= 160 * 100));
});

test('call diagonal at spot zero loses the full debit', () => {
  const long = {
    contract: 'SPCX270820C00115000',
    snapshot: {
      as_of: '2026-10-21 09:30:00',
      contract: {
        underlying_symbol: 'SPCX',
        expiration_date: '2027-08-20',
        strike: 115,
        option_right: 'call' as const,
        occ_symbol: 'SPCX270820C00115000',
      },
      quote: { bid: 55.63, ask: 55.63, mark: 55.63, last: 55.63 },
      greeks: null,
      implied_volatility: 0.4,
      underlying_price: 160,
    },
    qty: 1,
    avg_cost: '55.6300',
    leg_type: '',
    option_right: null,
    strike: null,
    valuation_years: null,
  };
  const short = {
    contract: 'SPCX261113C00150000',
    snapshot: {
      as_of: '2026-10-21 09:30:00',
      contract: {
        underlying_symbol: 'SPCX',
        expiration_date: '2026-11-13',
        strike: 150,
        option_right: 'call' as const,
        occ_symbol: 'SPCX261113C00150000',
      },
      quote: { bid: 14.1, ask: 14.1, mark: 14.1, last: 14.1 },
      greeks: null,
      implied_volatility: 0.54,
      underlying_price: 160,
    },
    qty: -1,
    avg_cost: '14.1000',
    leg_type: '',
    option_right: null,
    strike: null,
    valuation_years: null,
  };
  const strategy = OptionStrategy.prepare({
    positions: [long, short],
    qty: 1,
    evaluation_time: '2026-10-21 09:30:00',
    entry_cost: null,
    dividend_yield: 0,
  });
  assert.equal(strategy.pnlAt(0), -4153);
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
