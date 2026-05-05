export type OptionRight = 'call' | 'put';
export type OptionRightCode = 'C' | 'P';
export type OrderSide = 'buy' | 'sell';
export type PositionSide = 'long' | 'short';
export type ExecutionAction = 'open' | 'close';
export type PositionIntent = 'buy_to_open' | 'sell_to_open' | 'buy_to_close' | 'sell_to_close';
export type MoneynessLabel = 'itm' | 'atm' | 'otm';
export type AssignmentRiskLevel = 'danger' | 'critical' | 'high' | 'medium' | 'low' | 'safe';

export type OptionContract = {
  underlying_symbol: string;
  expiration_date: string;
  strike: number;
  option_right: OptionRight;
  occ_symbol: string;
};

export type OptionQuote = {
  bid: number | null;
  ask: number | null;
  mark: number | null;
  last: number | null;
};

export type ContractDisplay = {
  strike: string;
  expiration: string;
  compact: string;
  optionRightCode: OptionRightCode;
};

export type Greeks = {
  delta: number;
  gamma: number;
  vega: number;
  theta: number;
  rho: number;
};

export type BlackScholesInput = {
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  volatility: number;
  optionRight: OptionRight;
};

export type BlackScholesImpliedVolatilityInput = {
  targetPrice: number;
  spot: number;
  strike: number;
  years: number;
  rate: number;
  dividendYield: number;
  optionRight: OptionRight;
  lowerBound?: number;
  upperBound?: number;
  tolerance?: number;
  maxIterations?: number;
};

export type OptionSnapshot = {
  as_of: string;
  contract: OptionContract;
  quote: OptionQuote;
  greeks: Greeks | null;
  implied_volatility: number | null;
  underlying_price: number | null;
};

export type OptionPosition = {
  contract: string;
  snapshot: OptionSnapshot;
  qty: number;
  avg_cost: string;
  leg_type: string;
};

export type ShortItmPosition = {
  contract: OptionContract;
  quantity: number;
  optionPrice: number;
  intrinsic: number;
  extrinsic: number;
};

export type StrategyLegInput = {
  contract: OptionContract;
  orderSide: OrderSide;
  ratioQuantity: number;
  premiumPerContract: number | null;
};

export type QuotedLeg = {
  contract: OptionContract;
  orderSide: OrderSide;
  ratioQuantity: number;
  quote: OptionQuote;
  snapshot: OptionSnapshot | null;
};

export type GreeksInput = Partial<Greeks>;

export type ExecutionSnapshot = {
  contract: string;
  timestamp: string;
  bid: string;
  ask: string;
  price: string;
  greeks: Greeks;
  iv: number;
};

export type ExecutionLeg = {
  symbol: string;
  ratio_qty: string;
  side: OrderSide;
  position_intent: PositionIntent;
  leg_type: string;
  snapshot: ExecutionSnapshot | null;
};

export type RollLegSelection = {
  legType: string;
  quantity?: number | null;
};

export type RollRequest = {
  current_contract: string;
  leg_type?: string;
  qty: number;
  new_strike?: number;
  new_expiration: string;
};

export type ExecutionLegInput = {
  action: ExecutionAction;
  legType: string;
  contract: string;
  quantity?: number | null;
  snapshot?: ExecutionSnapshot | null;
  timestamp?: string | null;
  bid?: number | string | null;
  ask?: number | string | null;
  price?: number | string | null;
  spreadPercent?: number | string | null;
  greeks?: GreeksInput | null;
  iv?: number | string | null;
};

export type ExecutionQuoteRange = {
  bestPrice: number;
  worstPrice: number;
};

export type ScaledExecutionQuote = {
  structureQuantity: number;
  price: number;
  totalPrice: number;
  totalDollars: number;
};

export type ScaledExecutionQuoteRange = {
  structureQuantity: number;
  perStructure: ExecutionQuoteRange;
  perOrder: ExecutionQuoteRange;
  dollars: ExecutionQuoteRange;
};

export type ParsedOptionStratUrl = {
  underlyingDisplaySymbol: string;
  legFragments: string[];
};

export type OptionStratLegInput = {
  occSymbol?: string;
  underlyingSymbol?: string;
  expirationDate?: string;
  strike?: number | string;
  optionRight?: string;
  quantity: number | string;
  premiumPerContract?: number | string | null;
};

export type OptionStratStockInput = {
  underlyingSymbol: string;
  quantity: number | string;
  costPerShare: number | string;
};

export type OptionStratUrlInput = {
  underlyingDisplaySymbol: string;
  legs?: OptionStratLegInput[];
  stocks?: OptionStratStockInput[];
};

export type OptionChain = {
  underlying_symbol: string;
  as_of: string;
  snapshots: OptionSnapshot[];
};

export type OptionChainRecord = {
  as_of: string;
  underlying_symbol: string;
  occ_symbol: string;
  expiration_date: string;
  option_right: OptionRight;
  strike: number;
  underlying_price: number | null;
  bid: number | null;
  ask: number | null;
  mark: number | null;
  last: number | null;
  implied_volatility: number | null;
  delta: number | null;
  gamma: number | null;
  vega: number | null;
  theta: number | null;
  rho: number | null;
};

export type LiquidityOptionData = {
  occ_symbol: string;
  option_right: string;
  strike: number;
  expiration_date: string;
  dte: number;
  delta: number;
  spread_pct: number;
  liquidity?: boolean | null;
  bid: number;
  ask: number;
  mark: number;
  implied_volatility: number;
};

export type LiquidityStats = {
  total_count: number;
  avg_spread_pct: number;
  median_spread_pct: number;
  min_spread_pct: number;
  max_spread_pct: number;
  dte_range: [number, number];
  delta_range: [number, number];
};

export type LiquidityData = {
  underlying_symbol: string;
  as_of: string;
  underlying_price: number;
  options: LiquidityOptionData[];
  stats: LiquidityStats;
};

export type LiquidityBatchResponse = {
  results: Record<string, LiquidityData>;
};

export type PayoffLegInput = {
  optionRight: OptionRight;
  positionSide: PositionSide;
  strike: number;
  premium: number;
  quantity: number;
};

export type StrategyValuationPosition = {
  contract: OptionContract;
  quantity: number;
  avg_entry_price: number | null;
  implied_volatility: number | null;
  mark_price: number | null;
  reference_underlying_price: number | null;
};

export type OptionStrategyInput = {
  positions: StrategyValuationPosition[];
  evaluation_time?: string | null;
  entry_cost: number | null;
  rate?: number | null;
  dividend_yield?: number | null;
  long_volatility_shift?: number | null;
};

export type OptionStrategyCurveInput = {
  lower_bound: number;
  upper_bound: number;
  step: number;
};

export type OptionStrategyCurvePoint = {
  underlying_price: number;
  mark_value: number;
  pnl: number;
};

export type OptionStrategyBreakEvenBracketInput = {
  lower_bound: number;
  upper_bound: number;
  tolerance?: number | null;
  maxIterations?: number | null;
};

export type StrategyPnlInput = {
  positions: StrategyValuationPosition[];
  underlying_price: number;
  evaluation_time: string;
  entry_cost: number | null;
  rate: number;
  dividend_yield: number | null;
  long_volatility_shift: number | null;
};

export type StrategyBreakEvenInput = {
  positions: StrategyValuationPosition[];
  evaluation_time: string;
  entry_cost: number | null;
  rate: number;
  dividend_yield: number | null;
  long_volatility_shift: number | null;
  lower_bound: number;
  upper_bound: number;
  scan_step?: number;
  tolerance?: number;
  maxIterations?: number;
};
