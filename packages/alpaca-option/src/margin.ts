export type CollateralLeg = {
  long: boolean;
  call: boolean;
  strike: number;
  /** Premium per share. */
  basis: number;
  /** Absolute contract count. */
  size: number;
};

function requirement(leg: CollateralLeg, spot: number): number {
  const outOfMoney = leg.call ? Math.max(leg.strike - spot, 0) : Math.max(spot - leg.strike, 0);
  return Math.max(leg.basis + 0.2 * spot - outOfMoney, leg.basis + 0.1 * leg.strike);
}

function naked(legs: readonly CollateralLeg[], spot: number): number {
  return legs.reduce((sum, leg) => sum + (requirement(leg, spot) - leg.basis) * 100 * leg.size, 0);
}

/** Dollar collateral. Null when this structure has no quoted margin. */
export function strategyCollateral(
  kind: string,
  legs: readonly CollateralLeg[],
  spot: number,
): number | null {
  if (legs.length === 0) {
    return null;
  }
  if (kind === 'short-call' || kind === 'short-calls' || kind === 'short-put' || kind === 'short-puts') {
    return naked(legs, spot);
  }
  if (kind === 'cash-secured-put') {
    const leg = legs[0];
    return leg == null ? null : leg.strike * 100 * leg.size;
  }
  if (kind === 'bear-call-spread' || kind === 'bull-put-spread') {
    const long = legs.find((leg) => leg.long);
    const short = legs.find((leg) => !leg.long);
    if (long == null || short == null) {
      return null;
    }
    return Math.abs(long.strike * 100 * long.size - short.strike * 100 * short.size);
  }
  if (kind === 'short-straddle' || kind === 'short-strangle') {
    const call = legs.find((leg) => leg.call);
    const put = legs.find((leg) => !leg.call);
    if (call == null || put == null) {
      return null;
    }
    const callRequirement = requirement(call, spot) * call.size;
    const putRequirement = requirement(put, spot) * put.size;
    const premium = legs.reduce((sum, leg) => sum + leg.basis * leg.size, 0);
    const larger = callRequirement > putRequirement
      ? callRequirement + put.basis * put.size
      : putRequirement + call.basis * call.size;
    return (larger - premium) * 100;
  }
  if (kind === 'call-ratio-spread' || kind === 'put-ratio-spread') {
    const long = legs.find((leg) => leg.long);
    const short = legs.find((leg) => !leg.long);
    if (long == null || short == null) {
      return null;
    }
    return (requirement(short, spot) - short.basis) * (short.size - long.size) * 100;
  }
  if (
    kind === 'long-synthetic-future'
    || kind === 'short-synthetic-future'
    || kind === 'long-combo'
    || kind === 'short-combo'
  ) {
    const short = legs.find((leg) => !leg.long);
    if (short == null) {
      return null;
    }
    return (requirement(short, spot) - short.basis) * 100 * short.size;
  }
  return null;
}
