import { browser, calendar, clock, display, expiration, range } from './index';
import type {
  DateRange,
  DurationParts,
  MarketHours,
  NyDateString,
  NyTimestampString,
  WeekdayCode,
} from './types';

const nyTimestamp: NyTimestampString = clock.now();
const nyDate: NyDateString = clock.today();
const parsedDate: NyDateString = clock.parseDate('2025-02-06');
const parsedTimestamp: NyTimestampString = clock.parseTimestamp('2025-02-06 09:30:00');
const nextTradingDate: NyDateString = calendar.addTradingDays('2025-02-06', 1);
const nextDate: NyDateString = range.addDays('2025-02-06', 1);
const weeklyDates: NyDateString[] = range.weeklyLastTradingDates('2025-02-03', '2025-02-28');
const weekRange: DateRange = range.calendarWeekRange('2025-02-06');
const hours: MarketHours = calendar.marketHoursForDate('2025-02-06');
const expirationClose: NyTimestampString = expiration.close('2025-02-21');
const weekdayCode: WeekdayCode = display.weekdayCode('2025-02-06');
const weekdayZh: '一' | '二' | '三' | '四' | '五' | '六' | '日' = display.weekdayShortZh('2025-02-06');
const durationParts: DurationParts = display.relativeDurationParts('2025-02-06 09:30:00', '2025-02-06 16:00:00');
const browserDate: NyDateString = browser.dateObjectToNyDate(new Date('2025-02-06T00:00:00Z'));

void nyTimestamp;
void nyDate;
void parsedDate;
void parsedTimestamp;
void nextTradingDate;
void nextDate;
void weeklyDates;
void weekRange;
void hours;
void expirationClose;
void weekdayCode;
void weekdayZh;
void durationParts;
void browserDate;
