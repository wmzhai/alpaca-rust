import assert from 'node:assert/strict';
import test from 'node:test';

import * as alpacaOption from '../src/index.ts';

test('contract builders normalize display symbols and roundtrip OCC parsing', () => {
  const occSymbol = alpacaOption.contract.buildOccSymbol('BRK.B', '2025-01-17', 627.5, 'call');
  assert.equal(occSymbol, 'BRKB250117C00627500');
  assert.deepEqual(alpacaOption.contract.parseOccSymbol(occSymbol ?? ''), {
    underlying_symbol: 'BRKB',
    expiration_date: '2025-01-17',
    strike: 627.5,
    option_right: 'call',
    occ_symbol: 'BRKB250117C00627500',
  });
  assert.equal(alpacaOption.contract.buildOccSymbol('SPY', 'bad-date', 100, 'call'), null);
});

test('display and pricing helpers stay on top of canonical contract semantics', () => {
  const contract = alpacaOption.contract.parseOccSymbol('SPY250117P00580000');
  assert.ok(contract);
  assert.equal(alpacaOption.display.compactContract(contract, 'mm-dd'), '580P@01-17');
  assert.equal(alpacaOption.display.positionStrike({ contract }), '580');
  assert.equal(alpacaOption.pricing.contractExtrinsicValue(12, 570, contract), 2);
});

test('optionstrat url helpers build parse and merge canonical leg fragments', () => {
  const first = alpacaOption.url.buildOptionstratUrl({
    underlyingDisplaySymbol: 'BRK.B',
    legs: [
      {
        underlyingSymbol: 'BRK.B',
        expirationDate: '2025-01-17',
        strike: 627.5,
        optionRight: 'call',
        quantity: 1,
        premiumPerContract: 1.25,
      },
    ],
    stocks: [],
  });
  const second = alpacaOption.url.buildOptionstratUrl({
    underlyingDisplaySymbol: 'BRK.B',
    legs: [
      {
        occSymbol: 'BRKB250117P00600000',
        quantity: -2,
        premiumPerContract: 2.1,
      },
    ],
    stocks: [],
  });

  assert.ok(first);
  assert.ok(second);

  const merged = alpacaOption.url.mergeOptionstratUrls([first, second, 'not-a-url']);
  assert.ok(merged);

  const parsed = alpacaOption.url.parseOptionstratUrl(merged ?? '');
  assert.equal(parsed.underlyingDisplaySymbol, 'BRK.B');

  const legs = alpacaOption.url.parseOptionstratLegFragments(
    parsed.underlyingDisplaySymbol,
    parsed.legFragments,
  );
  assert.equal(legs.length, 2);
  assert.deepEqual(
    legs.map((leg) => ({
      occSymbol: leg.contract.occ_symbol,
      orderSide: leg.orderSide,
      ratioQuantity: leg.ratioQuantity,
      premiumPerContract: leg.premiumPerContract,
    })),
    [
      {
        occSymbol: 'BRKB250117C00627500',
        orderSide: 'buy',
        ratioQuantity: 1,
        premiumPerContract: 1.25,
      },
      {
        occSymbol: 'BRKB250117P00600000',
        orderSide: 'sell',
        ratioQuantity: 2,
        premiumPerContract: 2.1,
      },
    ],
  );
});

test('pricing surfaces the documented no-arbitrage validation error', () => {
  assert.throws(
    () =>
      alpacaOption.pricing.impliedVolatilityFromPrice({
        targetPrice: 0.01,
        spot: 100,
        strike: 50,
        years: 0.5,
        rate: 0.05,
        dividendYield: 0,
        optionRight: 'call',
      }),
    (error: unknown) =>
      error instanceof alpacaOption.OptionError && error.code === 'invalid_pricing_input',
  );
});
