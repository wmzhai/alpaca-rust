import { expiration as timeExpiration } from '@alpaca/time';

import * as executionQuote from './executionQuote';
import { canonicalContract } from './contract';
import { round } from './numeric';
import type { ExecutionSnapshot, OptionContract, OptionSnapshot } from './types';

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
