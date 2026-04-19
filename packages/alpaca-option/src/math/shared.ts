export function roundToFixtureYears(years: number): number {
  const scaled = years * 365;
  const base = Math.floor(scaled);
  const fraction = scaled - base;

  const roundedDays = fraction < 0.5
    ? base
    : fraction > 0.5
      ? base + 1
      : base % 2 === 0
        ? base
        : base + 1;

  return roundedDays / 365;
}
