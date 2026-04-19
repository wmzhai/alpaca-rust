export type NyDateString = string;
export type NyTimestampString = string;
export type ComparableTimeInput = NyDateString | NyTimestampString;
export type HhmmString = string;
export type WeekdayCode = 'mon' | 'tue' | 'wed' | 'thu' | 'fri' | 'sat' | 'sun';
export type MarketSession = 'premarket' | 'regular' | 'after_hours' | 'closed';
export type DayCountBasis = 'ACT/365' | 'ACT/365.25' | 'ACT/360';

export type DurationSign = -1 | 0 | 1;

export interface MarketHours {
  date: NyDateString;
  is_trading_date: boolean;
  is_early_close: boolean;
  premarket_open: HhmmString | null;
  regular_open: HhmmString | null;
  regular_close: HhmmString | null;
  after_hours_close: HhmmString | null;
}

export interface TradingDayInfo {
  date: NyDateString;
  is_trading_date: boolean;
  is_market_holiday: boolean;
  is_early_close: boolean;
  market_hours: MarketHours;
}

export interface DurationParts {
  sign: DurationSign;
  total_seconds: number;
  days: number;
  hours: number;
  minutes: number;
  seconds: number;
}

export interface DateRange {
  start_date: NyDateString;
  end_date: NyDateString;
}

export interface TimestampParts {
  date: NyDateString;
  timestamp: NyTimestampString;
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  second: number;
  hhmm: number;
  hhmm_string: HhmmString;
  weekday_from_sunday: number;
}
