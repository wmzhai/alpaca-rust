import test from 'node:test';
import assert from 'node:assert/strict';

import { analysis, chain, contract, display, executionQuote, numeric, payoff, pricing, probability, snapshot, url } from '../src/index';
import { OptionError } from '../src/error';
import * as mathAmerican from '../src/math/american';
import * as mathBarrier from '../src/math/barrier';
import * as mathGeometricAsian from '../src/math/geometricAsian';
import type {
  LiquidityBatchResponse,
  LiquidityData,
  LiquidityOptionData,
  LiquidityStats,
  OptionContract,
  OptionPosition,
  OptionSnapshot,
} from '../src/index';

function assertOptionError(fn: () => unknown, expectedCode: string): void {
  assert.throws(
    fn,
    (error: unknown) => error instanceof OptionError && error.code === expectedCode,
  );
}

function sampleContract(): OptionContract {
  return {
    underlying_symbol: 'SPY',
    expiration_date: '2025-03-21',
    strike: 600,
    option_right: 'call',
    occ_symbol: 'SPY250321C00600000',
  };
}

function sampleSnapshot(bid: number | null, ask: number | null): OptionSnapshot {
  return {
    as_of: '2025-02-06 11:30:04',
    contract: sampleContract(),
    quote: {
      bid,
      ask,
      mark: bid != null && ask != null ? (bid + ask) / 2 : (bid ?? ask ?? null),
      last: null,
    },
    greeks: null,
    implied_volatility: null,
    underlying_price: null,
  };
}

test('display.formatStrike keeps expected frontend shape', () => {
  assert.equal(display.formatStrike(600), '600');
  assert.equal(display.formatStrike(600.5), '600.5');
  assert.equal(display.formatStrike(600.125), '600.125');
  assert.equal(display.formatStrike(600.12), '600.12');
});

test('liquidity types stay available from root exports', () => {
  const option: LiquidityOptionData = {
    occ_symbol: 'SPY250321P00580000',
    option_right: 'put',
    strike: 580,
    expiration_date: '2025-03-21',
    dte: 43,
    delta: 0.3,
    spread_pct: 8.89,
    liquidity: true,
    bid: 2.15,
    ask: 2.35,
    mark: 2.25,
    implied_volatility: 0.28,
  };
  const stats: LiquidityStats = {
    total_count: 1,
    avg_spread_pct: 8.89,
    median_spread_pct: 8.89,
    min_spread_pct: 8.89,
    max_spread_pct: 8.89,
    dte_range: [43, 43],
    delta_range: [0.3, 0.3],
  };
  const data: LiquidityData = {
    underlying_symbol: 'SPY',
    as_of: '2025-02-06 11:30:04',
    underlying_price: 600,
    options: [option],
    stats,
  };
  const response: LiquidityBatchResponse = {
    results: {
      SPY: data,
    },
  };

  assert.equal(response.results.SPY.options[0].occ_symbol, 'SPY250321P00580000');
});

test('display.contractDisplay absorbs raw contract display fields directly', () => {
  assert.deepEqual(
    display.contractDisplay('SPY250321P00580000', 'yy-mm-dd'),
    {
      strike: '580',
      expiration: '25-03-21',
      compact: '580P@25-03-21',
      optionRightCode: 'P',
    },
  );
  assert.equal(
    display.contractDisplay('bad-contract'),
    null,
  );
});

test('display.compactContract and positionStrike use canonical contracts directly', () => {
  assert.equal(
    display.compactContract('SPY250321P00580000'),
    '580P@03-21',
  );
  assert.equal(
    display.compactContract({
      underlying_symbol: 'SPY',
      expiration_date: '2025-03-21',
      strike: 600,
      option_right: 'call',
      occ_symbol: 'SPY250321C00600000',
    }, 'yy-mm-dd'),
    '600C@25-03-21',
  );
  assert.equal(
    display.compactContract('bad-contract'),
    '-',
  );
});

test('display.positionStrike and analysis.positionOtmPercent read direct OCC positions', () => {
  assert.equal(display.positionStrike({ contract: 'SPY250321P00580000' }), '580');
  assert.equal(display.positionStrike({ contract: 'bad-contract' }), '-');
  assert.equal(
    Number(analysis.positionOtmPercent(598, { contract: 'SPY250321P00580000' })?.toFixed(4)),
    Number((((598 - 580) / 598) * 100).toFixed(4)),
  );
  assert.equal(analysis.positionOtmPercent(598, { contract: 'bad-contract' }), null);
});

test('snapshot helpers use canonical snapshots directly', () => {
  const canonical = sampleSnapshot(1.1, 1.3);
  const rustCanonical = {
    as_of: '2025-02-06 11:30:04',
    contract: {
      underlying_symbol: 'SPY',
      expiration_date: '2025-03-21',
      strike: 580,
      option_right: 'put',
      occ_symbol: 'SPY250321P00580000',
    },
    quote: {
      bid: 1.1,
      ask: 1.3,
      mark: 1.2,
      last: 1.2,
    },
    greeks: {
      delta: -0.25,
      gamma: 0.02,
      vega: 0.11,
      theta: -0.03,
      rho: -0.01,
    },
    implied_volatility: 0.22,
    underlying_price: 598.75,
  };
  const executionSnapshot = {
    contract: 'SPY250321P00580000',
    timestamp: '2025-02-06 11:30:04',
    bid: '1.10',
    ask: '1.30',
    price: '1.20',
    greeks: {
      delta: -0.25,
      gamma: 0.02,
      vega: 0.11,
      theta: -0.03,
      rho: -0.01,
    },
    iv: 0.22,
  };

  assert.equal(snapshot.contract(canonical)?.occ_symbol, 'SPY250321C00600000');
  assert.equal(snapshot.contract(rustCanonical)?.occ_symbol, 'SPY250321P00580000');
  assert.equal(snapshot.contract(executionSnapshot)?.occ_symbol, 'SPY250321P00580000');
  assert.equal(snapshot.spread(canonical), 0.2);
  assert.equal(snapshot.spread(rustCanonical), 0.2);
  assert.equal(snapshot.spread(executionSnapshot), 0.2);
  assert.equal(Number(snapshot.spreadPct(canonical).toFixed(6)), Number((0.2 / 1.2).toFixed(6)));
  assert.equal(Number(snapshot.spreadPct(rustCanonical).toFixed(6)), Number((0.2 / 1.2).toFixed(6)));
  assert.equal(snapshot.isValid(canonical), true);
  assert.equal(snapshot.isValid(rustCanonical), true);
  assert.equal(snapshot.isValid(executionSnapshot), true);
  assert.equal(typeof snapshot.liquidity(rustCanonical), 'boolean');
  assert.equal(typeof snapshot.liquidity(executionSnapshot), 'boolean');
  assert.equal(snapshot.contract({
    contract: 'bad-contract',
    timestamp: '2025-02-06 11:30:04',
    bid: '1.10',
    ask: '1.30',
    price: '1.20',
    greeks: {
      delta: -0.25,
      gamma: 0.02,
      vega: 0.11,
      theta: -0.03,
      rho: -0.01,
    },
    iv: 0.22,
  }), null);
  assert.equal(snapshot.isValid({
    contract: 'bad-contract',
    timestamp: '',
    bid: '1.10',
    ask: '1.30',
    price: '1.20',
    greeks: {
      delta: -0.25,
      gamma: 0.02,
      vega: 0.11,
      theta: -0.03,
      rho: -0.01,
    },
    iv: 0.22,
  }), false);
});

test('contract.buildOccSymbol and parseOccSymbol absorb invalid OCC inputs directly', () => {
  assert.equal(
    contract.buildOccSymbol('SPY', '2025-03-21', 600.1254, 'call'),
    'SPY250321C00600125',
  );

  assert.equal(contract.parseOccSymbol('SPY250321X00600000'), null);
  assert.equal(contract.parseOccSymbol('SPY250232C00600000'), null);
  assert.equal(contract.parseOccSymbol('SPY250321C00600A00'), null);
  assert.equal(contract.buildOccSymbol('BRK.B-', '2025-03-21', 600, 'call'), null);
  assert.equal(
    contract.buildOccSymbol('SPY', '2025-03-21', '600.1254', 'C'),
    'SPY250321C00600125',
  );
  assert.equal(
    contract.buildOccSymbol('SPY', '2025-03-21', 'bad', 'call'),
    null,
  );
  assert.deepEqual(
    contract.canonicalContract('SPY250321P00580000'),
    {
      underlying_symbol: 'SPY',
      expiration_date: '2025-03-21',
      strike: 580,
      option_right: 'put',
      occ_symbol: 'SPY250321P00580000',
    },
  );
  assert.deepEqual(
    contract.canonicalContract({
      underlying_symbol: 'SPY',
      expiration_date: '2025-03-21',
      strike: 580,
      option_right: 'put',
      occ_symbol: 'SPY250321P00580000',
    }),
    {
      underlying_symbol: 'SPY',
      expiration_date: '2025-03-21',
      strike: 580,
      option_right: 'put',
      occ_symbol: 'SPY250321P00580000',
    },
  );
  assert.equal(
    contract.canonicalContract('bad-contract'),
    null,
  );
});

test('optionstrat helpers cover query/hash, optional premium, and underlying mismatch', () => {
  const parsed = url.parseOptionstratUrl('https://optionstrat.com/build/custom/BRK%2FB/.BRKB250620P480x1@1.23?ref=abc#frag');
  assert.equal(parsed.underlyingDisplaySymbol, 'BRK.B');
  assert.deepEqual(parsed.legFragments, ['.BRKB250620P480x1@1.23']);

  const legs = url.parseOptionstratLegFragments('BRK.B', ['.BRKB250620P480x1']);
  assert.equal(legs.length, 1);
  assert.equal(legs[0].premiumPerContract, null);
  assert.equal(legs[0].ratioQuantity, 1);

  assertOptionError(() => url.parseOptionstratLegFragments('SPY', ['.QQQ250620P480x1@1.23']), 'invalid_optionstrat_leg_fragment');
  assert.equal(url.buildOptionstratLegFragment({
    occSymbol: 'SPY250321C00600000',
    quantity: 0,
    premiumPerContract: 1.23,
  }), null);

  assert.equal(
    url.buildOptionstratLegFragment({
      occSymbol: 'SPY250321P00580000',
      quantity: -1,
      premiumPerContract: 2.45,
    }),
    '-.SPY250321P580x1@2.45',
  );
  assert.equal(
    url.buildOptionstratLegFragment({
      occSymbol: 'SPY250321P00580000',
      quantity: -1,
      premiumPerContract: null,
    }),
    '-.SPY250321P580x1',
  );
  assert.equal(
    url.buildOptionstratStockFragment({
      underlyingSymbol: 'BRK.B',
      quantity: 100,
      costPerShare: 512.34,
    }),
    'BRKBx100@512.34',
  );
  assert.equal(
    url.buildOptionstratStockFragment({
      underlyingSymbol: 'BRK.B',
      quantity: 0,
      costPerShare: 512.34,
    }),
    null,
  );
  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'BRK.B',
      legs: [{ occSymbol: 'BRKB250620P00480000', quantity: -2, premiumPerContract: 12.34 }],
    }),
    'https://optionstrat.com/build/custom/BRK%2FB/-.BRKB250620P480x2@12.34',
  );
  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'SPY',
      legs: [{ occSymbol: 'SPY250321P00580000', quantity: '-1', premiumPerContract: '2.45' }],
    }),
    'https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45',
  );
  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'SPY',
      legs: [{
        underlyingSymbol: 'SPY',
        expirationDate: '2025-03-21',
        strike: 580,
        optionRight: 'put',
        quantity: '-1',
        premiumPerContract: '2.45',
      }],
    }),
    'https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45',
  );
  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'SPY',
      positions: [
        { contract: 'SPY250321P00580000', qty: -1, avg_cost: '2.45', leg_type: 'shortput', snapshot: sampleSnapshot(2.3, 2.6) },
        { contract: 'SPY250321C00600000', qty: 2, avg_cost: '0.00', leg_type: 'longcall', snapshot: sampleSnapshot(1.0, 1.2) },
      ],
    }),
    'https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45,.SPY250321C600x2@1.10',
  );
  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'SPY',
      positions: [
        {
          contract: 'SPY250321P00580000',
          qty: -1,
          avg_cost: '0.00',
          leg_type: 'shortput',
          snapshot: {
            as_of: '2025-02-06 11:30:04',
            contract: {
              underlying_symbol: 'SPY',
              expiration_date: '2025-03-21',
              strike: 580,
              option_right: 'put',
              occ_symbol: 'SPY250321P00580000',
            },
            quote: { bid: 2.4, ask: 2.5, mark: 2.45, last: 2.45 },
            greeks: null,
            implied_volatility: null,
            underlying_price: null,
          },
        },
        {
          contract: 'SPY250321C00600000',
          qty: 2,
          avg_cost: '0.00',
          leg_type: 'longcall',
          snapshot: {
            as_of: '2025-02-06 11:30:04',
            contract: {
              underlying_symbol: 'SPY',
              expiration_date: '2025-03-21',
              strike: 600,
              option_right: 'call',
              occ_symbol: 'SPY250321C00600000',
            },
            quote: { bid: 1.1, ask: 1.3, mark: 1.2, last: 1.2 },
            greeks: null,
            implied_volatility: null,
            underlying_price: null,
          },
        },
      ],
    }),
    'https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45,.SPY250321C600x2@1.20',
  );
  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'SPY',
      legs: [{ occSymbol: 'SPY250321P00580000', quantity: -1, premiumPerContract: 2.45 }],
      stocks: [{ underlyingSymbol: 'SPY', quantity: 100, costPerShare: 530.12 }],
    }),
    'https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45,SPYx100@530.12',
  );
  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'SPY',
      stocks: [{ underlyingSymbol: 'SPY', quantity: 100, costPerShare: 530.12 }],
    }),
    'https://optionstrat.com/build/custom/SPY/SPYx100@530.12',
  );
  assert.equal(
    url.buildOptionstratUrl({
      underlyingDisplaySymbol: 'SPY',
      legs: [],
    }),
    null,
  );

  assert.equal(
    url.mergeOptionstratUrls(
      [
        null,
        'bad-url',
        'https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45',
        'https://optionstrat.com/build/custom/BRK%2FB/.BRKB250620P480x1@1.23',
        'https://optionstrat.com/build/custom/SPY/.SPY250321C600x2?ref=abc',
      ],
      'SPY',
    ),
    'https://optionstrat.com/build/custom/SPY/-.SPY250321P580x1@2.45,.SPY250321C600x2',
  );
  assert.equal(
    url.mergeOptionstratUrls([
      'bad-url',
      'https://optionstrat.com/build/custom/BRK%2FB/.BRKB250620P480x1@1.23#frag',
    ]),
    'https://optionstrat.com/build/custom/BRK%2FB/.BRKB250620P480x1@1.23',
  );
  assert.equal(
    url.mergeOptionstratUrls(['bad-url', null], 'SPY'),
    null,
  );
});

test('chain helpers use canonical snapshots directly', () => {
  const snapshots = [
    {
      as_of: '2025-02-06 11:30:04',
      contract: {
        underlying_symbol: 'SPY',
        expiration_date: '2025-03-21',
        strike: 580,
        option_right: 'put',
        occ_symbol: 'SPY250321P00580000',
      },
      quote: { bid: 1.1, ask: 1.3, mark: 1.2, last: 1.2 },
      greeks: { delta: -0.12, gamma: 0, vega: 0, theta: 0, rho: 0 },
      implied_volatility: null,
      underlying_price: null,
    },
    {
      as_of: '2025-02-06 11:30:04',
      contract: {
        underlying_symbol: 'SPY',
        expiration_date: '2025-03-28',
        strike: 580,
        option_right: 'put',
        occ_symbol: 'SPY250328P00580000',
      },
      greeks: { delta: -0.18, gamma: 0, vega: 0, theta: 0, rho: 0 },
      quote: { bid: 1.4, ask: 1.6, mark: 1.5, last: 1.5 },
      implied_volatility: null,
      underlying_price: null,
    },
    {
      contract: {
        underlying_symbol: 'SPY',
        expiration_date: '2025-03-28',
        strike: 600,
        option_right: 'call',
        occ_symbol: 'SPY250328C00600000',
      },
      quote: { bid: 2.4, ask: 2.8, mark: 2.6 },
      greeks: null,
      implied_volatility: null,
      as_of: '2025-02-06 11:30:04',
      underlying_price: 598,
    },
  ];
  const optionChain = {
    underlying_symbol: 'SPY',
    as_of: '2025-02-06 11:30:04',
    snapshots,
  };

  assert.equal(
    chain.findSnapshot({
      chain: optionChain,
      optionRight: 'put',
      expirationDate: '2025-03-28',
      strike: '580',
    }),
    snapshots[1],
  );
  assert.deepEqual(
    chain.listSnapshots({
      chain: optionChain,
      expirationDate: '2025-03-28',
    }),
    [snapshots[1], snapshots[2]],
  );
  assert.deepEqual(
    chain.expirationDates({
      chain: optionChain,
      optionRight: 'put',
      minCalendarDays: 30,
      maxCalendarDays: 60,
      now: '2025-02-20 09:30:00',
    }),
    [
      { expirationDate: '2025-03-21', calendarDays: 29 },
      { expirationDate: '2025-03-28', calendarDays: 36 },
    ].filter((item) => item.calendarDays >= 30 && item.calendarDays <= 60),
  );
  assert.equal(
    chain.findSnapshot({
      chain: optionChain,
      occSymbol: 'SPY250328C00600000',
    }),
    snapshots[2],
  );
});

test('analysis.annualizedPremiumYieldDays keeps DTE based return semantics in core', () => {
  assert.equal(
    Number(analysis.annualizedPremiumYieldDays(2, 100, 14).toFixed(6)),
    Number((2 / 100 / (14 / 365)).toFixed(6)),
  );
  assertOptionError(() => analysis.annualizedPremiumYieldDays(2, 100, 0), 'invalid_analysis_input');
});

test('analysis.assignmentRisk exposes stable thresholds for app-side rendering', () => {
  assert.equal(analysis.assignmentRisk(-0.01), 'danger');
  assert.equal(analysis.assignmentRisk(0.03), 'critical');
  assert.equal(analysis.assignmentRisk(0.08), 'high');
  assert.equal(analysis.assignmentRisk(0.2), 'medium');
  assert.equal(analysis.assignmentRisk(0.5), 'low');
  assert.equal(analysis.assignmentRisk(1.2), 'safe');
});

test('analysis.shortExtrinsicAmount uses canonical positions directly', () => {
  const positions = [
    {
      contract: 'SPY250321C00100000',
      qty: -1,
      avg_cost: '0.00',
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
        quote: { bid: 5.2, ask: 5.3, mark: 5.25, last: 5.25 },
        greeks: null,
        implied_volatility: null,
        underlying_price: null,
      },
    },
    {
      contract: 'SPY250321P00095000',
      qty: -2,
      avg_cost: '0.00',
      leg_type: 'shortput',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: {
          underlying_symbol: 'SPY',
          expiration_date: '2025-03-21',
          strike: 95,
          option_right: 'put',
          occ_symbol: 'SPY250321P00095000',
        },
        quote: { bid: 0.55, ask: 0.65, mark: 0.6, last: 0.6 },
        greeks: null,
        implied_volatility: null,
        underlying_price: null,
      },
    },
    {
      contract: 'SPY250321C00110000',
      qty: 1,
      avg_cost: '0.00',
      leg_type: 'longcall',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: {
          underlying_symbol: 'SPY',
          expiration_date: '2025-03-21',
          strike: 110,
          option_right: 'call',
          occ_symbol: 'SPY250321C00110000',
        },
        quote: { bid: 0.2, ask: 0.3, mark: 0.25, last: 0.25 },
        greeks: null,
        implied_volatility: null,
        underlying_price: null,
      },
    },
  ];

  assert.equal(
    analysis.shortExtrinsicAmount('105', positions, '2'),
    290,
  );

  assert.equal(
    analysis.shortExtrinsicAmount(
      '105',
      [{ contract: 'bad-contract', qty: -1, avg_cost: '0.00', leg_type: 'shortcall', snapshot: sampleSnapshot(0.9, 1.1) }],
    ),
    null,
  );
});

test('analysis.shortItmPositions uses canonical positions directly', () => {
  const positions = [
    {
      contract: 'SPY250321C00100000',
      qty: -1,
      avg_cost: '0.00',
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
        quote: { bid: 5.2, ask: 5.3, mark: 5.25, last: 5.25 },
        greeks: null,
        implied_volatility: null,
        underlying_price: null,
      },
    },
    {
      contract: 'SPY250321P00110000',
      qty: -2,
      avg_cost: '0.00',
      leg_type: 'shortput',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: {
          underlying_symbol: 'SPY',
          expiration_date: '2025-03-21',
          strike: 110,
          option_right: 'put',
          occ_symbol: 'SPY250321P00110000',
        },
        quote: { bid: 0, ask: 0, mark: 0, last: 0 },
        greeks: null,
        implied_volatility: null,
        underlying_price: null,
      },
    },
    {
      contract: 'SPY250321C00110000',
      qty: 1,
      avg_cost: '0.00',
      leg_type: 'longcall',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: {
          underlying_symbol: 'SPY',
          expiration_date: '2025-03-21',
          strike: 110,
          option_right: 'call',
          occ_symbol: 'SPY250321C00110000',
        },
        quote: { bid: 0.2, ask: 0.3, mark: 0.25, last: 0.25 },
        greeks: null,
        implied_volatility: null,
        underlying_price: null,
      },
    },
    {
      contract: 'bad-contract',
      qty: -1,
      avg_cost: '0.00',
      leg_type: 'shortcall',
      snapshot: sampleSnapshot(0.9, 1.1),
    },
  ];

  assert.deepEqual(
    analysis.shortItmPositions('105', positions).map((item) => ({
      occSymbol: item.contract.occ_symbol,
      quantity: item.quantity,
      optionPrice: item.optionPrice,
      intrinsic: item.intrinsic,
      extrinsic: item.extrinsic,
    })),
    [
      {
        occSymbol: 'SPY250321C00100000',
        quantity: 1,
        optionPrice: 5.25,
        intrinsic: 5,
        extrinsic: 0.25,
      },
      {
        occSymbol: 'SPY250321P00110000',
        quantity: 2,
        optionPrice: 0,
        intrinsic: 5,
        extrinsic: 0,
      },
    ],
  );
});

test('pricing.contractExtrinsicValue parses raw OCC contracts directly', () => {
  assert.equal(
    pricing.contractExtrinsicValue('1.5', '590', 'SPY250321P00580000'),
    1.5,
  );
  assert.equal(
    pricing.contractExtrinsicValue('1.5', '590', 'bad-contract'),
    null,
  );
  assert.equal(
    pricing.contractExtrinsicValue('bad', '590', 'SPY250321P00580000'),
    null,
  );
});

test('pricing.extrinsicValue floors below-intrinsic quotes directly', () => {
  assert.equal(pricing.extrinsicValue(4.5, 110, 100, 'call'), 0);
  assert.equal(pricing.extrinsicValue(0.75, 95, 100, 'put'), 0);
  assert.equal(pricing.extrinsicValue(2.5, 103, 100, 'call'), 0);
});

test('executionQuote clamps progress and handles degenerate ranges', () => {
  assert.equal(executionQuote.limitQuoteByProgress({ bestPrice: 1.25, worstPrice: 1.45, progress: -1 }), 1.25);
  assert.equal(executionQuote.limitQuoteByProgress({ bestPrice: 1.25, worstPrice: 1.45, progress: 200 }), 1.45);
  assert.equal(executionQuote.limitQuoteByProgress({ bestPrice: 1.25, worstPrice: 1.45, progress: 40 }), 1.33);
  assert.equal(executionQuote.progressOfLimit({ bestPrice: 1.25, worstPrice: 1.45, limitPrice: 2 }), 1);
  assert.equal(executionQuote.progressOfLimit({ bestPrice: 1.25, worstPrice: 1.45, limitPrice: 1 }), 0);
  assert.equal(executionQuote.progressOfLimit({ bestPrice: 1.25, worstPrice: 1.25, limitPrice: 1.25 }), 0.5);

  const positions: OptionPosition[] = [
    {
      contract: 'SPY250321C00600000',
      qty: 2,
      avg_cost: '0.00',
      leg_type: 'longcall',
      snapshot: sampleSnapshot(1.1, 1.3),
    },
    {
      contract: 'SPY250321C00600000',
      qty: -1,
      avg_cost: '0.00',
      leg_type: 'shortcall',
      snapshot: sampleSnapshot(0.6, 0.8),
    },
  ];

  assert.deepEqual(
    executionQuote.bestWorst(positions),
    {
      structureQuantity: 1,
      perStructure: { bestPrice: 1.4, worstPrice: 2.0 },
      perOrder: { bestPrice: 1.4, worstPrice: 2.0 },
      dollars: { bestPrice: 140, worstPrice: 200 },
    },
  );
});

test('executionQuote.quote and limitPrice use canonical inputs directly', () => {
  assert.deepEqual(
    executionQuote.quote({
      bid: '1.10',
      ask: '1.30',
      price: '1.20',
    }),
    {
      bid: 1.1,
      ask: 1.3,
      mark: 1.2,
      last: 1.2,
    },
  );

  assert.deepEqual(
    executionQuote.quote({
      bid: '0.80',
      ask: null,
      mark: null,
    }),
    {
      bid: 0.8,
      ask: null,
      mark: 0.8,
      last: 0.8,
    },
  );

  assert.equal(
    executionQuote.quote({
      contract: 'SPY250321C00600000',
      qty: 1,
      avg_cost: '0.00',
      leg_type: 'longcall',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: sampleContract(),
        quote: {
          bid: 2.1,
          ask: 2.3,
          mark: 2.2,
          last: 2.2,
        },
        greeks: null,
        implied_volatility: null,
        underlying_price: null,
      },
    })?.mark,
    2.2,
  );

  assert.equal(
    executionQuote.quote({
      as_of: '2025-02-06 11:30:04',
      contract: sampleContract(),
      quote: {
        bid: 2.1,
        ask: 2.3,
        mark: 2.2,
        last: 2.2,
      },
      greeks: null,
      implied_volatility: null,
      underlying_price: null,
    })?.mark,
    2.2,
  );

  assert.equal(
    executionQuote.quote({
      contract: sampleContract(),
      orderSide: 'buy',
      ratioQuantity: 1,
      quote: {
        bid: 2.1,
        ask: 2.3,
        mark: 2.2,
        last: 2.2,
      },
      snapshot: null,
    })?.mark,
    2.2,
  );

  assert.equal(
    executionQuote.quote({
      snapshot: {
        contract: 'SPY250321C00600000',
        timestamp: '2025-02-06 11:30:04',
        bid: '2.10',
        ask: '2.30',
        price: '2.20',
        greeks: {
          delta: 0,
          gamma: 0,
          vega: 0,
          theta: 0,
          rho: 0,
        },
        iv: 0,
      },
    } as never)?.mark,
    2.2,
  );

  assert.equal(
    executionQuote.quote({
      position: {
        contract: 'SPY250321C00600000',
        qty: 1,
        avg_cost: '0.00',
        leg_type: 'longcall',
        snapshot: {
          as_of: '2025-02-06 11:30:04',
          contract: sampleContract(),
          quote: { bid: 2.1, ask: 2.3, mark: 2.2, last: 2.2 },
          greeks: null,
          implied_volatility: null,
          underlying_price: null,
        },
      },
    })?.mark,
    2.2,
  );

  assert.equal(executionQuote.limitPrice({ execution: { limit_price: '1.45' } }), 1.45);
  assert.equal(executionQuote.limitPrice({ execution: { limit_price: 'bad' } }), 0);
  assert.equal(executionQuote.limitPrice({ price: '2.05' }), 2.05);
});

test('executionQuote.rollRequest uses canonical snake_case inputs directly', () => {
  assert.deepEqual(
    executionQuote.rollRequest({
      current_contract: 'SPY250321P00580000',
      target_contract: 'SPY260417P00570000',
      leg_type: 'ShortPut',
      qty: '2',
    }),
    {
      current_contract: 'SPY250321P00580000',
      leg_type: 'shortput',
      qty: 2,
      new_strike: 570,
      new_expiration: '2026-04-17',
    },
  );
  assert.deepEqual(
    executionQuote.rollRequest({
      current_contract: 'SPY250321C00600000',
      new_strike: '605',
      new_expiration: '2026-04-25',
      qty: 0,
    }),
    {
      current_contract: 'SPY250321C00600000',
      qty: 1,
      new_strike: 605,
      new_expiration: '2026-04-25',
    },
  );
  assert.equal(
    executionQuote.rollRequest({
      current_contract: 'SPY250321C00600000',
      target_contract: 'bad-contract',
    }),
    null,
  );
  assert.equal(
    executionQuote.rollRequest({
      current_contract: '',
      new_strike: 605,
      new_expiration: '2026-04-25',
    }),
    null,
  );
});

test('executionQuote.legType resolves order legs directly', () => {
  assert.equal(
    executionQuote.legType({
      symbol: 'SPY250321P00580000',
      side: 'buy',
      position_intent: 'buy_to_close',
    }),
    'shortput',
  );
  assert.equal(
    executionQuote.legType({
      symbol: 'SPY250321C00600000',
      side: 'sell',
      position_intent: 'sell_to_open',
    }),
    'shortcall',
  );
  assert.equal(
    executionQuote.legType({
      symbol: 'SPY250321C00600000',
      side: 'buy',
      position_intent: 'buy_to_open',
      leg_type: 'LongCall',
    }),
    'longcall',
  );
  assert.equal(
    executionQuote.legType({
      symbol: 'bad-contract',
      side: 'buy',
      position_intent: 'buy_to_open',
    }),
    null,
  );
});

test('executionQuote computes canonical position and leg ranges directly', () => {
  const positions = [
    {
      contract: 'SPY250321C00600000',
      qty: 2,
      avg_cost: '1.25',
      leg_type: 'longcall',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: sampleContract(),
        quote: { bid: 1.1, ask: 1.3, mark: 1.2, last: 1.2 },
        greeks: { delta: 0.5, gamma: 0.02, vega: 0.1, theta: -0.03, rho: 0.02 },
        implied_volatility: 0.25,
        underlying_price: null,
      },
    },
    {
      contract: 'SPY250321C00600000',
      qty: -1,
      avg_cost: '0.80',
      leg_type: 'shortcall',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: sampleContract(),
        quote: { bid: 0.6, ask: 0.8, mark: 0.7, last: 0.7 },
        greeks: { delta: 0.2, gamma: 0.01, vega: 0.05, theta: -0.01, rho: 0.01 },
        implied_volatility: 0.22,
        underlying_price: null,
      },
    },
  ];

  assert.deepEqual(
    executionQuote.bestWorst(positions),
    {
      structureQuantity: 1,
      perStructure: { bestPrice: 1.4, worstPrice: 2.0 },
      perOrder: { bestPrice: 1.4, worstPrice: 2.0 },
      dollars: { bestPrice: 140, worstPrice: 200 },
    },
  );

  const legs = [
    {
      symbol: 'SPY250321P00580000',
      side: 'sell',
      ratio_qty: '2',
      position_intent: 'sell_to_open',
      leg_type: 'shortput',
      snapshot: {
        contract: 'SPY250321P00580000',
        timestamp: '2025-02-06 11:30:04',
        bid: '2.15',
        ask: '2.35',
        price: '2.25',
        greeks: { delta: -0.3, gamma: 0.02, vega: 0.09, theta: -0.03, rho: -0.01 },
        iv: 0.28,
      },
    },
    {
      symbol: 'SPY250321P00570000',
      side: 'buy',
      ratio_qty: '1',
      position_intent: 'buy_to_open',
      leg_type: 'longput',
      snapshot: {
        contract: 'SPY250321P00570000',
        timestamp: '2025-02-06 11:30:04',
        bid: '1.05',
        ask: '1.20',
        price: '1.12',
        greeks: { delta: -0.18, gamma: 0.01, vega: 0.06, theta: -0.02, rho: -0.01 },
        iv: 0.24,
      },
    },
  ];

  assert.deepEqual(
    executionQuote.bestWorst(legs, -2.9),
    {
      structureQuantity: 2,
      perStructure: { bestPrice: -3.65, worstPrice: -3.1 },
      perOrder: { bestPrice: -7.3, worstPrice: -6.2 },
      dollars: { bestPrice: -730, worstPrice: -620 },
    },
  );

  assert.deepEqual(
    executionQuote.scaleQuote({
      price: 1.237,
      structureQuantity: -3.8,
    }),
    {
      structureQuantity: 3,
      price: 1.24,
      totalPrice: 3.72,
      totalDollars: 372,
    },
  );
});

test('executionQuote.orderLegs and rollLegs build canonical execution legs', () => {
  const positions = [
    {
      contract: 'SPY250321C00600000',
      qty: 2,
      avg_cost: '0.00',
      leg_type: 'longcall',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: sampleContract(),
        quote: { bid: 1.1, ask: 1.3, mark: 1.2, last: 1.2 },
        greeks: { delta: 0.5, gamma: 0.02, vega: 0.1, theta: -0.03, rho: 0.02 },
        implied_volatility: 0.25,
        underlying_price: null,
      },
    },
    {
      contract: 'SPY250321P00580000',
      qty: -1,
      avg_cost: '0.00',
      leg_type: 'shortput',
      snapshot: {
        as_of: '2025-02-06 11:30:04',
        contract: {
          underlying_symbol: 'SPY',
          expiration_date: '2025-03-21',
          strike: 580,
          option_right: 'put',
          occ_symbol: 'SPY250321P00580000',
        },
        quote: { bid: 2.15, ask: 2.35, mark: 2.25, last: 2.25 },
        greeks: { delta: -0.3, gamma: 0.02, vega: 0.09, theta: -0.03, rho: -0.01 },
        implied_volatility: 0.28,
        underlying_price: null,
      },
    },
  ];

  assert.deepEqual(
    executionQuote.orderLegs({
      positions,
      action: 'close',
    }),
    [
      {
        symbol: 'SPY250321C00600000',
        ratio_qty: '2',
        side: 'sell',
        position_intent: 'sell_to_close',
        leg_type: 'longcall',
        snapshot: executionQuote.leg({
          action: 'close',
          legType: 'longcall',
          contract: 'SPY250321C00600000',
          quantity: 2,
          snapshot: positions[0].snapshot as never,
        })?.snapshot,
      },
      {
        symbol: 'SPY250321P00580000',
        ratio_qty: '1',
        side: 'buy',
        position_intent: 'buy_to_close',
        leg_type: 'shortput',
        snapshot: executionQuote.leg({
          action: 'close',
          legType: 'shortput',
          contract: 'SPY250321P00580000',
          quantity: 1,
          snapshot: positions[1].snapshot as never,
        })?.snapshot,
      },
    ],
  );

  assert.deepEqual(
    executionQuote.orderLegs({
      positions,
      action: 'open',
      excludeLegTypes: ['shortput'],
    }),
    [
      {
        symbol: 'SPY250321C00600000',
        ratio_qty: '2',
        side: 'buy',
        position_intent: 'buy_to_open',
        leg_type: 'longcall',
        snapshot: executionQuote.leg({
          action: 'open',
          legType: 'longcall',
          contract: 'SPY250321C00600000',
          quantity: 2,
          snapshot: positions[0].snapshot as never,
        })?.snapshot,
      },
    ],
  );

  assert.deepEqual(
    executionQuote.rollLegs({
      positions,
      snapshots: {
        shortput: {
          contract: 'SPY250328P00575000',
          timestamp: '2025-02-06 11:31:00',
          bid: '1.75',
          ask: '1.95',
          price: '1.85',
          greeks: { delta: -0.24, gamma: 0.02, vega: 0.08, theta: -0.02, rho: -0.01 },
          iv: 0.24,
        },
      },
      selections: [{ legType: 'shortput', quantity: 1 }],
    }),
    [
      {
        symbol: 'SPY250321P00580000',
        ratio_qty: '1',
        side: 'buy',
        position_intent: 'buy_to_close',
        leg_type: 'shortput',
        snapshot: executionQuote.leg({
          action: 'close',
          legType: 'shortput',
          contract: 'SPY250321P00580000',
          quantity: 1,
          snapshot: positions[1].snapshot as never,
        })?.snapshot,
      },
      {
        symbol: 'SPY250328P00575000',
        ratio_qty: '1',
        side: 'sell',
        position_intent: 'sell_to_open',
        leg_type: 'shortput',
        snapshot: {
          contract: 'SPY250328P00575000',
          timestamp: '2025-02-06 11:31:00',
          bid: '1.75',
          ask: '1.95',
          price: '1.85',
          greeks: { delta: -0.24, gamma: 0.02, vega: 0.08, theta: -0.02, rho: -0.01 },
          iv: 0.24,
        },
      },
    ],
  );
});

test('executionQuote.leg builds a single execution leg from direct quote inputs', () => {
  assert.deepEqual(
    executionQuote.leg({
      action: 'open',
      legType: 'longcall',
      contract: 'SPY250321C00600000',
      quantity: 1,
      timestamp: '2025-02-06 11:30:04',
      price: '1.20',
      spreadPercent: '0.10',
      iv: '0.25',
      greeks: {
        delta: 0.5,
        theta: -0.03,
      },
    }),
    {
      symbol: 'SPY250321C00600000',
      ratio_qty: '1',
      side: 'buy',
      position_intent: 'buy_to_open',
      leg_type: 'longcall',
      snapshot: {
        contract: 'SPY250321C00600000',
        timestamp: '2025-02-06 11:30:04',
        bid: '1.14',
        ask: '1.26',
        price: '1.2',
        iv: 0.25,
        greeks: {
          delta: 0.5,
          gamma: 0,
          vega: 0,
          theta: -0.03,
          rho: 0,
        },
      },
    },
  );

  assert.equal(
    executionQuote.leg({
      action: 'open',
      legType: 'longcall',
      contract: 'bad-contract',
      price: 1.2,
    }),
    null,
  );
  assert.equal(
    executionQuote.leg({
      action: 'open',
      legType: 'longcall',
      contract: 'SPY250321P00580000',
      bid: 1.1,
      ask: 1.3,
    }),
    null,
  );
});

test('payoff and probability boundary cases are explicit', () => {
  assert.equal(payoff.strategyPayoffAtExpiry({ legs: [], underlyingPriceAtExpiry: 100 }), 0);
  assert.deepEqual(payoff.breakEvenPoints({ legs: [] }), []);

  assert.equal(
    payoff.strategyPayoffAtExpiry({
      underlyingPriceAtExpiry: 90,
      legs: [{ optionRight: 'call', positionSide: 'short', strike: 100, premium: 2, quantity: 1 }],
    }),
    2,
  );

  assertOptionError(() => payoff.strategyPayoffAtExpiry({ legs: [], underlyingPriceAtExpiry: -1 }), 'invalid_payoff_input');
  assertOptionError(() => probability.expiryProbabilityInRange({
    spot: 100,
    lowerPrice: 105,
    upperPrice: 95,
    years: 0.1,
    rate: 0.045,
    dividendYield: 0,
    volatility: 0.2,
  }), 'invalid_probability_input');
});

test('numeric.brentSolve surfaces bracketing and convergence errors', () => {
  assertOptionError(() => numeric.brentSolve(1, 2, (x) => x * x + 1), 'root_not_bracketed');

  assertOptionError(() => numeric.brentSolve(0, 2, (x) => x * x - 2, 1e-20, 1), 'root_not_converged');
});

test('pricing.impliedVolatilityFromPrice uses discounted no-arbitrage lower bounds', () => {
  const impliedVolatility = pricing.impliedVolatilityFromPrice({
    targetPrice: 130.65325629560832,
    spot: 250,
    strike: 100,
    years: 10,
    rate: 0.03,
    dividendYield: 0.02,
    optionRight: 'call',
    lowerBound: 0.000001,
    upperBound: 5,
    tolerance: 1e-12,
  });

  assert.ok(Math.abs(impliedVolatility - 0.12) <= 1e-10);
});

test('advanced math kernels surface explicit boundary errors', () => {
  assertOptionError(() => mathAmerican.discreteDividendPrice({
    spot: 100,
    strike: 95,
    years: 1,
    rate: 0.03,
    volatility: 0.25,
    optionRight: 'call',
    cashDividendModel: 'spot',
    dividends: [],
  }), 'invalid_math_input');

  assertOptionError(() => mathBarrier.price({
    spot: 125,
    strike: 105,
    barrier: 125,
    rebate: 0,
    years: 0.5,
    rate: 0.01,
    dividendYield: 0.02,
    volatility: 0.3,
    optionRight: 'put',
    barrierType: 'up_out',
  }), 'invalid_math_input');

  assertOptionError(() => mathGeometricAsian.price({
    spot: 100,
    strike: 100,
    years: 1,
    rate: 0.03,
    dividendYield: 0.01,
    volatility: 0.25,
    optionRight: 'call',
    averageStyle: 'discrete' as 'continuous',
  }), 'unsupported_math_input');
});
