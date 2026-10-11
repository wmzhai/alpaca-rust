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

const OPTIONSTRAT_BUILD_PREFIX = '/build/';

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
    if (Number.isFinite(avgCost) && (avgCost > 1e-12 || avgCost < -1e-12)) {
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

    const occRoot = contract.occ_symbol.length > 15
      ? contract.occ_symbol.slice(0, contract.occ_symbol.length - 15)
      : contract.underlying_symbol;
    const compactContract = `${occRoot}${contract.expiration_date.slice(2, 4)}${contract.expiration_date.slice(5, 7)}${contract.expiration_date.slice(8, 10)}${optionRightCode(contract.option_right)}${formatStrike(contract.strike)}`;

    const premiumSuffix = leg.premiumPerContract == null
      ? ''
      : `@${leg.premiumPerContract.toFixed(2)}`;
    return `.${compactContract}x${leg.quantity}${premiumSuffix}`;
  } catch {
    return null;
  }
}

export function buildOptionstratStockFragment(input: OptionStratStockInput): string | null {
  const quantity = typeof input.quantity === 'number' ? input.quantity : Number(input.quantity);
  const costPerShare = typeof input.costPerShare === 'number'
    ? input.costPerShare
    : Number(input.costPerShare);
  if (!Number.isInteger(quantity) || quantity === 0 || !Number.isFinite(costPerShare)) {
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

function mergeStockPositions(stocks: OptionStratStockInput[]): OptionStratStockInput[] {
  const merged: OptionStratStockInput[] = [];
  for (const stock of stocks) {
    const quantity = typeof stock.quantity === 'number' ? stock.quantity : Number(stock.quantity);
    const costPerShare = typeof stock.costPerShare === 'number'
      ? stock.costPerShare
      : Number(stock.costPerShare);
    const underlyingSymbol = normalizeUnderlyingSymbol(stock.underlyingSymbol);
    if (!underlyingSymbol || !Number.isInteger(quantity) || quantity === 0 || !Number.isFinite(costPerShare)) {
      continue;
    }

    const existing = merged.find((item) => item.underlyingSymbol === underlyingSymbol);
    if (!existing) {
      merged.push({ underlyingSymbol, quantity, costPerShare });
      continue;
    }

    const existingQuantity = typeof existing.quantity === 'number' ? existing.quantity : Number(existing.quantity);
    const existingCost = typeof existing.costPerShare === 'number'
      ? existing.costPerShare
      : Number(existing.costPerShare);
    const totalQuantity = existingQuantity + quantity;
    if (totalQuantity === 0) {
      merged.splice(merged.indexOf(existing), 1);
      continue;
    }
    existing.quantity = totalQuantity;
    existing.costPerShare = (existingQuantity * existingCost + quantity * costPerShare) / totalQuantity;
  }
  return merged;
}

function parseOptionstratStockFragment(fragment: string): OptionStratStockInput | null {
  const atIndex = fragment.lastIndexOf('@');
  const quantityIndex = fragment.lastIndexOf('x');
  if (atIndex <= 0 || quantityIndex <= 0 || quantityIndex >= atIndex) {
    return null;
  }

  const underlyingSymbol = normalizeUnderlyingSymbol(fragment.slice(0, quantityIndex));
  const quantity = Number(fragment.slice(quantityIndex + 1, atIndex));
  const costPerShare = Number(fragment.slice(atIndex + 1));
  if (!underlyingSymbol || !Number.isInteger(quantity) || quantity === 0 || !Number.isFinite(costPerShare)) {
    return null;
  }

  return {
    underlyingSymbol,
    quantity,
    costPerShare,
  };
}

export function mergeOptionstratUrls(
  urls: Array<string | null | undefined>,
  underlyingDisplaySymbolInput?: string | null,
): string | null {
  let underlyingDisplaySymbol = underlyingDisplaySymbolInput ?? null;
  const legs: OptionStratLegInput[] = [];
  const stocks: OptionStratStockInput[] = [];

  for (const rawUrl of urls) {
    if (!rawUrl) {
      continue;
    }

    let parsed: ParsedOptionStratUrl;
    try {
      parsed = parseOptionstratUrl(rawUrl);
    } catch {
      continue;
    }

    if (!underlyingDisplaySymbol) {
      underlyingDisplaySymbol = parsed.underlyingDisplaySymbol;
    }
    if (parsed.underlyingDisplaySymbol !== underlyingDisplaySymbol) {
      continue;
    }

    const optionFragments: string[] = [];
    const stockFragments: string[] = [];
    for (const fragment of parsed.legFragments) {
      if (fragment.startsWith('-.') || fragment.startsWith('.')) {
        optionFragments.push(fragment);
      } else {
        stockFragments.push(fragment);
      }
    }

    try {
      legs.push(
        ...parseOptionstratLegFragments(underlyingDisplaySymbol, optionFragments)
          .map(strategyLegToBuildInput),
      );
    } catch {
      continue;
    }

    for (const fragment of stockFragments) {
      const stock = parseOptionstratStockFragment(fragment);
      if (stock) {
        stocks.push(stock);
      }
    }
  }

  const mergedStocks = mergeStockPositions(stocks);
  if (!underlyingDisplaySymbol || (legs.length === 0 && mergedStocks.length === 0)) {
    return null;
  }

  return buildOptionstratUrl({
    underlyingDisplaySymbol,
    legs,
    stocks: mergedStocks,
  });
}

export function parseOptionstratUrl(url: string): ParsedOptionStratUrl {
  const withoutSuffix = url.split(/[?#]/, 1)[0] ?? url;
  const markerIndex = withoutSuffix.indexOf(OPTIONSTRAT_BUILD_PREFIX);
  if (markerIndex < 0) {
    fail('invalid_optionstrat_url', `invalid optionstrat url: ${url}`);
  }

  const rest = withoutSuffix.slice(markerIndex + OPTIONSTRAT_BUILD_PREFIX.length);
  const [strategy, underlyingPath, fragments = ''] = rest.split('/', 3);
  if (!strategy || !underlyingPath) {
    fail('invalid_optionstrat_url', `invalid optionstrat url: ${url}`);
  }

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
  let prefixOrderSide: 'buy' | 'sell';

  if (body.startsWith('-.')) {
    prefixOrderSide = 'sell';
    body = body.slice(2);
  } else if (body.startsWith('.')) {
    prefixOrderSide = 'buy';
    body = body.slice(1);
  } else {
    fail('invalid_optionstrat_leg_fragment', `invalid optionstrat leg fragment: ${fragment}`);
  }

  const [withoutPremium, premiumText] = body.split('@', 2);
  const xIndex = withoutPremium.lastIndexOf('x');
  const compactContract = xIndex < 0 ? withoutPremium : withoutPremium.slice(0, xIndex);
  const quantityText = xIndex < 0 ? null : withoutPremium.slice(xIndex + 1);
  const signedQuantity = quantityText == null ? 1 : Number(quantityText);
  if (!Number.isInteger(signedQuantity) || signedQuantity === 0) {
    fail('invalid_optionstrat_leg_fragment', `invalid optionstrat leg fragment: ${fragment}`);
  }
  const orderSide = prefixOrderSide === 'sell' || signedQuantity < 0 ? 'sell' : 'buy';
  const ratioQuantity = Math.abs(signedQuantity);

  const compact = parseCompactContract(compactContract);
  const stripped = compact.underlyingSymbol.replace(/\d+$/, '');
  const listedSymbol = stripped || compact.underlyingSymbol;
  if (normalizeUnderlyingSymbol(underlyingDisplaySymbol) !== listedSymbol) {
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
        return value;
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
