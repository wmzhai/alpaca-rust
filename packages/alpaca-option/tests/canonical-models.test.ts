import test from 'node:test';
import assert from 'node:assert/strict';

import { analysis, contract, url } from '../src/index';

test('contract helpers expose snake_case canonical contract objects', () => {
  assert.deepEqual(
    contract.canonicalContract('SPY250321P00580000') as Record<string, unknown> | null,
    {
      underlying_symbol: 'SPY',
      expiration_date: '2025-03-21',
      strike: 580,
      option_right: 'put',
      occ_symbol: 'SPY250321P00580000',
    },
  );
});

test('analysis and url helpers consume snake_case canonical positions directly', () => {
  const positions = [
    {
      contract: 'SPY250321C00100000',
      qty: -1,
      avg_cost: '5.25',
      leg_type: 'shortcall',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: {
          underlying_symbol: 'SPY',
          expiration_date: '2025-03-21',
          strike: 100,
          option_right: 'call',
          occ_symbol: 'SPY250321C00100000',
        },
        quote: {
          bid: 5.1,
          ask: 5.4,
          mark: 5.25,
          last: 5.25,
        },
        greeks: null,
        implied_volatility: 0.24,
        underlying_price: 105,
      },
    },
  ] as never;

  assert.deepEqual(
    analysis.shortItmPositions(105, positions).map((item) => item.contract as unknown as Record<string, unknown>),
    [{
      underlying_symbol: 'SPY',
      expiration_date: '2025-03-21',
      strike: 100,
      option_right: 'call',
      occ_symbol: 'SPY250321C00100000',
    }],
  );

  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'SPY',
      positions,
    }),
    'https://optionstrat.com/build/custom/SPY/-.SPY250321C100x1@5.25',
  );
});
