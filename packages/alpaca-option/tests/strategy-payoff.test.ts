import test from 'node:test';
import assert from 'node:assert/strict';

import { expiration as timeExpiration } from '@alpaca/time';

import { OptionError } from '../src/error';
import { payoff, pricing } from '../src/index';
import type {
  Greeks,
  OptionContract,
  OptionPosition,
  OptionStrategyInput,
  OptionSnapshot,
  StrategyBreakEvenInput,
  StrategyPnlInput,
  StrategyValuationPosition,
} from '../src/index';

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

function strategyPosition(
  expirationDate: string,
  strike: number,
  optionRight: 'call' | 'put',
  quantity: number,
  avgEntryPrice: number,
  impliedVolatility: number,
): StrategyValuationPosition {
  return {
    contract: contract(expirationDate, strike, optionRight),
    quantity,
    avg_entry_price: avgEntryPrice,
    implied_volatility: impliedVolatility,
    mark_price: avgEntryPrice,
    reference_underlying_price: 100,
  };
}

function optionPosition(
  expirationDate: string,
  strike: number,
  optionRight: 'call' | 'put',
  quantity: number,
  greeks: Greeks,
): OptionPosition {
  const resolvedContract = contract(expirationDate, strike, optionRight);
  const snapshot: OptionSnapshot = {
    as_of: '2025-03-20 10:30:00',
    contract: resolvedContract,
    quote: {
      bid: 1,
      ask: 1.2,
      mark: 1.1,
      last: 1.1,
    },
    greeks,
    implied_volatility: 0.25,
    underlying_price: 100,
  };

  return {
    contract: resolvedContract.occ_symbol,
    snapshot,
    qty: quantity,
    avg_cost: '1.10',
    leg_type: '',
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

test('OptionStrategy uses earliest expiration close by default', () => {
  const positions = [
    strategyPosition('2025-05-16', 100, 'call', 1, 5, 0.24),
    strategyPosition('2025-04-17', 105, 'call', -1, 1.5, 0.31),
    strategyPosition('2025-06-20', 90, 'put', 1, 2, 0.28),
  ];

  assert.equal(payoff.OptionStrategy.expirationTime(positions), '2025-04-17 16:00:00');

  const strategy = payoff.OptionStrategy.fromInput({
    positions,
    evaluation_time: null,
    entry_cost: null,
    rate: 0.03,
    dividend_yield: null,
    long_volatility_shift: null,
  } satisfies OptionStrategyInput);

  const direct = strategy.pnlAt(100);
  const curve = strategy.sampleCurve({ lower_bound: 90, upper_bound: 110, step: 10 });
  assert.equal(curve.length, 3);
  assert.ok(Math.abs(curve[1].pnl - direct) < 1e-9);
});

test('OptionStrategy finds break even between bracketed prices', () => {
  const strategy = payoff.OptionStrategy.prepare({
    positions: [strategyPosition('2025-03-21', 100, 'call', 1, 5, 0.30)],
    evaluation_time: '2025-03-21 16:00:00',
    entry_cost: null,
    rate: 0.03,
    dividend_yield: null,
    long_volatility_shift: null,
  });

  const root = strategy.breakEvenBetween({
    lower_bound: 100,
    upper_bound: 110,
    tolerance: 1e-9,
    maxIterations: 100,
  });
  assert.ok(root != null);
  assert.ok(Math.abs(root - 105) < 1e-6, `root=${root}`);

  const noRoot = strategy.breakEvenBetween({
    lower_bound: 90,
    upper_bound: 100,
    tolerance: 1e-9,
    maxIterations: 100,
  });
  assert.equal(noRoot, null);
});

test('OptionStrategy aggregates snapshot Greeks with strategy quantity', () => {
  const positions = [
    optionPosition('2025-04-17', 100, 'call', 1, {
      delta: 0.50,
      gamma: 0.01,
      vega: 0.08,
      theta: -0.03,
      rho: 0.02,
    }),
    optionPosition('2025-04-17', 95, 'put', -2, {
      delta: -0.25,
      gamma: 0.02,
      vega: 0.05,
      theta: -0.02,
      rho: -0.01,
    }),
  ];

  const actual = payoff.OptionStrategy.aggregateSnapshotGreeks({ positions, strategyQuantity: 3 });

  assert.ok(Math.abs(actual.delta - 300) < 1e-9);
  assert.ok(Math.abs(actual.gamma - -9) < 1e-9);
  assert.ok(Math.abs(actual.vega - -6) < 1e-9);
  assert.ok(Math.abs(actual.theta - 3) < 1e-9);
  assert.ok(Math.abs(actual.rho - 12) < 1e-9);
});

test('OptionStrategy aggregates model Greeks with strategy quantity', () => {
  const evaluationTime = '2025-03-20 11:30:04';
  const positions = [
    strategyPosition('2025-04-17', 100, 'call', 1, 4, 0.22),
    strategyPosition('2025-04-17', 105, 'call', -1, 1.5, 0.30),
  ];

  const actual = payoff.OptionStrategy.aggregateModelGreeks({
    positions,
    underlying_price: 102,
    evaluation_time: evaluationTime,
    rate: 0.03,
    dividend_yield: null,
    long_volatility_shift: null,
    strategyQuantity: 2,
  });

  const years = timeExpiration.years('2025-04-17', evaluationTime);
  const long = pricing.greeksBlackScholes({
    spot: 102,
    strike: 100,
    years,
    rate: 0.03,
    dividendYield: 0,
    volatility: 0.22,
    optionRight: 'call',
  });
  const short = pricing.greeksBlackScholes({
    spot: 102,
    strike: 105,
    years,
    rate: 0.03,
    dividendYield: 0,
    volatility: 0.30,
    optionRight: 'call',
  });

  assert.ok(Math.abs(actual.delta - (long.delta - short.delta) * 200) < 1e-9);
  assert.ok(Math.abs(actual.gamma - (long.gamma - short.gamma) * 200) < 1e-9);
  assert.ok(Math.abs(actual.vega - (long.vega - short.vega) * 200) < 1e-9);
  assert.ok(Math.abs(actual.theta - (long.theta - short.theta) * 200) < 1e-9);
  assert.ok(Math.abs(actual.rho - (long.rho - short.rho) * 200) < 1e-9);
});

test('OptionStrategy values common multi-leg shapes', () => {
  const cases: Array<{ name: string; positions: StrategyValuationPosition[]; pivot: number }> = [
    {
      name: 'pmcc',
      positions: [
        strategyPosition('2025-06-20', 95, 'call', 1, 12, 0.24),
        strategyPosition('2025-04-17', 105, 'call', -1, 2, 0.30),
      ],
      pivot: 105,
    },
    {
      name: 'double_diagonal',
      positions: [
        strategyPosition('2025-05-16', 90, 'put', 1, 2.2, 0.27),
        strategyPosition('2025-04-17', 95, 'put', -1, 1.4, 0.32),
        strategyPosition('2025-04-17', 105, 'call', -1, 1.5, 0.31),
        strategyPosition('2025-05-16', 110, 'call', 1, 2.4, 0.26),
      ],
      pivot: 100,
    },
    {
      name: 'broken_wing_plus_diagonal',
      positions: [
        strategyPosition('2025-04-17', 100, 'call', 1, 4, 0.25),
        strategyPosition('2025-04-17', 105, 'call', -2, 2, 0.27),
        strategyPosition('2025-04-17', 115, 'call', 1, 0.6, 0.30),
        strategyPosition('2025-04-17', 95, 'put', -1, 1.1, 0.30),
        strategyPosition('2025-05-16', 90, 'put', 1, 1.4, 0.25),
      ],
      pivot: 105,
    },
    {
      name: 'iron_condor',
      positions: [
        strategyPosition('2025-04-17', 90, 'put', 1, 0.6, 0.30),
        strategyPosition('2025-04-17', 95, 'put', -1, 1.3, 0.30),
        strategyPosition('2025-04-17', 105, 'call', -1, 1.2, 0.30),
        strategyPosition('2025-04-17', 110, 'call', 1, 0.5, 0.30),
      ],
      pivot: 100,
    },
    {
      name: 'short_straddle',
      positions: [
        strategyPosition('2025-04-17', 100, 'call', -1, 3, 0.30),
        strategyPosition('2025-04-17', 100, 'put', -1, 2.8, 0.30),
      ],
      pivot: 100,
    },
    {
      name: 'call_butterfly',
      positions: [
        strategyPosition('2025-04-17', 95, 'call', 1, 6, 0.30),
        strategyPosition('2025-04-17', 100, 'call', -2, 3, 0.30),
        strategyPosition('2025-04-17', 105, 'call', 1, 1, 0.30),
      ],
      pivot: 100,
    },
  ];

  for (const item of cases) {
    const strategy = payoff.OptionStrategy.fromInput({
      positions: item.positions,
      evaluation_time: null,
      entry_cost: null,
      rate: 0.03,
      dividend_yield: null,
      long_volatility_shift: null,
    });
    const pivotPnl = strategy.pnlAt(item.pivot);
    assert.ok(Number.isFinite(pivotPnl), `${item.name}: non-finite pivot pnl`);

    const curve = strategy.sampleCurve({
      lower_bound: Math.max(item.pivot * 0.8, 1),
      upper_bound: item.pivot * 1.2,
      step: item.pivot * 0.1,
    });
    assert.ok(curve.length >= 5, `${item.name}: curve too small`);
    assert.ok(
      curve.every((point) => Number.isFinite(point.underlying_price)
        && Number.isFinite(point.mark_value)
        && Number.isFinite(point.pnl)),
      `${item.name}: curve contains non-finite point`,
    );
  }
});
