import { DateTime } from "luxon";
import type { GCalEvent } from "./gcalEvent.ts";
import { layoutDay, getChronologicalEvents } from "./layout.ts";
import { getLocalTimezone } from "./time.ts";

export interface MonthDayLayout {
  day: DateTime;
  inCurrentMonth: boolean;
  events: GCalEvent[];
}

export type MonthWeekLayout = MonthDayLayout[];

export type MonthNavigationAction =
  | "prevDay"
  | "nextDay"
  | "prevWeek"
  | "nextWeek"
  | "prevMonth"
  | "nextMonth"
  | "firstDay"
  | "lastDay";

function getSundayStart(day: DateTime): DateTime {
  const monthStart = day.startOf("month").startOf("day");
  return monthStart.minus({ days: monthStart.weekday % 7 });
}

export function getMonthGridDays(month: DateTime): DateTime[] {
  const firstVisibleDay = getSundayStart(month);
  return Array.from({ length: 42 }, (_, index) =>
    firstVisibleDay.plus({ days: index }).startOf("day")
  );
}

export function isDayInMonthGrid(month: DateTime, day: DateTime): boolean {
  const days = getMonthGridDays(month);
  const firstDay = days[0];
  const lastDay = days[days.length - 1];
  if (!firstDay || !lastDay) return false;

  const target = day.startOf("day");
  return target >= firstDay && target <= lastDay;
}

export function getMonthGridBounds(month: DateTime): { firstDay: DateTime; lastDay: DateTime } {
  const days = getMonthGridDays(month);
  const firstDay = days[0];
  const lastDay = days[days.length - 1];
  if (!firstDay || !lastDay) {
    const fallback = month.startOf("month").startOf("day");
    return { firstDay: fallback, lastDay: fallback };
  }

  return { firstDay, lastDay };
}

function clampDayInMonth(month: DateTime, dayOfMonth: number): DateTime {
  return month.set({ day: Math.min(dayOfMonth, month.daysInMonth ?? dayOfMonth) }).startOf("day");
}

function anchorForSelectedDay(displayedMonth: DateTime, selectedDay: DateTime): DateTime {
  if (isDayInMonthGrid(displayedMonth, selectedDay)) {
    return displayedMonth.startOf("month");
  }

  const { firstDay, lastDay } = getMonthGridBounds(displayedMonth);
  if (selectedDay < firstDay) {
    return displayedMonth.minus({ months: 1 }).startOf("month");
  }
  if (selectedDay > lastDay) {
    return displayedMonth.plus({ months: 1 }).startOf("month");
  }

  return selectedDay.startOf("month");
}

export function navigateMonthSelection(
  displayedMonth: DateTime,
  selectedDay: DateTime,
  action: MonthNavigationAction
): { displayedMonth: DateTime; selectedDay: DateTime } {
  const month = displayedMonth.startOf("month");
  const day = selectedDay.startOf("day");

  switch (action) {
    case "prevDay": {
      const nextDay = day.minus({ days: 1 });
      return { selectedDay: nextDay, displayedMonth: anchorForSelectedDay(month, nextDay) };
    }
    case "nextDay": {
      const nextDay = day.plus({ days: 1 });
      return { selectedDay: nextDay, displayedMonth: anchorForSelectedDay(month, nextDay) };
    }
    case "prevWeek": {
      const nextDay = day.minus({ days: 7 });
      return { selectedDay: nextDay, displayedMonth: anchorForSelectedDay(month, nextDay) };
    }
    case "nextWeek": {
      const nextDay = day.plus({ days: 7 });
      return { selectedDay: nextDay, displayedMonth: anchorForSelectedDay(month, nextDay) };
    }
    case "prevMonth": {
      const nextMonth = month.minus({ months: 1 }).startOf("month");
      return { selectedDay: clampDayInMonth(nextMonth, day.day), displayedMonth: nextMonth };
    }
    case "nextMonth": {
      const nextMonth = month.plus({ months: 1 }).startOf("month");
      return { selectedDay: clampDayInMonth(nextMonth, day.day), displayedMonth: nextMonth };
    }
    case "firstDay":
      return { selectedDay: month, displayedMonth: month };
    case "lastDay":
      return { selectedDay: month.endOf("month").startOf("day"), displayedMonth: month };
  }
}

export function layoutMonth(
  events: GCalEvent[],
  month: DateTime,
  tz: string = getLocalTimezone()
): MonthWeekLayout[] {
  const currentMonth = month.startOf("month");
  const days = getMonthGridDays(month);
  const dayLayouts = days.map((day): MonthDayLayout => {
    const dayLayout = layoutDay(events, day, tz);
    return {
      day,
      inCurrentMonth: day.hasSame(currentMonth, "month"),
      events: getChronologicalEvents(dayLayout),
    };
  });

  const weeks: MonthWeekLayout[] = [];
  for (let i = 0; i < dayLayouts.length; i += 7) {
    weeks.push(dayLayouts.slice(i, i + 7));
  }
  return weeks;
}
