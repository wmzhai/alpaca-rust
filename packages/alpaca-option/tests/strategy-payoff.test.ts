import test from 'node:test';
import assert from 'node:assert/strict';

import { expiration as timeExpiration } from '@alpaca/time';

import { OptionError } from '../src/error';
import { payoff, pricing } from '../src/index';
import type { OptionContract, StrategyBreakEvenInput, StrategyPnlInput, StrategyValuationPosition } from '../src/index';

function contract(expirationDate: string, strike: number, optionRight: 'call' | 'put'): OptionContract {
  const rightCode = optionRight === 'call' ? 'C' : 'P';
  const occStrike = Math.round(strike * 1000).toString().padStart(8, '0');
  const compactExpiration = expirationDate.replace(/-/g, '').slice(2);
  return {
    underlying_symbol: 'SPY',
    expiration_date: expirationDate,
    strike,
    option_right: optionRight,
    occ_symbol: `SPY${compactExpiration}${rightCode}${occStrike}`,
  };
}

test('strategyPnl mixes expired and unexpired positions', () => {
  const evaluationTime = '2025-03-21 16:00:00';
  const positions: StrategyValuationPosition[] = [
    {
      contract: contract('2025-03-21', 100, 'put'),
      quantity: -1,
      avg_entry_price: 2,
      implied_volatility: 0.30,
      mark_price: null,
      reference_underlying_price: null,
    },
    {
      contract: contract('2025-04-24', 95, 'put'),
      quantity: 1,
      avg_entry_price: 1,
      implied_volatility: 0.25,
      mark_price: null,
      reference_underlying_price: null,
    },
  ];

  const expectedLongValue = pricing.priceBlackScholes({
    spot: 97,
    strike: 95,
    years: timeExpiration.years('2025-04-24', evaluationTime),
    rate: 0.03,
    dividendYield: 0,
    volatility: 0.25,
    optionRight: 'put',
  });
  const expected = (expectedLongValue - 3) * 100 + 100;

  const actual = payoff.strategyPnl({
    positions,
    underlying_price: 97,
    evaluation_time: evaluationTime,
    entry_cost: null,
    rate: 0.03,
    dividend_yield: null,
    long_volatility_shift: null,
  } satisfies StrategyPnlInput);

  assert.ok(Math.abs(actual - expected) < 1e-9, `actual=${actual}, expected=${expected}`);
});

test('strategyPnl applies volatility shift only to long positions', () => {
  const evaluationTime = '2025-03-20 11:30:04';
  const positions: StrategyValuationPosition[] = [
    {
      contract: contract('2025-04-24', 100, 'call'),
      quantity: 1,
      avg_entry_price: 2.5,
      implied_volatility: 0.20,
      mark_price: null,
      reference_underlying_price: null,
    },
    {
      contract: contract('2025-04-24', 95, 'put'),
      quantity: -1,
      avg_entry_price: 1,
      implied_volatility: 0.30,
      mark_price: null,
      reference_underlying_price: null,
    },
  ];

  const years = timeExpiration.years('2025-04-24', evaluationTime);
  const expectedLong = pricing.priceBlackScholes({
    spot: 102,
    strike: 100,
    years,
    rate: 0.03,
    dividendYield: 0,
    volatility: 0.15,
    optionRight: 'call',
  });
  const expectedShort = pricing.priceBlackScholes({
    spot: 102,
    strike: 95,
    years,
    rate: 0.03,
    dividendYield: 0,
    volatility: 0.30,
    optionRight: 'put',
  });
  const expected = (expectedLong - expectedShort) * 100 - 150;

  const actual = payoff.strategyPnl({
    positions,
    underlying_price: 102,
    evaluation_time: evaluationTime,
    entry_cost: 150,
    rate: 0.03,
    dividend_yield: null,
    long_volatility_shift: -0.05,
  } satisfies StrategyPnlInput);

  assert.ok(Math.abs(actual - expected) < 1e-9, `actual=${actual}, expected=${expected}`);
});

test('strategyBreakEvenPoints finds credit strangle roots', () => {
  const actual = payoff.strategyBreakEvenPoints({
    positions: [
      {
        contract: contract('2025-03-21', 90, 'put'),
        quantity: -1,
        avg_entry_price: 1.5,
        implied_volatility: 0.25,
        mark_price: null,
        reference_underlying_price: null,
      },
      {
        contract: contract('2025-03-21', 110, 'call'),
        quantity: -1,
        avg_entry_price: 1.5,
        implied_volatility: 0.25,
        mark_price: null,
        reference_underlying_price: null,
      },
    ],
    evaluation_time: '2025-03-21 16:00:00',
    entry_cost: null,
    rate: 0.03,
    dividend_yield: null,
    long_volatility_shift: null,
    lower_bound: 50,
    upper_bound: 150,
    scan_step: 1,
    tolerance: 1e-9,
    maxIterations: 100,
  } satisfies StrategyBreakEvenInput);

  assert.equal(actual.length, 2);
  assert.ok(Math.abs(actual[0] - 87) < 1e-6, `actual=${JSON.stringify(actual)}`);
  assert.ok(Math.abs(actual[1] - 113) < 1e-6, `actual=${JSON.stringify(actual)}`);
});

test('strategyPnl requires entryCost or leg costs', () => {
  assert.throws(
    () => payoff.strategyPnl({
      positions: [{
        contract: contract('2025-04-24', 100, 'call'),
        quantity: 1,
        avg_entry_price: null,
        implied_volatility: 0.20,
        mark_price: null,
        reference_underlying_price: null,
      }],
      underlying_price: 102,
      evaluation_time: '2025-03-20 11:30:04',
      entry_cost: null,
      rate: 0.03,
      dividend_yield: null,
      long_volatility_shift: null,
    } satisfies StrategyPnlInput),
    (error: unknown) => error instanceof OptionError && error.code === 'invalid_strategy_payoff_input',
  );
});
