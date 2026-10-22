import { expiration as timeExpiration } from '@alpaca/time';

import * as executionQuote from './executionQuote';
import { canonicalContract } from './contract';
import { round, roundHalfEven } from './numeric';
import type { ExecutionSnapshot, OptionContract, OptionPosition, OptionSnapshot } from './types';

export type SnapshotInput = OptionSnapshot | ExecutionSnapshot | null | undefined;

function asOf(snapshot: SnapshotInput): string {
  if (!snapshot || typeof snapshot !== 'object') {
    return '';
  }

  if ('as_of' in snapshot) {
    return snapshot.as_of.trim();
  }

  return snapshot.timestamp.trim();
}

export function quote(snapshot: SnapshotInput) {
  return executionQuote.quote(snapshot);
}

/** Two-sided mid. Missing, crossed, or non-positive ask quotes return null. */
export function twoSidedMid(snapshot: SnapshotInput): number | null {
  const normalized = quote(snapshot);
  const bid = normalized?.bid;
  const ask = normalized?.ask;
  if (bid == null || ask == null || !Number.isFinite(bid) || !Number.isFinite(ask)) {
    return null;
  }
  if (!(ask > 0 && ask >= bid)) {
    return null;
  }
  return (bid + ask) / 2;
}

/**
 * Option mark from two-sided mids. Each quote is rounded to cents, then multiplied
 * by the signed contract quantity and 100. Any leg without a two-sided mid returns null.
 */
export function quoteMidOptionValue(positions: readonly OptionPosition[], strategyQty: number): number | null {
  const quantity = Math.max(1, Math.trunc(strategyQty));
  let total = 0;
  for (const position of positions) {
    const mid = twoSidedMid(position.snapshot);
    if (mid == null) {
      return null;
    }
    total += roundHalfEven(mid, 2) * position.qty * 100;
  }
  return roundHalfEven(total * quantity, 2);
}

export function contract(snapshot: SnapshotInput): OptionContract | null {
  return canonicalContract(snapshot?.contract);
}

export function spread(snapshot: SnapshotInput): number {
  const normalized = quote(snapshot);
  return round((normalized?.ask ?? 0) - (normalized?.bid ?? 0), 12);
}

export function spreadPct(snapshot: SnapshotInput): number {
  const normalized = quote(snapshot);
  const price = normalized?.mark ?? 0;
  if (Math.abs(price) <= 1e-10) {
    return 0;
  }

  return spread(snapshot) / price;
}

export function isValid(snapshot: SnapshotInput): boolean {
  return contract(snapshot) != null && asOf(snapshot).length > 0;
}

export function liquidity(snapshot: SnapshotInput): boolean | null {
  const normalized = quote(snapshot);
  const price = normalized?.mark ?? 0;
  if (Math.abs(price) <= 1e-10) {
    return null;
  }

  const resolvedContract = contract(snapshot);
  if (!resolvedContract) {
    return null;
  }

  const now = asOf(snapshot);
  if (!now) {
    return null;
  }

  const dte = (() => {
    try {
      return timeExpiration.calendarDays(resolvedContract.expiration_date, now);
    } catch {
      return null;
    }
  })();
  if (dte == null) {
    return null;
  }

  const isEtf = ['SPY', 'QQQ', 'IWM', 'SMH', 'GLD'].includes(resolvedContract.underlying_symbol);
  const baseTolerance = isEtf ? 0.06 : 0.10;
  const dteFactor = Math.min(1.0 + (dte / 30) * 0.40, 3.5);
  const absDelta = Math.abs(snapshot?.greeks?.delta ?? 0);
  const deltaFactor = absDelta < 0.3 ? 2.5 : absDelta > 0.7 ? 1.3 : 1.0;
  const tolerance = Math.min(baseTolerance * dteFactor * deltaFactor, 0.40);

  return spreadPct(snapshot) <= tolerance;
}
