import { contract, pricing } from './index';
import type { ContractInput } from './contract';
import type {
  BlackScholesImpliedVolatilityInput,
  BlackScholesInput,
  Greeks,
  OptionContract,
} from './types';

const contractInput: ContractInput = 'SPY250321C00600000';
const canonicalContract: OptionContract | null = contract.canonicalContract(contractInput);

const blackScholesInput: BlackScholesInput = {
  spot: 598,
  strike: 600,
  years: 7 / 365,
  rate: 0.045,
  dividendYield: 0.012,
  volatility: 0.18,
  optionRight: 'call',
};
const price: number = pricing.priceBlackScholes(blackScholesInput);
const greeks: Greeks = pricing.greeksBlackScholes(blackScholesInput);

const impliedVolatilityInput: BlackScholesImpliedVolatilityInput = {
  targetPrice: 4.2,
  spot: 598,
  strike: 600,
  years: 7 / 365,
  rate: 0.045,
  dividendYield: 0.012,
  optionRight: 'call',
};
const impliedVolatility: number = pricing.impliedVolatilityFromPrice(impliedVolatilityInput);

void contractInput;
void canonicalContract;
void blackScholesInput;
void price;
void greeks;
void impliedVolatilityInput;
void impliedVolatility;
