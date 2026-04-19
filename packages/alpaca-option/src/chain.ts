import { expiration as timeExpiration } from '@alpaca/time';

import { canonicalContract, type ContractInput } from './contract';

type SnapshotLike = {
  contract?: ContractInput;
} | null | undefined;

type SnapshotCollection<T extends SnapshotLike> = {
  snapshots?: T[] | null;
} | null | undefined;

type NumericInput = number | string | null | undefined;

function coerceNumber(value: NumericInput): number | null {
  if (value == null) {
    return null;
  }

  const parsed = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function strikeTolerance(value: NumericInput): number {
  const parsed = coerceNumber(value);
  return parsed != null && parsed >= 0 ? parsed : 0.01;
}

function matchesContract(
  contract: NonNullable<ReturnType<typeof canonicalContract>>,
  input: {
    occSymbol?: string;
    expirationDate?: string;
    strike?: NumericInput;
    optionRight?: string | null;
    strikeTolerance?: NumericInput;
  },
): boolean {
  if (input.occSymbol) {
    return contract.occ_symbol === input.occSymbol.trim().toUpperCase();
  }

  if (input.expirationDate && contract.expiration_date !== input.expirationDate) {
    return false;
  }

  if (input.optionRight && contract.option_right !== input.optionRight) {
    return false;
  }

  const strike = coerceNumber(input.strike);
  if (strike != null && Math.abs(contract.strike - strike) > strikeTolerance(input.strikeTolerance)) {
    return false;
  }

  return true;
}

export function listSnapshots<T extends SnapshotLike>(input: {
  chain?: SnapshotCollection<T>;
  occSymbol?: string;
  expirationDate?: string;
  strike?: NumericInput;
  optionRight?: string | null;
  strikeTolerance?: NumericInput;
}): T[] {
  const snapshots = input.chain?.snapshots ?? [];
  const results: T[] = [];

  for (const snapshot of snapshots) {
    const contract = canonicalContract(snapshot?.contract);
    if (!contract) {
      continue;
    }

    if (matchesContract(contract, input)) {
      results.push(snapshot);
    }
  }

  return results;
}

export function findSnapshot<T extends SnapshotLike>(input: {
  chain?: SnapshotCollection<T>;
  occSymbol?: string;
  expirationDate?: string;
  strike?: NumericInput;
  optionRight?: string | null;
  strikeTolerance?: NumericInput;
}): T | null {
  return listSnapshots(input)[0] ?? null;
}

export function expirationDates<T extends SnapshotLike>(input: {
  chain?: SnapshotCollection<T>;
  optionRight?: string | null;
  minCalendarDays?: NumericInput;
  maxCalendarDays?: NumericInput;
  now?: string;
}): Array<{ expirationDate: string; calendarDays: number }> {
  const minCalendarDays = coerceNumber(input.minCalendarDays);
  const maxCalendarDays = coerceNumber(input.maxCalendarDays);
  const seen = new Set<string>();
  const results: Array<{ expirationDate: string; calendarDays: number }> = [];

  for (const snapshot of input.chain?.snapshots ?? []) {
    const contract = canonicalContract(snapshot?.contract);
    if (!contract) {
      continue;
    }

    if (input.optionRight && contract.option_right !== input.optionRight) {
      continue;
    }

    if (seen.has(contract.expiration_date)) {
      continue;
    }

    const calendarDays = timeExpiration.calendarDays(contract.expiration_date, input.now);
    if (minCalendarDays != null && calendarDays < minCalendarDays) {
      continue;
    }
    if (maxCalendarDays != null && calendarDays > maxCalendarDays) {
      continue;
    }

    seen.add(contract.expiration_date);
    results.push({
      expirationDate: contract.expiration_date,
      calendarDays,
    });
  }

  return results.sort((left, right) => {
    if (left.calendarDays !== right.calendarDays) {
      return left.calendarDays - right.calendarDays;
    }
    return left.expirationDate.localeCompare(right.expirationDate);
  });
}
