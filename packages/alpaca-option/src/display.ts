import { display as timeDisplay } from '@alpaca/time';

import { canonicalContract, type ContractInput } from './contract';
import type { ContractDisplay, OptionRight, OptionRightCode } from './types';

export function formatStrike(strike: number): string {
  let text = strike.toFixed(3);
  while (text.includes('.') && text.endsWith('0')) {
    text = text.slice(0, -1);
  }
  if (text.endsWith('.')) {
    text = text.slice(0, -1);
  }
  return text;
}

export function positionStrike(position?: {
  contract?: ContractInput;
} | null): string {
  const contract = canonicalContract(position?.contract);
  return contract ? formatStrike(contract.strike) : '-';
}

export function compactContract(
  contract?: ContractInput,
  expirationFormat: 'mm-dd' | 'yy-mm-dd' | 'yymmdd' = 'mm-dd',
): string {
  return contractDisplay(contract, expirationFormat)?.compact ?? '-';
}

export function contractDisplay(
  contractInput?: ContractInput,
  expirationFormat: 'mm-dd' | 'yy-mm-dd' | 'yymmdd' = 'mm-dd',
): ContractDisplay | null {
  const contract = canonicalContract(contractInput);
  if (!contract) {
    return null;
  }

  const strike = formatStrike(contract.strike);
  const optionRight = optionRightCode(contract.option_right);
  const expiration = timeDisplay.compact(contract.expiration_date, expirationFormat);
  return {
    strike,
    expiration,
    compact: `${strike}${optionRight}@${expiration}`,
    optionRightCode: optionRight,
  };
}

export function optionRightCode(optionRight: OptionRight): OptionRightCode {
  return optionRight === 'call' ? 'C' : 'P';
}
