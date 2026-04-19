import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

import { analysis, contract, executionQuote, expirationSelection, numeric, payoff, pricing, probability, url } from '../src/index';
import * as mathAmerican from '../src/math/american';
import * as mathBachelier from '../src/math/bachelier';
import * as mathBarrier from '../src/math/barrier';
import * as mathBlack76 from '../src/math/black76';
import * as mathGeometricAsian from '../src/math/geometricAsian';
import type { Greeks, OptionContract, OptionPosition, OptionQuote, OptionSnapshot, QuotedLeg } from '../src/index';

type FixtureCase = {
  id: string;
  api: string;
  input: Record<string, unknown>;
  expected: Record<string, unknown>;
  tolerance?: number;
  field_tolerances?: Record<string, number>;
};

type FixtureCatalog = {
  support_paths?: string[];
  layers?: Array<{
    status?: string;
    fixture_paths?: string[];
  }>;
};

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '../../..');

async function loadFixture(relativePath: string): Promise<FixtureCase[]> {
  const content = await readFile(path.join(repoRoot, relativePath), 'utf8');
  return JSON.parse(content).cases as FixtureCase[];
}

async function loadFixturePaths(): Promise<string[]> {
  const content = await readFile(path.join(repoRoot, 'fixtures/catalog.json'), 'utf8');
  const catalog = JSON.parse(content) as FixtureCatalog;
  const supportPaths = catalog.support_paths ?? [];
  const layerPaths = (catalog.layers ?? [])
    .filter((layer) => layer.status === 'integrated')
    .flatMap((layer) => layer.fixture_paths ?? []);
  return [...supportPaths, ...layerPaths];
}

function unwrapExpected(expected: Record<string, unknown>): unknown {
  return Object.prototype.hasOwnProperty.call(expected, 'value') ? expected.value : expected;
}

function toSnakeCase(value: string): string {
  return value.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}

function comparable(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(comparable);
  }

  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([key, child]) => [toSnakeCase(key), comparable(child)]),
    );
  }

  return value;
}

function blackScholesPutCallParity(input: {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
}): { callMinusPut: number; discountedForwardMinusStrike: number } {
  const call = pricing.priceBlackScholes({ ...input, optionRight: 'call' });
  const put = pricing.priceBlackScholes({ ...input, optionRight: 'put' });
  return {
    callMinusPut: call - put,
    discountedForwardMinusStrike: input.spot * Math.exp(-input.dividendYield * input.years)
      - input.strike * Math.exp(-input.rate * input.years),
  };
}

function resolveTolerance(
  tolerance: number | undefined,
  fieldTolerances: Record<string, number> | undefined,
  path: string[],
): number | undefined {
  if (fieldTolerances == null) {
    return tolerance;
  }

  const fullPath = path.join('.');
  if (fullPath && fieldTolerances[fullPath] != null) {
    return fieldTolerances[fullPath];
  }

  const leaf = path[path.length - 1];
  if (leaf && fieldTolerances[leaf] != null) {
    return fieldTolerances[leaf];
  }

  return tolerance;
}

function assertWithTolerance(
  actual: unknown,
  expected: unknown,
  tolerance: number | undefined,
  caseId: string,
  fieldTolerances?: Record<string, number>,
  path: string[] = [],
): void {
  if (typeof actual === 'number' && typeof expected === 'number') {
    const effectiveTolerance = resolveTolerance(tolerance, fieldTolerances, path);
    if (effectiveTolerance != null) {
      assert.ok(
        Math.abs(actual - expected) <= effectiveTolerance,
        `${caseId}: expected ${expected}, got ${actual}, tolerance ${effectiveTolerance}`,
      );
      return;
    }
    assert.strictEqual(actual, expected, caseId);
    return;
  }

  if (Array.isArray(actual) && Array.isArray(expected)) {
    assert.strictEqual(actual.length, expected.length, caseId);
    for (let index = 0; index < actual.length; index += 1) {
      assertWithTolerance(actual[index], expected[index], tolerance, caseId, fieldTolerances, [...path, String(index)]);
    }
    return;
  }

  if (actual && expected && typeof actual === 'object' && typeof expected === 'object') {
    const actualEntries = actual as Record<string, unknown>;
    const expectedEntries = expected as Record<string, unknown>;
    assert.deepStrictEqual(Object.keys(actualEntries).sort(), Object.keys(expectedEntries).sort(), caseId);
    for (const key of Object.keys(expectedEntries)) {
      assertWithTolerance(actualEntries[key], expectedEntries[key], tolerance, caseId, fieldTolerances, [...path, key]);
    }
    return;
  }

  assert.deepStrictEqual(actual, expected, caseId);
}

function validateOptionSnapshotModel(input: Record<string, unknown>): boolean {
  return typeof input.as_of === 'string'
    && typeof input.contract === 'object'
    && typeof input.quote === 'object'
    && ('greeks' in input)
    && ('implied_volatility' in input)
    && ('underlying_price' in input);
}

function validateOptionPositionModel(input: Record<string, unknown>): boolean {
  return typeof input.contract === 'string'
    && Number.isInteger(input.qty)
    && typeof input.avg_cost === 'string'
    && typeof input.leg_type === 'string'
    && ('snapshot' in input);
}

function validateOptionChainRecordModel(input: Record<string, unknown>): boolean {
  return typeof input.as_of === 'string'
    && typeof input.underlying_symbol === 'string'
    && typeof input.occ_symbol === 'string'
    && typeof input.expiration_date === 'string'
    && (input.option_right === 'call' || input.option_right === 'put')
    && typeof input.strike === 'number';
}

function validateOptionChainModel(input: Record<string, unknown>): boolean {
  return typeof input.underlying_symbol === 'string'
    && typeof input.as_of === 'string'
    && Array.isArray(input.snapshots);
}

function toOptionQuote(input: Record<string, unknown>): OptionQuote {
  return {
    bid: input.bid == null ? null : Number(input.bid),
    ask: input.ask == null ? null : Number(input.ask),
    mark: input.mark == null ? null : Number(input.mark),
    last: input.last == null ? null : Number(input.last),
  };
}

function toOptionContract(input: Record<string, unknown>): OptionContract {
  return {
    underlying_symbol: String(input.underlying_symbol),
    expiration_date: String(input.expiration_date),
    strike: Number(input.strike),
    option_right: input.option_right as OptionContract['option_right'],
    occ_symbol: String(input.occ_symbol),
  };
}

function toGreeks(input: Record<string, unknown>): Greeks {
  return {
    delta: Number(input.delta),
    gamma: Number(input.gamma),
    vega: Number(input.vega),
    theta: Number(input.theta),
    rho: Number(input.rho),
  };
}

function toOptionSnapshot(input: Record<string, unknown>): OptionSnapshot {
  return {
    as_of: String(input.as_of),
    contract: toOptionContract(input.contract as Record<string, unknown>),
    quote: toOptionQuote(input.quote as Record<string, unknown>),
    greeks: input.greeks == null ? null : toGreeks(input.greeks as Record<string, unknown>),
    implied_volatility: input.implied_volatility == null ? null : Number(input.implied_volatility),
    underlying_price: input.underlying_price == null ? null : Number(input.underlying_price),
  };
}

function emptyOptionSnapshot(contract: OptionContract): OptionSnapshot {
  return {
    as_of: '',
    contract,
    quote: {
      bid: null,
      ask: null,
      mark: null,
      last: null,
    },
    greeks: null,
    implied_volatility: null,
    underlying_price: null,
  };
}

function toOptionPosition(input: Record<string, unknown>): OptionPosition {
  const contractInput = typeof input.contract === 'object' && input.contract != null
    ? toOptionContract(input.contract as Record<string, unknown>)
    : null;
  const contract = typeof input.contract === 'string'
    ? input.contract
    : String(contractInput?.occ_symbol ?? '');
  const rawQty = input.qty == null
    ? Number(input.quantity)
    : Number(input.qty);
  const positionSide = input.position_side == null ? null : String(input.position_side);
  const qty = Number.isFinite(rawQty)
    ? (
      positionSide === 'short'
        ? -Math.abs(Math.trunc(rawQty))
        : positionSide === 'long'
          ? Math.abs(Math.trunc(rawQty))
          : Math.trunc(rawQty)
    )
    : 0;
  const avgCost = input.avg_cost == null
    ? (input.avg_entry_price == null ? '0.00' : Number(input.avg_entry_price).toFixed(2))
    : String(input.avg_cost);
  const legType = (() => {
    if (typeof input.leg_type === 'string') {
      return input.leg_type;
    }

    const optionRight = typeof input.contract === 'object'
      ? String((input.contract as Record<string, unknown>).option_right)
      : '';
    const side = positionSide === 'short' || qty < 0 ? 'short' : 'long';
    return `${side}${optionRight}`;
  })();

  return {
    contract,
    qty,
    avg_cost: avgCost,
    leg_type: legType,
    snapshot: input.snapshot == null
      ? emptyOptionSnapshot(contractInput ?? toOptionContract({
        underlying_symbol: '',
        expiration_date: '',
        strike: 0,
        option_right: 'call',
        occ_symbol: contract,
      }))
      : toOptionSnapshot(input.snapshot as Record<string, unknown>),
  };
}

function toQuotedLeg(input: Record<string, unknown>): QuotedLeg {
  return {
    contract: toOptionContract(input.contract as Record<string, unknown>),
    orderSide: input.order_side as QuotedLeg['orderSide'],
    ratioQuantity: Number(input.ratio_quantity),
    quote: toOptionQuote(input.quote as Record<string, unknown>),
    snapshot: input.snapshot == null ? null : toOptionSnapshot(input.snapshot as Record<string, unknown>),
  };
}

function runCase(item: FixtureCase): unknown {
  switch (item.api) {
    case 'model.option_snapshot':
      return { valid: validateOptionSnapshotModel(item.input) };
    case 'model.option_position':
      return { valid: validateOptionPositionModel(item.input) };
    case 'model.option_chain_record':
      return { valid: validateOptionChainRecordModel(item.input) };
    case 'model.option_chain':
      return { valid: validateOptionChainModel(item.input) };
    case 'contract.normalize_underlying_symbol':
      return contract.normalizeUnderlyingSymbol(String(item.input.symbol));
    case 'contract.is_occ_symbol':
      return contract.isOccSymbol(String(item.input.occ_symbol));
    case 'contract.parse_occ_symbol':
      return contract.parseOccSymbol(String(item.input.occ_symbol));
    case 'contract.build_occ_symbol':
      return contract.buildOccSymbol(
        String(item.input.underlying_symbol),
        String(item.input.expiration_date),
        item.input.strike as string | number,
        String(item.input.option_right),
      );
    case 'url.to_optionstrat_underlying_path':
      return url.toOptionstratUnderlyingPath(String(item.input.symbol));
    case 'url.from_optionstrat_underlying_path':
      return url.fromOptionstratUnderlyingPath(String(item.input.path));
    case 'url.build_optionstrat_leg_fragment':
      return url.buildOptionstratLegFragment({
        occSymbol: String(item.input.occ_symbol),
        quantity: Number(item.input.quantity),
        premiumPerContract: item.input.premium_per_contract == null ? null : Number(item.input.premium_per_contract),
      });
    case 'url.build_optionstrat_stock_fragment':
      return url.buildOptionstratStockFragment({
        underlyingSymbol: String(item.input.underlying_symbol),
        quantity: Number(item.input.quantity),
        costPerShare: Number(item.input.cost_per_share),
      });
    case 'url.build_optionstrat_url':
      return url.buildOptionstratUrl({
        underlyingDisplaySymbol: String(item.input.underlying_display_symbol),
        legs: ((item.input.legs as Array<Record<string, unknown>> | undefined) ?? []).map((leg) => ({
          occSymbol: String(leg.occ_symbol),
          quantity: Number(leg.quantity),
          premiumPerContract: leg.premium_per_contract == null ? null : Number(leg.premium_per_contract),
        })),
        stocks: ((item.input.stocks as Array<Record<string, unknown>> | undefined) ?? []).map((stock) => ({
          underlyingSymbol: String(stock.underlying_symbol),
          quantity: Number(stock.quantity),
          costPerShare: Number(stock.cost_per_share),
        })),
      });
    case 'url.parse_optionstrat_url':
      return url.parseOptionstratUrl(String(item.input.url));
    case 'url.parse_optionstrat_leg_fragments':
      return url.parseOptionstratLegFragments(
        String(item.input.underlying_display_symbol),
        (item.input.leg_fragments as Array<unknown>).map(String),
      );
    case 'pricing.price_black_scholes':
      return pricing.priceBlackScholes({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'pricing.greeks_black_scholes':
      return pricing.greeksBlackScholes({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'pricing.intrinsic_value':
      return pricing.intrinsicValue(
        Number(item.input.spot),
        Number(item.input.strike),
        item.input.option_right as 'call' | 'put',
      );
    case 'pricing.extrinsic_value':
      return pricing.extrinsicValue(
        Number(item.input.option_price),
        Number(item.input.spot),
        Number(item.input.strike),
        item.input.option_right as 'call' | 'put',
      );
    case 'pricing.implied_volatility_from_price':
      return pricing.impliedVolatilityFromPrice({
        targetPrice: Number(item.input.target_price),
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        optionRight: item.input.option_right as 'call' | 'put',
        lowerBound: Number(item.input.lower_bound),
        upperBound: Number(item.input.upper_bound),
        tolerance: Number(item.input.tolerance),
        maxIterations: item.input.max_iterations == null ? undefined : Number(item.input.max_iterations),
      });
    case 'pricing.black_scholes_put_call_parity':
      return blackScholesPutCallParity({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
      });
    case 'math.black76_price':
      return mathBlack76.price({
        forward: Number(item.input.forward),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'math.black76_greeks':
      return mathBlack76.greeks({
        forward: Number(item.input.forward),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'math.black76_implied_volatility_from_price':
      return mathBlack76.impliedVolatilityFromPrice({
        targetPrice: Number(item.input.target_price),
        forward: Number(item.input.forward),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        optionRight: item.input.option_right as 'call' | 'put',
        lowerBound: Number(item.input.lower_bound),
        upperBound: Number(item.input.upper_bound),
        tolerance: Number(item.input.tolerance),
        maxIterations: item.input.max_iterations == null ? undefined : Number(item.input.max_iterations),
      });
    case 'math.bachelier_price':
      return mathBachelier.price({
        forward: Number(item.input.forward),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        normalVolatility: Number(item.input.normal_volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'math.bachelier_greeks':
      return mathBachelier.greeks({
        forward: Number(item.input.forward),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        normalVolatility: Number(item.input.normal_volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'math.bachelier_implied_volatility_from_price':
      return mathBachelier.impliedVolatilityFromPrice({
        targetPrice: Number(item.input.target_price),
        forward: Number(item.input.forward),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        optionRight: item.input.option_right as 'call' | 'put',
        lowerBound: Number(item.input.lower_bound),
        upperBound: Number(item.input.upper_bound),
        tolerance: Number(item.input.tolerance),
        maxIterations: item.input.max_iterations == null ? undefined : Number(item.input.max_iterations),
      });
    case 'math.american_tree_price':
      return mathAmerican.treePrice({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
        steps: item.input.steps == null ? undefined : Number(item.input.steps),
        useRichardson: item.input.use_richardson == null ? undefined : Boolean(item.input.use_richardson),
      });
    case 'math.american_barone_adesi_whaley_price':
      return mathAmerican.baroneAdesiWhaleyPrice({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'math.american_bjerksund_stensland_1993_price':
      return mathAmerican.bjerksundStensland1993Price({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'math.american_ju_quadratic_price':
      return mathAmerican.juQuadraticPrice({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
      });
    case 'math.american.discrete_dividend_price':
      return mathAmerican.discreteDividendPrice({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
        cashDividendModel: item.input.cash_dividend_model as 'spot' | 'escrowed',
        dividends: (item.input.dividends as Array<Record<string, unknown>>).map((dividend) => ({
          time: Number(dividend.time),
          amount: Number(dividend.amount),
        })),
      });
    case 'math.barrier.price':
      return mathBarrier.price({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        barrier: Number(item.input.barrier),
        rebate: Number(item.input.rebate),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
        barrierType: item.input.barrier_type as 'down_in' | 'down_out' | 'up_in' | 'up_out',
      });
    case 'math.geometric_asian.price':
      return mathGeometricAsian.price({
        spot: Number(item.input.spot),
        strike: Number(item.input.strike),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
        optionRight: item.input.option_right as 'call' | 'put',
        averageStyle: item.input.average_style as 'continuous',
      });
    case 'probability.expiry_probability_in_range':
      return probability.expiryProbabilityInRange({
        spot: Number(item.input.spot),
        lowerPrice: Number(item.input.lower_price),
        upperPrice: Number(item.input.upper_price),
        years: Number(item.input.years),
        rate: Number(item.input.rate),
        dividendYield: Number(item.input.dividend_yield),
        volatility: Number(item.input.volatility),
      });
    case 'analysis.annualized_premium_yield':
      return analysis.annualizedPremiumYield(
        Number(item.input.premium),
        Number(item.input.capital_base),
        Number(item.input.years),
      );
    case 'analysis.calendar_forward_factor':
      return analysis.calendarForwardFactor(
        Number(item.input.short_iv),
        Number(item.input.long_iv),
        Number(item.input.short_years),
        Number(item.input.long_years),
      );
    case 'analysis.moneyness_ratio':
      return analysis.moneynessRatio(Number(item.input.spot), Number(item.input.strike));
    case 'analysis.moneyness_label':
      return analysis.moneynessLabel(
        Number(item.input.spot),
        Number(item.input.strike),
        item.input.option_right as 'call' | 'put',
        item.input.atm_band == null ? undefined : Number(item.input.atm_band),
      );
    case 'analysis.otm_percent':
      return analysis.otmPercent(
        Number(item.input.spot),
        Number(item.input.strike),
        item.input.option_right as 'call' | 'put',
      );
    case 'analysis.assignment_risk':
      return analysis.assignmentRisk(Number(item.input.extrinsic));
    case 'analysis.short_extrinsic_amount':
      return analysis.shortExtrinsicAmount(
        Number(item.input.spot),
        (item.input.positions as Array<Record<string, unknown>>).map(toOptionPosition),
        item.input.structure_quantity == null ? undefined : Number(item.input.structure_quantity),
      );
    case 'analysis.short_itm_positions':
      return analysis.shortItmPositions(
        Number(item.input.spot),
        (item.input.positions as Array<Record<string, unknown>>).map(toOptionPosition),
      );
    case 'analysis.strike_for_target_delta':
      return analysis.strikeForTargetDelta(
        Number(item.input.spot),
        Number(item.input.years),
        Number(item.input.rate),
        Number(item.input.dividend_yield),
        Number(item.input.volatility),
        Number(item.input.target_delta),
        item.input.option_right as 'call' | 'put',
        Number(item.input.strike_step),
      );
    case 'execution_quote.best_worst':
      if (Array.isArray(item.input.positions)) {
        return executionQuote.bestWorst({
          positions: item.input.positions.map((position) => toOptionPosition(position as Record<string, unknown>)),
          structureQuantity: item.input.structure_quantity == null ? undefined : Number(item.input.structure_quantity),
        });
      }
      return executionQuote.bestWorst({
        legs: (item.input.legs as Array<Record<string, unknown>>).map(toQuotedLeg),
        structureQuantity: item.input.structure_quantity == null ? undefined : Number(item.input.structure_quantity),
      });
    case 'execution_quote.quote':
      if (item.input.snapshot) {
        return executionQuote.quote({
          snapshot: toOptionSnapshot(item.input.snapshot as Record<string, unknown>),
        });
      }
      if (item.input.position) {
        return executionQuote.quote({
          position: toOptionPosition(item.input.position as Record<string, unknown>),
        });
      }
      if (item.input.leg) {
        return executionQuote.quote({
          leg: toQuotedLeg(item.input.leg as Record<string, unknown>),
        });
      }
      return executionQuote.quote({
        quote: toOptionQuote(item.input.quote as Record<string, unknown>),
      });
    case 'execution_quote.limit_price':
      return executionQuote.limitPrice({
        price: item.input.price == null ? null : Number(item.input.price),
      });
    case 'execution_quote.order_legs':
      return executionQuote.orderLegs({
        positions: (item.input.positions as Array<Record<string, unknown>>).map(toOptionPosition),
        action: String(item.input.action) as 'open' | 'close',
        includeLegTypes: (item.input.include_leg_types as Array<unknown> | undefined)?.map(String),
        excludeLegTypes: (item.input.exclude_leg_types as Array<unknown> | undefined)?.map(String),
      });
    case 'execution_quote.leg':
      return executionQuote.leg({
        action: String(item.input.action) as 'open' | 'close',
        legType: String(item.input.leg_type),
        contract: String(item.input.contract),
        quantity: item.input.quantity == null ? undefined : Number(item.input.quantity),
        snapshot: item.input.snapshot == null ? undefined : item.input.snapshot as Record<string, unknown>,
        timestamp: item.input.timestamp == null ? undefined : String(item.input.timestamp),
        bid: item.input.bid == null ? undefined : Number(item.input.bid),
        ask: item.input.ask == null ? undefined : Number(item.input.ask),
        price: item.input.price == null ? undefined : Number(item.input.price),
        spreadPercent: item.input.spread_percent == null ? undefined : Number(item.input.spread_percent),
        greeks: item.input.greeks == null ? undefined : item.input.greeks as Record<string, unknown>,
        iv: item.input.iv == null ? undefined : Number(item.input.iv),
      });
    case 'execution_quote.roll_legs':
      return executionQuote.rollLegs({
        positions: (item.input.positions as Array<Record<string, unknown>>).map(toOptionPosition),
        snapshots: Object.fromEntries(
          Object.entries(item.input.snapshots as Record<string, unknown>).map(([legType, snapshot]) => [
            legType,
            snapshot as Record<string, unknown>,
          ]),
        ),
        selections: (item.input.selections as Array<Record<string, unknown>>).map((selection) => ({
          legType: String(selection.leg_type ?? selection.legType),
          quantity: selection.quantity == null ? undefined : Number(selection.quantity),
        })),
      });
    case 'execution_quote.scale_quote':
      return executionQuote.scaleQuote({
        price: Number(item.input.price),
        structureQuantity: Number(item.input.structure_quantity),
      });
    case 'execution_quote.scale_quote_range':
      return executionQuote.scaleQuoteRange({
        bestPrice: Number(item.input.best_price),
        worstPrice: Number(item.input.worst_price),
        structureQuantity: Number(item.input.structure_quantity),
      });
    case 'execution_quote.limit_quote_by_progress':
      return executionQuote.limitQuoteByProgress({
        bestPrice: Number(item.input.best_price),
        worstPrice: Number(item.input.worst_price),
        progress: Number(item.input.progress),
      });
    case 'execution_quote.progress_of_limit':
      return executionQuote.progressOfLimit({
        bestPrice: Number(item.input.best_price),
        worstPrice: Number(item.input.worst_price),
        limitPrice: Number(item.input.limit_price),
      });
    case 'payoff.single_leg_payoff_at_expiry':
      return payoff.singleLegPayoffAtExpiry({
        optionRight: item.input.option_right as 'call' | 'put',
        positionSide: item.input.position_side as 'long' | 'short',
        strike: Number(item.input.strike),
        premium: Number(item.input.premium),
        quantity: Number(item.input.quantity),
        underlyingPriceAtExpiry: Number(item.input.underlying_price_at_expiry),
      });
    case 'payoff.strategy_payoff_at_expiry':
      return payoff.strategyPayoffAtExpiry({
        legs: (item.input.legs as Array<Record<string, unknown>>).map((leg) => ({
          optionRight: leg.option_right as 'call' | 'put',
          positionSide: leg.position_side as 'long' | 'short',
          strike: Number(leg.strike),
          premium: Number(leg.premium),
          quantity: Number(leg.quantity),
        })),
        underlyingPriceAtExpiry: Number(item.input.underlying_price_at_expiry),
      });
    case 'payoff.break_even_points':
      return payoff.breakEvenPoints({
        legs: (item.input.legs as Array<Record<string, unknown>>).map((leg) => ({
          optionRight: leg.option_right as 'call' | 'put',
          positionSide: leg.position_side as 'long' | 'short',
          strike: Number(leg.strike),
          premium: Number(leg.premium),
          quantity: Number(leg.quantity),
        })),
      });
    case 'payoff.strategy_pnl':
      return payoff.strategyPnl({
        positions: (item.input.positions as Array<Record<string, unknown>>).map((position) => ({
          contract: {
            underlying_symbol: String((position.contract as Record<string, unknown>).underlying_symbol),
            expiration_date: String((position.contract as Record<string, unknown>).expiration_date),
            strike: Number((position.contract as Record<string, unknown>).strike),
            option_right: String((position.contract as Record<string, unknown>).option_right) as 'call' | 'put',
            occ_symbol: String((position.contract as Record<string, unknown>).occ_symbol),
          },
          quantity: Number(position.quantity),
          avg_entry_price: position.avg_entry_price == null ? null : Number(position.avg_entry_price),
          implied_volatility: position.implied_volatility == null ? null : Number(position.implied_volatility),
        })),
        underlying_price: Number(item.input.underlying_price),
        evaluation_time: String(item.input.evaluation_time),
        entry_cost: item.input.entry_cost == null ? null : Number(item.input.entry_cost),
        rate: Number(item.input.rate),
        dividend_yield: item.input.dividend_yield == null ? null : Number(item.input.dividend_yield),
        long_volatility_shift: item.input.long_volatility_shift == null ? null : Number(item.input.long_volatility_shift),
      });
    case 'payoff.strategy_break_even_points':
      return payoff.strategyBreakEvenPoints({
        positions: (item.input.positions as Array<Record<string, unknown>>).map((position) => ({
          contract: {
            underlying_symbol: String((position.contract as Record<string, unknown>).underlying_symbol),
            expiration_date: String((position.contract as Record<string, unknown>).expiration_date),
            strike: Number((position.contract as Record<string, unknown>).strike),
            option_right: String((position.contract as Record<string, unknown>).option_right) as 'call' | 'put',
            occ_symbol: String((position.contract as Record<string, unknown>).occ_symbol),
          },
          quantity: Number(position.quantity),
          avg_entry_price: position.avg_entry_price == null ? null : Number(position.avg_entry_price),
          implied_volatility: position.implied_volatility == null ? null : Number(position.implied_volatility),
        })),
        evaluation_time: String(item.input.evaluation_time),
        entry_cost: item.input.entry_cost == null ? null : Number(item.input.entry_cost),
        rate: Number(item.input.rate),
        dividend_yield: item.input.dividend_yield == null ? null : Number(item.input.dividend_yield),
        long_volatility_shift: item.input.long_volatility_shift == null ? null : Number(item.input.long_volatility_shift),
        lower_bound: Number(item.input.lower_bound),
        upper_bound: Number(item.input.upper_bound),
        scan_step: item.input.scan_step == null ? undefined : Number(item.input.scan_step),
        tolerance: item.input.tolerance == null ? undefined : Number(item.input.tolerance),
        maxIterations: item.input.max_iterations == null ? undefined : Number(item.input.max_iterations),
      });
    case 'expiration_selection.nearest_weekly_expiration':
      return expirationSelection.nearestWeeklyExpiration(String(item.input.anchor_date));
    case 'expiration_selection.weekly_expirations_between':
      return expirationSelection.weeklyExpirationsBetween(
        String(item.input.start_date),
        String(item.input.end_date),
      );
    case 'expiration_selection.standard_monthly_expiration':
      return expirationSelection.standardMonthlyExpiration(Number(item.input.year), Number(item.input.month));
    case 'numeric.normal_cdf':
      return numeric.normalCdf(Number(item.input.x));
    case 'numeric.normal_pdf':
      return numeric.normalPdf(Number(item.input.x));
    case 'numeric.round':
      return numeric.round(Number(item.input.value), Number(item.input.decimals));
    case 'numeric.linspace':
      return numeric.linspace(
        Number(item.input.start),
        Number(item.input.end),
        Number(item.input.count),
      );
    case 'numeric.brent_solve':
      return numeric.brentSolve(
        Number(item.input.lower_bound),
        Number(item.input.upper_bound),
        (() => {
          switch (item.input.evaluator) {
            case 'square_minus_two':
              return (value: number) => value * value - 2;
            default:
              throw new Error(`Unhandled numeric evaluator: ${String(item.input.evaluator)}`);
          }
        })(),
        Number(item.input.tolerance),
        Number(item.input.max_iterations),
      );
    default:
      throw new Error(`Unhandled fixture api: ${item.api}`);
  }
}

test('fixtures catalog', async (t) => {
  const fixturePaths = await loadFixturePaths();
  for (const fixturePath of fixturePaths) {
    await t.test(fixturePath, async () => {
      const cases = await loadFixture(fixturePath);
      for (const item of cases) {
        const actual = comparable(runCase(item));
        const expected = unwrapExpected(item.expected);
        assertWithTolerance(actual, expected, item.tolerance, `${fixturePath}::${item.id}`, item.field_tolerances);
      }
    });
  }
});
