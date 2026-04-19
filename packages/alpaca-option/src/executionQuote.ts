import { clock as timeClock } from '@alpaca/time';

import * as optionContract from './contract';
import { fail } from './error';
import { round } from './numeric';
import type {
  ExecutionAction,
  ExecutionLeg,
  ExecutionLegInput,
  ExecutionQuoteRange,
  ExecutionSnapshot,
  Greeks,
  GreeksInput,
  OptionPosition,
  OptionQuote,
  OptionSnapshot,
  PositionIntent,
  QuotedLeg,
  RollRequest,
  RollLegSelection,
  ScaledExecutionQuote,
  ScaledExecutionQuoteRange,
} from './types';

const CONTRACT_MULTIPLIER = 100;
type QuoteNumber = number | string | null | undefined;
type SnapshotInput = OptionSnapshot | ExecutionSnapshot | null | undefined;
type PositionInput = OptionPosition;
type LegInput = QuotedLeg | ExecutionLeg;

type RollRequestInput = {
  current_contract?: string | null;
  target_contract?: string | null;
  new_strike?: QuoteNumber;
  new_expiration?: string | null;
  leg_type?: string | null;
  qty?: QuoteNumber;
};

type DirectLegInput = ExecutionLegInput;

type QuoteEnvelopeInput =
  | { quote: OptionQuote | null | undefined }
  | { snapshot: SnapshotInput }
  | { position: PositionInput | null | undefined }
  | { leg: LegInput | null | undefined };

type QuoteInput = QuoteEnvelopeInput | OptionQuote | SnapshotInput | PositionInput | LegInput | null | undefined;

type LimitPriceInput =
  | { execution: { type?: string; limit_price?: QuoteNumber } | null | undefined }
  | { price: QuoteNumber };

type BestWorstEnvelopeInput =
  | { positions: PositionInput[]; structureQuantity?: number }
  | { legs: LegInput[]; structureQuantity?: number };

function ensureFinite(name: string, value: number): void {
  if (!Number.isFinite(value)) {
    fail('invalid_execution_quote_input', `${name} must be finite: ${value}`);
  }
}

function round2(value: number): number {
  return round(value, 2);
}

function coerceNumber(value: QuoteNumber): number | null {
  if (value == null) {
    return null;
  }

  const parsed = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function quoteValue(value: QuoteNumber, name: string): number {
  const parsed = coerceNumber(value);
  if (parsed == null) {
    return 0;
  }
  ensureFinite(name, parsed);
  return parsed;
}

function normalizeStructureQuantity(value: number | undefined): number {
  if (value == null || !Number.isFinite(value)) {
    return 0;
  }

  return Math.max(0, Math.abs(Math.trunc(value)));
}

function normalizeRatioQuantity(value: string | number | undefined): number {
  const parsed = typeof value === 'number' ? value : Number.parseInt(value ?? '', 10);
  if (!Number.isFinite(parsed)) {
    return 1;
  }

  return Math.max(1, Math.abs(Math.trunc(parsed)));
}

function normalizeExecutionLegType(value: string | null | undefined): string | null {
  const normalized = value?.trim().toLowerCase();
  return normalized === 'longcall'
    || normalized === 'shortcall'
    || normalized === 'longput'
    || normalized === 'shortput'
    ? normalized
    : null;
}

function normalizeExecutionSide(value: string | null | undefined): 'buy' | 'sell' | null {
  const normalized = value?.trim().toLowerCase();
  return normalized === 'buy' || normalized === 'sell' ? normalized : null;
}

function normalizePositionIntent(value: string | null | undefined): PositionIntent | null {
  const normalized = value?.trim().toLowerCase();
  return normalized === 'buy_to_open'
    || normalized === 'sell_to_open'
    || normalized === 'buy_to_close'
    || normalized === 'sell_to_close'
    ? normalized
    : null;
}

function normalizeQuote(source: OptionQuote | null | undefined): OptionQuote | null {
  if (!source) {
    return null;
  }

  const bid = coerceNumber(source.bid);
  const ask = coerceNumber(source.ask);
  const mark = coerceNumber(source.mark);
  const last = coerceNumber(source.last);
  const resolvedMark = mark != null
    ? mark
    : bid != null && ask != null
      ? round((bid + ask) / 2, 12)
      : bid ?? ask ?? last ?? null;

  return {
    bid,
    ask,
    mark: resolvedMark,
    last: last ?? resolvedMark,
  };
}

function executionGreeks(input: GreeksInput | Greeks | null | undefined): Greeks {
  return {
    delta: coerceNumber(input?.delta) ?? 0,
    gamma: coerceNumber(input?.gamma) ?? 0,
    vega: coerceNumber(input?.vega) ?? 0,
    theta: coerceNumber(input?.theta) ?? 0,
    rho: coerceNumber(input?.rho) ?? 0,
  };
}

function quoteFromSnapshot(input: SnapshotInput): OptionQuote | null {
  if (!input) {
    return null;
  }

  if ('as_of' in input) {
    return normalizeQuote(input.quote);
  }

  return normalizeQuote({
    bid: coerceNumber(input.bid),
    ask: coerceNumber(input.ask),
    mark: coerceNumber(input.price),
    last: coerceNumber(input.price),
  });
}

function isQuotedLeg(leg: LegInput): leg is QuotedLeg {
  return 'orderSide' in leg && 'ratioQuantity' in leg;
}

function isWrappedQuoteInput(input: unknown): input is { quote: OptionQuote | null | undefined } {
  return typeof input === 'object'
    && input != null
    && 'quote' in input
    && Object.keys(input).length === 1;
}

function isWrappedSnapshotInput(input: unknown): input is { snapshot: SnapshotInput } {
  return typeof input === 'object'
    && input != null
    && 'snapshot' in input
    && Object.keys(input).length === 1;
}

function isWrappedPositionInput(input: unknown): input is { position: PositionInput | null | undefined } {
  return typeof input === 'object'
    && input != null
    && 'position' in input
    && Object.keys(input).length === 1;
}

function isWrappedLegInput(input: unknown): input is { leg: LegInput | null | undefined } {
  return typeof input === 'object'
    && input != null
    && 'leg' in input
    && Object.keys(input).length === 1;
}

function looksLikePosition(input: unknown): input is PositionInput {
  return typeof input === 'object'
    && input != null
    && 'contract' in input
    && typeof (input as { contract?: unknown }).contract === 'string'
    && 'qty' in input
    && 'snapshot' in input;
}

function looksLikeLeg(input: unknown): input is LegInput {
  return typeof input === 'object'
    && input != null
    && (
      ('orderSide' in input && 'ratioQuantity' in input && 'quote' in input)
      || ('symbol' in input && 'ratio_qty' in input && 'position_intent' in input)
    );
}

function looksLikeSnapshot(input: unknown): input is SnapshotInput {
  return typeof input === 'object'
    && input != null
    && 'contract' in input
    && (('as_of' in input && 'quote' in input) || ('timestamp' in input && 'price' in input));
}

function quoteForPosition(position: PositionInput): OptionQuote | null {
  return quote(position.snapshot);
}

function quoteForLeg(leg: LegInput): OptionQuote | null {
  if (isQuotedLeg(leg)) {
    return normalizeQuote(leg.quote) ?? quote(leg.snapshot);
  }

  return quote(leg.snapshot);
}

function positionSide(position: PositionInput): 'long' | 'short' {
  if (position.qty < 0) {
    return 'short';
  }

  if (position.qty > 0) {
    return 'long';
  }

  return position.leg_type.trim().toLowerCase().startsWith('short') ? 'short' : 'long';
}

function positionQuantity(position: PositionInput): number {
  if (!Number.isFinite(position.qty)) {
    return 0;
  }

  return Math.abs(Math.trunc(position.qty));
}

function legSide(leg: LegInput): 'buy' | 'sell' {
  if (isQuotedLeg(leg)) {
    return leg.orderSide;
  }

  return leg.side;
}

function legQuantity(leg: LegInput): number {
  if (isQuotedLeg(leg)) {
    return leg.ratioQuantity;
  }

  return normalizeRatioQuantity(leg.ratio_qty);
}

function quoteBidAsk(quoteInput: OptionQuote): { bid: number; ask: number } {
  return {
    bid: quoteValue(quoteInput.bid, 'bid'),
    ask: quoteValue(quoteInput.ask, 'ask'),
  };
}

function clampProgress(progress: number): number {
  ensureFinite('progress', progress);
  const normalized = Math.abs(progress) > 1 ? progress / 100 : progress;
  return Math.min(1, Math.max(0, normalized));
}

function normalizeExecutionAction(action: string): ExecutionAction {
  if (action === 'open' || action === 'close') {
    return action;
  }

  fail('invalid_execution_quote_input', `invalid execution action: ${action}`);
}

function quoteNumberString(value: QuoteNumber, name: string): string {
  return quoteValue(value, name).toString();
}

function isExecutionSnapshot(input: SnapshotInput): input is ExecutionSnapshot {
  return Boolean(
    input
    && typeof input === 'object'
    && 'timestamp' in input
    && typeof input.timestamp === 'string'
    && 'price' in input
    && typeof input.price === 'string',
  );
}

function positionSymbol(position: PositionInput): string | null {
  return optionContract.isOccSymbol(position.contract) ? position.contract : null;
}

function inferredLegType(positionSideValue: 'long' | 'short', occSymbol: string): string | null {
  const parsed = optionContract.parseOccSymbol(occSymbol);
  if (!parsed) {
    return null;
  }

  return `${positionSideValue}${parsed.option_right}`;
}

function positionLegType(position: PositionInput, symbol: string | null = positionSymbol(position)): string | null {
  if (!symbol) {
    return null;
  }

  const explicit = normalizeExecutionLegType(position.leg_type);
  return explicit ?? inferredLegType(positionSide(position), symbol);
}

function executionSnapshot(
  input: SnapshotInput,
  fallbackSymbol?: string | null,
): ExecutionSnapshot | null {
  if (!input) {
    return null;
  }

  if (isExecutionSnapshot(input)) {
    return {
      contract: input.contract,
      timestamp: input.timestamp,
      bid: input.bid ?? '0',
      ask: input.ask ?? '0',
      price: input.price,
      greeks: executionGreeks(input.greeks),
      iv: coerceNumber(input.iv) ?? 0,
    };
  }

  const normalizedQuote = quote(input);
  const contract = optionContract.canonicalContract(input.contract);
  return {
    contract: contract?.occ_symbol ?? fallbackSymbol ?? '',
    timestamp: input.as_of,
    bid: quoteNumberString(normalizedQuote?.bid, 'bid'),
    ask: quoteNumberString(normalizedQuote?.ask, 'ask'),
    price: quoteNumberString(normalizedQuote?.mark ?? normalizedQuote?.last, 'price'),
    greeks: executionGreeks(input.greeks),
    iv: coerceNumber(input.implied_volatility) ?? 0,
  };
}

function orderSideForAction(positionSideValue: 'long' | 'short', action: ExecutionAction): 'buy' | 'sell' {
  if (action === 'open') {
    return positionSideValue === 'long' ? 'buy' : 'sell';
  }

  return positionSideValue === 'long' ? 'sell' : 'buy';
}

function positionIntent(side: 'buy' | 'sell', action: ExecutionAction): PositionIntent {
  if (action === 'open') {
    return side === 'buy' ? 'buy_to_open' : 'sell_to_open';
  }

  return side === 'buy' ? 'buy_to_close' : 'sell_to_close';
}

function normalizeLegTypeFilter(values: string[] | undefined): Set<string> {
  return new Set(
    (values ?? [])
      .map((value) => value.trim().toLowerCase())
      .filter((value) => value.length > 0),
  );
}

function executionLeg(
  symbol: string,
  leg_type: string,
  quantity: number,
  side: 'buy' | 'sell',
  action: ExecutionAction,
  snapshot: ExecutionSnapshot | null,
): ExecutionLeg {
  return {
    symbol,
    ratio_qty: Math.max(1, Math.abs(Math.trunc(quantity))).toString(),
    side,
    position_intent: positionIntent(side, action),
    leg_type,
    snapshot,
  };
}

function legSideFromType(legType: string, action: ExecutionAction): 'buy' | 'sell' | null {
  const normalized = normalizeExecutionLegType(legType);
  if (!normalized) {
    return null;
  }

  const isLong = normalized.startsWith('long');
  return action === 'open'
    ? (isLong ? 'buy' : 'sell')
    : (isLong ? 'sell' : 'buy');
}

function normalizeRollExpiration(value: string | null | undefined): string | null {
  const normalized = value?.trim() ?? '';
  if (!normalized) {
    return null;
  }

  try {
    return timeClock.parseDate(normalized);
  } catch {
    return null;
  }
}

function normalizeRollQuantity(value: QuoteNumber): number {
  const parsed = typeof value === 'number' ? value : Number.parseInt(value ?? '', 10);
  if (!Number.isFinite(parsed)) {
    return 1;
  }

  return Math.max(1, Math.abs(Math.trunc(parsed)));
}

function directLegQuantity(value: number | null | undefined): number {
  if (!Number.isFinite(value)) {
    return 1;
  }

  return Math.max(1, Math.abs(Math.trunc(value as number)));
}

function quoteFromDirectLeg(input: DirectLegInput): OptionQuote | null {
  const bid = coerceNumber(input.bid);
  const ask = coerceNumber(input.ask);
  const price = coerceNumber(input.price);
  if (bid == null && ask == null && price == null) {
    return null;
  }

  if (bid == null && ask == null && price != null) {
    const spreadPercent = coerceNumber(input.spreadPercent);
    if (spreadPercent != null && spreadPercent > 0) {
      const spread = Math.max(0, price * spreadPercent);
      return normalizeQuote({
        bid: price - spread / 2,
        ask: price + spread / 2,
        mark: price,
        last: price,
      });
    }

    return normalizeQuote({
      bid: price,
      ask: price,
      mark: price,
      last: price,
    });
  }

  return normalizeQuote({
    bid,
    ask,
    mark: price,
    last: price,
  });
}

function directExecutionSnapshot(input: DirectLegInput): ExecutionSnapshot | null {
  if (input.snapshot) {
    return executionSnapshot(input.snapshot, input.contract);
  }

  const normalizedQuote = quoteFromDirectLeg(input);
  if (!normalizedQuote) {
    return null;
  }

  return {
    contract: input.contract,
    timestamp: input.timestamp ?? '',
    bid: quoteNumberString(normalizedQuote.bid, 'bid'),
    ask: quoteNumberString(normalizedQuote.ask, 'ask'),
    price: quoteNumberString(normalizedQuote.mark ?? normalizedQuote.last, 'price'),
    greeks: executionGreeks(input.greeks),
    iv: coerceNumber(input.iv) ?? 0,
  };
}

function selectionLegType(selection: RollLegSelection): string | null {
  return normalizeExecutionLegType(selection.legType);
}

function selectionQuantity(selection: RollLegSelection, fallbackQuantity: number): number {
  const quantity = selection.quantity ?? 0;
  if (!Number.isFinite(quantity) || quantity <= 0) {
    return Math.max(1, fallbackQuantity);
  }

  return Math.min(Math.max(1, Math.abs(Math.trunc(quantity))), Math.max(1, fallbackQuantity));
}

export function legType(input: {
  symbol: string;
  side?: string | null;
  position_intent?: string | null;
  leg_type?: string | null;
}): string | null {
  const explicit = normalizeExecutionLegType(input.leg_type ?? '');
  if (explicit) {
    return explicit;
  }

  const side = normalizeExecutionSide(input.side);
  const positionIntentValue = normalizePositionIntent(input.position_intent);
  const parsed = optionContract.parseOccSymbol(input.symbol);
  if (!side || !positionIntentValue || !parsed) {
    return null;
  }

  const isClose = positionIntentValue.endsWith('_to_close');
  const isLong = side === 'buy' ? !isClose : isClose;
  return `${isLong ? 'long' : 'short'}${parsed.option_right}`;
}

export function rollRequest(input: RollRequestInput): RollRequest | null {
  const currentContract = input.current_contract?.trim() ?? '';
  if (!currentContract) {
    return null;
  }

  const rawLegType = input.leg_type?.trim() ?? '';
  const legType = rawLegType ? normalizeExecutionLegType(rawLegType) : null;
  if (rawLegType && !legType) {
    return null;
  }

  let newStrike: number | null = null;
  let newExpiration: string | null = null;
  const targetContract = input.target_contract?.trim() ?? '';
  if (targetContract) {
    const parsed = optionContract.parseOccSymbol(targetContract);
    if (!parsed) {
      return null;
    }
    newStrike = parsed.strike;
    newExpiration = parsed.expiration_date;
  } else {
    newStrike = coerceNumber(input.new_strike);
    newExpiration = normalizeRollExpiration(input.new_expiration);
    if (newStrike == null || newExpiration == null) {
      return null;
    }
  }

  return {
    current_contract: currentContract,
    ...(legType ? { leg_type: legType } : {}),
    qty: normalizeRollQuantity(input.qty),
    new_strike: newStrike,
    new_expiration: newExpiration,
  };
}

export function quote(input: QuoteInput): OptionQuote | null {
  if (!input) {
    return null;
  }

  if (isWrappedQuoteInput(input)) {
    return normalizeQuote(input.quote);
  }

  if (isWrappedSnapshotInput(input)) {
    return quoteFromSnapshot(input.snapshot);
  }

  if (isWrappedPositionInput(input)) {
    return input.position ? quoteForPosition(input.position) : null;
  }

  if (isWrappedLegInput(input)) {
    return input.leg ? quoteForLeg(input.leg) : null;
  }

  if (looksLikeLeg(input)) {
    return quoteForLeg(input);
  }

  if (looksLikePosition(input)) {
    return quoteForPosition(input);
  }

  if (looksLikeSnapshot(input)) {
    return quoteFromSnapshot(input);
  }

  return normalizeQuote(input);
}

export function leg(input: DirectLegInput): ExecutionLeg | null {
  const action = normalizeExecutionAction(input.action);
  const legType = normalizeExecutionLegType(input.legType);
  const contractInfo = optionContract.parseOccSymbol(input.contract);
  if (!legType || !contractInfo || !legType.endsWith(contractInfo.option_right)) {
    return null;
  }

  const side = legSideFromType(legType, action);
  if (!side) {
    return null;
  }

  return executionLeg(
    input.contract,
    legType,
    directLegQuantity(input.quantity),
    side,
    action,
    directExecutionSnapshot(input),
  );
}

export function orderLegs(input: {
  positions: PositionInput[];
  action: ExecutionAction;
  includeLegTypes?: string[];
  excludeLegTypes?: string[];
}): ExecutionLeg[] {
  const action = normalizeExecutionAction(input.action);
  const includeLegTypes = normalizeLegTypeFilter(input.includeLegTypes);
  const excludeLegTypes = normalizeLegTypeFilter(input.excludeLegTypes);
  const legs: ExecutionLeg[] = [];

  for (const position of input.positions) {
    const symbol = positionSymbol(position);
    const legTypeValue = positionLegType(position, symbol);
    const quantity = positionQuantity(position);
    if (!symbol || !legTypeValue || quantity <= 0) {
      continue;
    }

    if (includeLegTypes.size > 0 && !includeLegTypes.has(legTypeValue)) {
      continue;
    }
    if (excludeLegTypes.has(legTypeValue)) {
      continue;
    }

    const side = orderSideForAction(positionSide(position), action);
    legs.push(executionLeg(
      symbol,
      legTypeValue,
      quantity,
      side,
      action,
      executionSnapshot(position.snapshot, symbol),
    ));
  }

  return legs;
}

export function rollLegs(input: {
  positions: PositionInput[];
  snapshots: Record<string, SnapshotInput>;
  selections: RollLegSelection[];
}): ExecutionLeg[] {
  const positionsByLegType = new Map<string, PositionInput>();
  for (const position of input.positions) {
    const symbol = positionSymbol(position);
    const legTypeValue = positionLegType(position, symbol);
    if (!legTypeValue) {
      continue;
    }
    positionsByLegType.set(legTypeValue, position);
  }

  const snapshotsByLegType = new Map<string, SnapshotInput>();
  for (const [legTypeValue, nextSnapshot] of Object.entries(input.snapshots)) {
    const normalized = normalizeExecutionLegType(legTypeValue);
    if (!normalized || nextSnapshot == null) {
      continue;
    }
    snapshotsByLegType.set(normalized, nextSnapshot);
  }

  const legs: ExecutionLeg[] = [];
  for (const selection of input.selections) {
    const legTypeValue = selectionLegType(selection);
    if (!legTypeValue) {
      continue;
    }

    const position = positionsByLegType.get(legTypeValue);
    const nextSnapshotInput = snapshotsByLegType.get(legTypeValue);
    const symbol = position ? positionSymbol(position) : null;
    if (!position || !nextSnapshotInput || !symbol) {
      continue;
    }

    const quantity = selectionQuantity(selection, positionQuantity(position));
    const nextSnapshot = executionSnapshot(nextSnapshotInput);
    if (!nextSnapshot) {
      continue;
    }

    legs.push(executionLeg(
      symbol,
      legTypeValue,
      quantity,
      orderSideForAction(positionSide(position), 'close'),
      'close',
      executionSnapshot(position.snapshot, symbol),
    ));
    legs.push(executionLeg(
      nextSnapshot.contract,
      legTypeValue,
      quantity,
      orderSideForAction(positionSide(position), 'open'),
      'open',
      nextSnapshot,
    ));
  }

  return legs;
}

export function limitPrice(input: LimitPriceInput): number {
  return 'execution' in input
    ? coerceNumber(input.execution?.limit_price) ?? 0
    : coerceNumber(input.price) ?? 0;
}

function positionsRange(positions: PositionInput[]): ExecutionQuoteRange {
  let best = 0;
  let worst = 0;

  for (const position of positions) {
    const normalizedQuote = quoteForPosition(position);
    if (!normalizedQuote) {
      continue;
    }

    const { bid, ask } = quoteBidAsk(normalizedQuote);
    const quantity = positionQuantity(position);
    if (positionSide(position) === 'long') {
      best += bid * quantity;
      worst += ask * quantity;
    } else {
      best -= ask * quantity;
      worst -= bid * quantity;
    }
  }

  return {
    bestPrice: round2(best),
    worstPrice: round2(worst),
  };
}

function legsRange(legs: LegInput[]): ExecutionQuoteRange | null {
  let best = 0;
  let worst = 0;

  for (const legInput of legs) {
    const normalizedQuote = quoteForLeg(legInput);
    if (!normalizedQuote) {
      return null;
    }

    const { bid, ask } = quoteBidAsk(normalizedQuote);
    const quantity = legQuantity(legInput);
    if (legSide(legInput) === 'buy') {
      best += bid * quantity;
      worst += ask * quantity;
    } else {
      best -= ask * quantity;
      worst -= bid * quantity;
    }
  }

  return {
    bestPrice: round2(best),
    worstPrice: round2(worst),
  };
}

export function bestWorst(
  input: BestWorstEnvelopeInput | PositionInput[] | LegInput[],
  structureQuantity?: number,
): ScaledExecutionQuoteRange | null {
  const perStructure = (() => {
    if (Array.isArray(input)) {
      let first: PositionInput | LegInput | null = null;
      for (const value of input) {
        if (value != null) {
          first = value;
          break;
        }
      }

      if (!first) {
        return positionsRange(input as PositionInput[]);
      }

      return looksLikeLeg(first)
        ? legsRange(input as LegInput[])
        : positionsRange(input as PositionInput[]);
    }

    return 'positions' in input
      ? positionsRange(input.positions)
      : legsRange(input.legs);
  })();

  if (!perStructure) {
    return null;
  }

  return scaleQuoteRange({
    bestPrice: perStructure.bestPrice,
    worstPrice: perStructure.worstPrice,
    structureQuantity: Array.isArray(input)
      ? structureQuantity ?? 1
      : input.structureQuantity ?? 1,
  });
}

export function scaleQuote(input: {
  price: number;
  structureQuantity: number;
}): ScaledExecutionQuote {
  ensureFinite('price', input.price);
  const structureQuantity = normalizeStructureQuantity(input.structureQuantity);

  const price = round2(input.price);
  const totalPrice = round2(price * structureQuantity);
  return {
    structureQuantity,
    price,
    totalPrice,
    totalDollars: round2(totalPrice * CONTRACT_MULTIPLIER),
  };
}

export function scaleQuoteRange(input: {
  bestPrice: number;
  worstPrice: number;
  structureQuantity: number;
}): ScaledExecutionQuoteRange {
  ensureFinite('bestPrice', input.bestPrice);
  ensureFinite('worstPrice', input.worstPrice);
  const structureQuantity = normalizeStructureQuantity(input.structureQuantity);

  const perStructure = {
    bestPrice: round2(input.bestPrice),
    worstPrice: round2(input.worstPrice),
  };
  const perOrder = {
    bestPrice: round2(perStructure.bestPrice * structureQuantity),
    worstPrice: round2(perStructure.worstPrice * structureQuantity),
  };

  return {
    structureQuantity,
    perStructure,
    perOrder,
    dollars: {
      bestPrice: round2(perOrder.bestPrice * CONTRACT_MULTIPLIER),
      worstPrice: round2(perOrder.worstPrice * CONTRACT_MULTIPLIER),
    },
  };
}

export function limitQuoteByProgress(input: {
  bestPrice: number;
  worstPrice: number;
  progress: number;
}): number {
  ensureFinite('bestPrice', input.bestPrice);
  ensureFinite('worstPrice', input.worstPrice);
  const progress = clampProgress(input.progress);
  return round2(input.bestPrice + (input.worstPrice - input.bestPrice) * progress);
}

export function progressOfLimit(input: {
  bestPrice: number;
  worstPrice: number;
  limitPrice: number;
}): number {
  ensureFinite('bestPrice', input.bestPrice);
  ensureFinite('worstPrice', input.worstPrice);
  ensureFinite('limitPrice', input.limitPrice);
  if (Math.abs(input.worstPrice - input.bestPrice) < 1e-12) {
    return 0.5;
  }

  return round(
    Math.min(1, Math.max(0, (input.limitPrice - input.bestPrice) / (input.worstPrice - input.bestPrice))),
    12,
  );
}
