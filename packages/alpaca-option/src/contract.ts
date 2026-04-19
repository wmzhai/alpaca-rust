import { clock as timeClock } from '@alpaca/time';

import { fail } from './error';
import type { OptionContract, OptionPosition, OptionRight } from './types';

const OCC_TAIL_LENGTH = 15;
const MAX_UNDERLYING_LENGTH = 6;

function canonicalUnderlyingSymbol(symbol: string): string {
  return symbol
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, '');
}

function ensureUnderlyingSymbol(symbol: string): string {
  const trimmed = symbol.trim();
  if (!trimmed || !/^[A-Z0-9./]+$/i.test(trimmed)) {
    fail('invalid_underlying_symbol', `invalid underlying symbol: ${symbol}`);
  }

  const normalized = canonicalUnderlyingSymbol(symbol);
  if (!normalized || normalized.length > MAX_UNDERLYING_LENGTH) {
    fail('invalid_underlying_symbol', `invalid underlying symbol: ${symbol}`);
  }
  return normalized;
}

function optionRightFromCode(code: string): OptionRight {
  if (code === 'C') {
    return 'call';
  }
  if (code === 'P') {
    return 'put';
  }
  fail('invalid_option_right_code', `invalid option right code: ${code}`);
}

function optionRightCode(optionRight: OptionRight): string {
  return optionRight === 'call' ? 'C' : 'P';
}

function normalizeOptionRight(input: string): OptionRight | null {
  const normalized = input.trim().toLowerCase();
  if (normalized === 'call' || normalized === 'c') {
    return 'call';
  }
  if (normalized === 'put' || normalized === 'p') {
    return 'put';
  }
  return null;
}

export type ContractInput = string | OptionContract | OptionPosition | {
  contract?: string | OptionContract | OptionPosition | null;
  occ_symbol?: string | null;
  underlying_symbol?: string | null;
  expiration_date?: string | null;
  strike?: number | string | null;
  option_right?: string | null;
} | null | undefined;

function normalizedContractInput(input: ContractInput): OptionContract | null {
  if (!input) {
    return null;
  }

  if (typeof input === 'string') {
    return parseOccSymbol(input);
  }

  if (typeof input !== 'object') {
    return null;
  }

  if ('contract' in input && input.contract != null) {
    return normalizedContractInput(input.contract as ContractInput);
  }

  const direct = 'occ_symbol' in input && typeof input.occ_symbol === 'string'
    ? parseOccSymbol(input.occ_symbol)
    : null;
  if (direct) {
    return direct;
  }

  const occSymbol = buildOccSymbol(
    ('underlying_symbol' in input ? input.underlying_symbol : '') ?? '',
    ('expiration_date' in input ? input.expiration_date : '') ?? '',
    'strike' in input ? input.strike ?? NaN : NaN,
    ('option_right' in input ? input.option_right : '') ?? '',
  );
  if (!occSymbol) {
    return null;
  }

  return parseOccSymbol(occSymbol);
}

export function normalizeUnderlyingSymbol(symbol: string): string {
  return canonicalUnderlyingSymbol(symbol);
}

export function isOccSymbol(occSymbol: string): boolean {
  return parseOccSymbol(occSymbol) != null;
}

export function parseOccSymbol(occSymbol: string): OptionContract | null {
  try {
    const normalized = occSymbol.trim().toUpperCase();
    if (normalized.length <= OCC_TAIL_LENGTH) {
      fail('invalid_occ_symbol', `invalid occ symbol: ${occSymbol}`);
    }

    const split = normalized.length - OCC_TAIL_LENGTH;
    const underlyingSymbol = normalized.slice(0, split);
    if (!underlyingSymbol || underlyingSymbol.length > MAX_UNDERLYING_LENGTH || !/^[A-Z0-9]+$/.test(underlyingSymbol)) {
      fail('invalid_occ_symbol', `invalid occ symbol: ${occSymbol}`);
    }

    const yy = normalized.slice(split, split + 2);
    const mm = normalized.slice(split + 2, split + 4);
    const dd = normalized.slice(split + 4, split + 6);
    const expirationDate = (() => {
      try {
        return timeClock.parseDate(`20${yy}-${mm}-${dd}`);
      } catch {
        fail('invalid_occ_symbol', `invalid occ symbol: ${occSymbol}`);
      }
    })();

    const optionRight = (() => {
      try {
        return optionRightFromCode(normalized.slice(split + 6, split + 7));
      } catch {
        fail('invalid_occ_symbol', `invalid occ symbol: ${occSymbol}`);
      }
    })();
    const strikeDigits = normalized.slice(split + 7);
    if (!/^\d{8}$/.test(strikeDigits)) {
      fail('invalid_occ_symbol', `invalid occ symbol: ${occSymbol}`);
    }

    return {
      underlying_symbol: underlyingSymbol,
      expiration_date: expirationDate,
      strike: Number(strikeDigits) / 1000,
      option_right: optionRight,
      occ_symbol: normalized,
    };
  } catch {
    return null;
  }
}

export function buildOccSymbol(
  underlyingSymbolInput: string,
  expirationDateInput: string,
  strikeInput: string | number,
  optionRightInput: string,
): string | null {
  const normalizedOptionRight = normalizeOptionRight(optionRightInput);
  const strike = typeof strikeInput === 'number' ? strikeInput : Number(strikeInput);
  if (!normalizedOptionRight || !Number.isFinite(strike)) {
    return null;
  }

  try {
    const underlyingSymbol = ensureUnderlyingSymbol(underlyingSymbolInput);
    const expirationDate = (() => {
      try {
        return timeClock.parseDate(expirationDateInput);
      } catch {
        fail('invalid_expiration_date', `invalid expiration date: ${expirationDateInput}`);
      }
    })();

    if (strike < 0) {
      fail('invalid_strike', `invalid strike: ${strikeInput}`);
    }

    const strikeThousandths = Math.round(strike * 1000);
    if (strikeThousandths < 0 || strikeThousandths > 99_999_999) {
      fail('invalid_strike', `invalid strike: ${strikeInput}`);
    }

    const yymmdd = `${expirationDate.slice(2, 4)}${expirationDate.slice(5, 7)}${expirationDate.slice(8, 10)}`;
    return `${underlyingSymbol}${yymmdd}${optionRightCode(normalizedOptionRight)}${strikeThousandths.toString().padStart(8, '0')}`;
  } catch {
    return null;
  }
}

export function canonicalContract(input: ContractInput): OptionContract | null {
  return normalizedContractInput(input);
}
