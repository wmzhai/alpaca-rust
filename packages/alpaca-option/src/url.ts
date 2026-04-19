import { buildOccSymbol, normalizeUnderlyingSymbol, parseOccSymbol } from './contract';
import { formatStrike, optionRightCode } from './display';
import { fail } from './error';
import type {
  OptionPosition,
  OptionStratLegInput,
  OptionStratStockInput,
  OptionStratUrlInput,
  ParsedOptionStratUrl,
  StrategyLegInput,
} from './types';

const OPTIONSTRAT_PREFIX = '/build/custom/';

type ResolvedOptionstratLegInput = {
  occSymbol: string;
  quantity: number;
  premiumPerContract: number | null;
};

export function toOptionstratUnderlyingPath(symbol: string): string {
  return symbol.trim().replace(/\./g, '/').replace(/\//g, '%2F');
}

export function fromOptionstratUnderlyingPath(path: string): string {
  return path.replace(/%2F/gi, '/').replace(/\//g, '.');
}

function resolveLegInput(input: OptionStratLegInput): ResolvedOptionstratLegInput | null {
  const quantity = typeof input.quantity === 'number' ? input.quantity : Number(input.quantity);
  if (!Number.isInteger(quantity) || quantity === 0) {
    return null;
  }
  const premiumPerContract = input.premiumPerContract == null
    ? null
    : (typeof input.premiumPerContract === 'number'
      ? input.premiumPerContract
      : Number(input.premiumPerContract));
  if (premiumPerContract != null && !Number.isFinite(premiumPerContract)) {
    return null;
  }
  const occSymbol = input.occSymbol?.trim()
    ? input.occSymbol
    : buildOccSymbol(
      input.underlyingSymbol ?? '',
      input.expirationDate ?? '',
      input.strike ?? '',
      input.optionRight ?? '',
    );
  if (!occSymbol) {
    return null;
  }
  return {
    occSymbol,
    quantity,
    premiumPerContract,
  };
}

function strategyLegToBuildInput(leg: StrategyLegInput): OptionStratLegInput {
  return {
    occSymbol: leg.contract.occ_symbol,
    quantity: leg.orderSide === 'sell' ? -leg.ratioQuantity : leg.ratioQuantity,
    premiumPerContract: leg.premiumPerContract,
  };
}

function resolvePositionLeg(position: OptionPosition): OptionStratLegInput | null {
  if (!parseOccSymbol(position.contract)) {
    return null;
  }

  if (!Number.isInteger(position.qty) || position.qty === 0) {
    return null;
  }

  const premiumPerContract = (() => {
    const avgCost = Number(position.avg_cost);
    if (Number.isFinite(avgCost) && Math.abs(avgCost) > 1e-12) {
      return avgCost;
    }

    return position.snapshot.quote.mark ?? position.snapshot.quote.last ?? null;
  })();

  return {
    occSymbol: position.contract,
    quantity: position.qty,
    premiumPerContract,
  };
}

export function buildOptionstratLegFragment(input: OptionStratLegInput): string | null {
  const leg = resolveLegInput(input);
  if (!leg) {
    return null;
  }

  try {
    const contract = parseOccSymbol(leg.occSymbol);
    if (!contract) {
      return null;
    }

    const prefix = leg.quantity < 0 ? '-.' : '.';
    const compactContract = `${contract.underlying_symbol}${contract.expiration_date.slice(2, 4)}${contract.expiration_date.slice(5, 7)}${contract.expiration_date.slice(8, 10)}${optionRightCode(contract.option_right)}${formatStrike(contract.strike)}`;

    const premiumSuffix = leg.premiumPerContract == null
      ? ''
      : `@${Math.abs(leg.premiumPerContract).toFixed(2)}`;
    return `${prefix}${compactContract}x${Math.abs(leg.quantity)}${premiumSuffix}`;
  } catch {
    return null;
  }
}

export function buildOptionstratStockFragment(input: OptionStratStockInput): string | null {
  const quantity = typeof input.quantity === 'number' ? input.quantity : Number(input.quantity);
  const costPerShare = typeof input.costPerShare === 'number'
    ? input.costPerShare
    : Number(input.costPerShare);
  if (!Number.isInteger(quantity) || quantity <= 0 || !Number.isFinite(costPerShare)) {
    return null;
  }

  const symbol = normalizeUnderlyingSymbol(input.underlyingSymbol);
  if (!symbol) {
    return null;
  }

  return `${symbol}x${quantity}@${costPerShare.toFixed(2)}`;
}

export function buildOptionstratUrl(
  input: OptionStratUrlInput & { positions?: OptionPosition[] },
): string | null {
  const positionLegs = (input.positions ?? []).map((position) => resolvePositionLeg(position));
  if (positionLegs.some((leg) => leg == null)) {
    return null;
  }

  const legs = [...(input.legs ?? []), ...positionLegs.filter((leg): leg is OptionStratLegInput => leg != null)];
  const legFragments = legs.map((leg) => buildOptionstratLegFragment(leg));
  if (legFragments.some((fragment) => fragment == null)) {
    return null;
  }

  const stockFragments = (input.stocks ?? []).map((stock) => buildOptionstratStockFragment(stock));
  if (stockFragments.some((fragment) => fragment == null)) {
    return null;
  }

  const fragments = [
    ...legFragments.filter((fragment): fragment is string => fragment != null),
    ...stockFragments.filter((fragment): fragment is string => fragment != null),
  ];
  if (fragments.length === 0) {
    return null;
  }

  return `https://optionstrat.com/build/custom/${toOptionstratUnderlyingPath(input.underlyingDisplaySymbol)}/${fragments.join(',')}`;
}

export function mergeOptionstratUrls(
  urls: Array<string | null | undefined>,
  underlyingDisplaySymbolInput?: string | null,
): string | null {
  let underlyingDisplaySymbol = underlyingDisplaySymbolInput ?? null;
  const legs: OptionStratLegInput[] = [];

  for (const rawUrl of urls) {
    if (!rawUrl) {
      continue;
    }

    try {
      const parsed = parseOptionstratUrl(rawUrl);
      if (!underlyingDisplaySymbol) {
        underlyingDisplaySymbol = parsed.underlyingDisplaySymbol;
      }
      if (parsed.underlyingDisplaySymbol !== underlyingDisplaySymbol) {
        continue;
      }

      legs.push(
        ...parseOptionstratLegFragments(underlyingDisplaySymbol, parsed.legFragments)
          .map(strategyLegToBuildInput),
      );
    } catch {
      continue;
    }
  }

  if (!underlyingDisplaySymbol || legs.length === 0) {
    return null;
  }

  return buildOptionstratUrl({
    underlyingDisplaySymbol,
    legs,
    stocks: [],
  });
}

export function parseOptionstratUrl(url: string): ParsedOptionStratUrl {
  const withoutSuffix = url.split(/[?#]/, 1)[0] ?? url;
  const markerIndex = withoutSuffix.indexOf(OPTIONSTRAT_PREFIX);
  if (markerIndex < 0) {
    fail('invalid_optionstrat_url', `invalid optionstrat url: ${url}`);
  }

  const rest = withoutSuffix.slice(markerIndex + OPTIONSTRAT_PREFIX.length);
  const slashIndex = rest.indexOf('/');
  if (slashIndex < 0) {
    fail('invalid_optionstrat_url', `invalid optionstrat url: ${url}`);
  }

  const underlyingPath = rest.slice(0, slashIndex);
  const fragments = rest.slice(slashIndex + 1);
  return {
    underlyingDisplaySymbol: fromOptionstratUnderlyingPath(underlyingPath),
    legFragments: fragments ? fragments.split(',') : [],
  };
}

function parseCompactContract(input: string): {
  underlyingSymbol: string;
  expirationDate: string;
  optionRightCode: 'C' | 'P';
  strike: number;
} {
  for (let index = 7; index < input.length - 1; index += 1) {
    const rightCode = input[index];
    if (rightCode !== 'C' && rightCode !== 'P') {
      continue;
    }

    const dateStart = index - 6;
    const underlyingSymbol = input.slice(0, dateStart);
    const date = input.slice(dateStart, index);
    const strikeText = input.slice(index + 1);

    if (!underlyingSymbol || underlyingSymbol.length > 6 || !/^[A-Z0-9]+$/.test(underlyingSymbol)) {
      continue;
    }
    if (!/^\d{6}$/.test(date) || !/^\d+(?:\.\d+)?$/.test(strikeText)) {
      continue;
    }

    return {
      underlyingSymbol,
      expirationDate: `20${date.slice(0, 2)}-${date.slice(2, 4)}-${date.slice(4, 6)}`,
      optionRightCode: rightCode,
      strike: Number(strikeText),
    };
  }

  fail('invalid_optionstrat_leg_fragment', `invalid compact contract: ${input}`);
}

function parseOptionstratLegFragment(fragment: string, underlyingDisplaySymbol: string): StrategyLegInput {
  let body = fragment;
  let orderSide: 'buy' | 'sell';

  if (body.startsWith('-.')) {
    orderSide = 'sell';
    body = body.slice(2);
  } else if (body.startsWith('.')) {
    orderSide = 'buy';
    body = body.slice(1);
  } else {
    fail('invalid_optionstrat_leg_fragment', `invalid optionstrat leg fragment: ${fragment}`);
  }

  const [withoutPremium, premiumText] = body.split('@', 2);
  const xIndex = withoutPremium.lastIndexOf('x');
  if (xIndex < 0) {
    fail('invalid_optionstrat_leg_fragment', `invalid optionstrat leg fragment: ${fragment}`);
  }

  const compactContract = withoutPremium.slice(0, xIndex);
  const quantityText = withoutPremium.slice(xIndex + 1);
  const ratioQuantity = Number(quantityText);
  if (!Number.isInteger(ratioQuantity) || ratioQuantity <= 0) {
    fail('invalid_optionstrat_leg_fragment', `invalid optionstrat leg fragment: ${fragment}`);
  }

  const compact = parseCompactContract(compactContract);
  if (normalizeUnderlyingSymbol(underlyingDisplaySymbol) !== compact.underlyingSymbol) {
    fail('invalid_optionstrat_leg_fragment', `fragment underlying does not match: ${fragment}`);
  }

  const occSymbol = buildOccSymbol(
    compact.underlyingSymbol,
    compact.expirationDate,
    compact.strike,
    compact.optionRightCode === 'C' ? 'call' : 'put',
  );
  if (!occSymbol) {
    fail('invalid_optionstrat_leg_fragment', `invalid optionstrat leg fragment: ${fragment}`);
  }

  const premiumPerContract = premiumText == null
    ? null
    : (() => {
        const value = Number(premiumText);
        if (!Number.isFinite(value)) {
          fail('invalid_optionstrat_leg_fragment', `invalid optionstrat leg fragment: ${fragment}`);
        }
        return Math.abs(value);
      })();

  return {
    contract: (() => {
      const contract = parseOccSymbol(occSymbol);
      if (!contract) {
        fail('invalid_optionstrat_leg_fragment', `invalid optionstrat leg fragment: ${fragment}`);
      }
      return contract;
    })(),
    orderSide,
    ratioQuantity,
    premiumPerContract,
  };
}

export function parseOptionstratLegFragments(
  underlyingDisplaySymbol: string,
  legFragments: string[],
): StrategyLegInput[] {
  return legFragments.map((fragment) => parseOptionstratLegFragment(fragment, underlyingDisplaySymbol));
}
