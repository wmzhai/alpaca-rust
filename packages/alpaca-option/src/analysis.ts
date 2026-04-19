import { fail } from './error';
import { canonicalContract, type ContractInput } from './contract';
import { extrinsicValue, greeksBlackScholes, intrinsicValue } from './pricing';
import type { AssignmentRiskLevel, MoneynessLabel, OptionPosition, OptionRight, ShortItmPosition } from './types';

const CONTRACT_MULTIPLIER = 100;
type NumericInput = number | string | null | undefined;

function ensureFinite(name: string, value: number): void {
  if (!Number.isFinite(value)) {
    fail('invalid_analysis_input', `${name} must be finite: ${value}`);
  }
}

function ensurePositive(name: string, value: number): void {
  ensureFinite(name, value);
  if (value <= 0) {
    fail('invalid_analysis_input', `${name} must be greater than zero: ${value}`);
  }
}

function validateOptionRight(optionRight: OptionRight): OptionRight {
  if (optionRight !== 'call' && optionRight !== 'put') {
    fail('invalid_analysis_input', `invalid option right: ${optionRight}`);
  }
  return optionRight;
}

function coerceNumber(value: NumericInput): number | null {
  if (value == null) {
    return null;
  }

  const parsed = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function isShortPosition(position: OptionPosition): boolean {
  const normalized = position.leg_type.trim().toLowerCase();
  if (normalized.startsWith('short')) {
    return true;
  }
  if (normalized.startsWith('long')) {
    return false;
  }
  return position.qty < 0;
}

function positionQuantity(position: OptionPosition): number | null {
  if (!Number.isFinite(position.qty)) {
    return null;
  }

  const normalized = Math.abs(Math.trunc(position.qty));
  return normalized > 0 ? normalized : null;
}

function positionPrice(position: OptionPosition): number | null {
  return position.snapshot.quote.mark ?? position.snapshot.quote.last ?? null;
}

function resolvedContract(position: OptionPosition) {
  return canonicalContract(position.contract);
}

function normalizeStructureQuantity(value: NumericInput): number {
  const quantity = coerceNumber(value);
  if (quantity == null) {
    return 1;
  }

  const normalized = Math.abs(Math.trunc(quantity));
  return normalized > 0 ? normalized : 1;
}

export function annualizedPremiumYield(premium: number, capitalBase: number, years: number): number {
  ensureFinite('premium', premium);
  ensurePositive('capitalBase', capitalBase);
  ensurePositive('years', years);
  return premium / capitalBase / years;
}

export function annualizedPremiumYieldDays(
  premium: number,
  capitalBase: number,
  calendarDays: number,
): number {
  ensurePositive('calendarDays', calendarDays);
  return annualizedPremiumYield(premium, capitalBase, calendarDays / 365);
}

export function calendarForwardFactor(
  shortIv: number,
  longIv: number,
  shortYears: number,
  longYears: number,
): number {
  ensurePositive('shortIv', shortIv);
  ensurePositive('longIv', longIv);
  ensurePositive('shortYears', shortYears);
  ensurePositive('longYears', longYears);
  if (longYears <= shortYears) {
    fail('invalid_analysis_input', `longYears must be greater than shortYears: ${longYears} <= ${shortYears}`);
  }

  const shortVariance = shortIv * shortIv;
  const longVariance = longIv * longIv;
  const forwardVariance = (longVariance * longYears - shortVariance * shortYears)
    / (longYears - shortYears);
  if (forwardVariance <= 0) {
    fail('invalid_analysis_input', `forward variance must be positive: ${forwardVariance}`);
  }

  const forwardIv = Math.sqrt(forwardVariance);
  return (shortIv - forwardIv) / forwardIv;
}

export function moneynessRatio(spot: number, strike: number): number {
  ensurePositive('spot', spot);
  ensurePositive('strike', strike);
  return spot / strike;
}

export function moneynessLabel(
  spot: number,
  strike: number,
  optionRightInput: OptionRight,
  atmBand = 0.02,
): MoneynessLabel {
  const optionRight = validateOptionRight(optionRightInput);
  ensureFinite('atmBand', atmBand);
  if (atmBand < 0) {
    fail('invalid_analysis_input', `atmBand must be non-negative: ${atmBand}`);
  }

  const ratio = moneynessRatio(spot, strike);
  if (Math.abs(ratio - 1) <= atmBand) {
    return 'atm';
  }

  if (optionRight === 'call') {
    return ratio > 1 ? 'itm' : 'otm';
  }
  return ratio < 1 ? 'itm' : 'otm';
}

export function otmPercent(spot: number, strike: number, optionRightInput: OptionRight): number {
  ensurePositive('spot', spot);
  ensurePositive('strike', strike);
  const optionRight = validateOptionRight(optionRightInput);
  return optionRight === 'call'
    ? ((strike - spot) / spot) * 100
    : ((spot - strike) / spot) * 100;
}

export function positionOtmPercent(
  spotInput: NumericInput,
  position?: { contract?: ContractInput } | null,
): number | null {
  const spot = coerceNumber(spotInput);
  if (spot == null || spot <= 0) {
    return null;
  }

  const contract = canonicalContract(position?.contract ?? null);
  if (!contract) {
    return null;
  }

  return otmPercent(spot, contract.strike, contract.option_right);
}

export function assignmentRisk(extrinsic: number): AssignmentRiskLevel {
  ensureFinite('extrinsic', extrinsic);

  if (extrinsic < 0) {
    return 'danger';
  }
  if (extrinsic < 0.05) {
    return 'critical';
  }
  if (extrinsic < 0.1) {
    return 'high';
  }
  if (extrinsic < 0.3) {
    return 'medium';
  }
  if (extrinsic < 1) {
    return 'low';
  }
  return 'safe';
}

export function shortExtrinsicAmount(
  spotInput: NumericInput,
  positions?: OptionPosition[] | null,
  structureQuantity?: NumericInput,
): number | null {
  const spot = coerceNumber(spotInput);
  if (spot == null || spot <= 0) {
    return null;
  }

  let totalExtrinsicPerShare = 0;
  let hasShortPosition = false;

  for (const position of positions ?? []) {
    if (!isShortPosition(position)) {
      continue;
    }

    hasShortPosition = true;
    const quantity = positionQuantity(position);
    const optionPrice = positionPrice(position);
    const contract = resolvedContract(position);
    if (quantity == null || optionPrice == null || !contract) {
      return null;
    }

    totalExtrinsicPerShare += extrinsicValue(
      optionPrice,
      spot,
      contract.strike,
      contract.option_right,
    ) * quantity;
  }

  if (!hasShortPosition) {
    return null;
  }

  return totalExtrinsicPerShare * CONTRACT_MULTIPLIER * normalizeStructureQuantity(structureQuantity);
}

export function shortItmPositions(
  spotInput: NumericInput,
  positions?: OptionPosition[] | null,
): ShortItmPosition[] {
  const spot = coerceNumber(spotInput);
  if (spot == null || spot <= 0) {
    return [];
  }

  const items: ShortItmPosition[] = [];
  for (const position of positions ?? []) {
    if (!isShortPosition(position)) {
      continue;
    }

    const quantity = positionQuantity(position);
    const contract = resolvedContract(position);
    if (quantity == null || !contract) {
      continue;
    }

    const optionPrice = positionPrice(position) ?? 0;
    const intrinsic = intrinsicValue(spot, contract.strike, contract.option_right);
    if (intrinsic <= 0) {
      continue;
    }

    items.push({
      contract,
      quantity,
      optionPrice,
      intrinsic,
      extrinsic: extrinsicValue(optionPrice, spot, contract.strike, contract.option_right),
    });
  }

  return items;
}

export function strikeForTargetDelta(
  spot: number,
  years: number,
  rate: number,
  dividendYield: number,
  volatility: number,
  targetDelta: number,
  optionRightInput: OptionRight,
  strikeStep: number,
): number {
  ensurePositive('spot', spot);
  ensurePositive('years', years);
  ensureFinite('rate', rate);
  ensureFinite('dividendYield', dividendYield);
  ensurePositive('volatility', volatility);
  ensureFinite('targetDelta', targetDelta);
  ensurePositive('strikeStep', strikeStep);
  const optionRight = validateOptionRight(optionRightInput);

  if (optionRight === 'call') {
    for (
      let strike = Math.round(spot / strikeStep) * strikeStep;
      strike < spot * 1.5 + strikeStep * 0.5;
      strike += strikeStep
    ) {
      const greeks = greeksBlackScholes({
        spot,
        strike,
        years,
        rate,
        dividendYield,
        volatility,
        optionRight: 'call',
      });
      if (greeks.delta <= targetDelta) {
        return strike;
      }
    }
  } else {
    for (
      let strike = Math.round((spot * 0.7) / strikeStep) * strikeStep;
      strike <= spot * 1.1 + strikeStep * 0.5;
      strike += strikeStep
    ) {
      const greeks = greeksBlackScholes({
        spot,
        strike,
        years,
        rate,
        dividendYield,
        volatility,
        optionRight: 'put',
      });
      if (greeks.delta <= targetDelta) {
        return strike;
      }
    }
  }

  fail('target_delta_not_found', `no strike found for target delta: ${targetDelta}`);
}
