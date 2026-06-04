import { describe, expect, test } from "bun:test";
import { DateTime } from "luxon";
import { getMonthGridDays, isDayInMonthGrid, layoutMonth, navigateMonthSelection, type MonthNavigationAction } from "../src/domain/monthLayout.ts";
import type { GCalEvent } from "../src/domain/gcalEvent.ts";

function timedEvent(id: string, start: string, end: string, status: GCalEvent["status"] = "confirmed"): GCalEvent {
  return {
    id,
    summary: id,
    status,
    eventType: "default",
    start: { dateTime: start, timeZone: "UTC" },
    end: { dateTime: end, timeZone: "UTC" },
  };
}

function allDayEvent(id: string, start: string, end: string, status: GCalEvent["status"] = "confirmed"): GCalEvent {
  return {
    id,
    summary: id,
    status,
    eventType: "default",
    start: { date: start },
    end: { date: end },
  };
}

function navigate(displayedMonth: string, selectedDay: string, action: MonthNavigationAction): [string, string] {
  const result = navigateMonthSelection(
    DateTime.fromISO(displayedMonth),
    DateTime.fromISO(selectedDay),
    action
  );

  return [
    result.displayedMonth.toISODate() ?? "",
    result.selectedDay.toISODate() ?? "",
  ];
}

describe("month layout", () => {
  test("builds a Sunday-first 6x7 grid with adjacent month days", () => {
    const days = getMonthGridDays(DateTime.fromISO("2026-06-15"));

    expect(days).toHaveLength(42);
    expect(days[0]?.toISODate()).toBe("2026-05-31");
    expect(days[41]?.toISODate()).toBe("2026-07-11");
    expect(days.every((day, index) => index % 7 !== 0 || day.weekday === 7)).toBe(true);
  });

  test("includes current month start and end for months starting on different weekdays", () => {
    for (let month = 1; month <= 12; month++) {
      const anchor = DateTime.fromObject({ year: 2026, month, day: 15 });
      const days = getMonthGridDays(anchor);
      const isoDays = days.map((day) => day.toISODate());

      expect(days).toHaveLength(42);
      expect(days[0]?.weekday).toBe(7);
      expect(isoDays).toContain(anchor.startOf("month").toISODate());
      expect(isoDays).toContain(anchor.endOf("month").startOf("day").toISODate());
    }
  });

  test("checks whether a selected day is already visible in the displayed month grid", () => {
    const june = DateTime.fromISO("2026-06-15");

    expect(isDayInMonthGrid(june, DateTime.fromISO("2026-05-31"))).toBe(true);
    expect(isDayInMonthGrid(june, DateTime.fromISO("2026-07-11"))).toBe(true);
    expect(isDayInMonthGrid(june, DateTime.fromISO("2026-05-30"))).toBe(false);
    expect(isDayInMonthGrid(june, DateTime.fromISO("2026-07-12"))).toBe(false);
  });

  test("moves month selection with vim day and week directions without reanchoring inside the visible grid", () => {
    expect(navigate("2026-06-01", "2026-06-10", "prevDay")).toEqual(["2026-06-01", "2026-06-09"]);
    expect(navigate("2026-06-01", "2026-06-10", "nextDay")).toEqual(["2026-06-01", "2026-06-11"]);
    expect(navigate("2026-06-01", "2026-06-10", "prevWeek")).toEqual(["2026-06-01", "2026-06-03"]);
    expect(navigate("2026-06-01", "2026-06-10", "nextWeek")).toEqual(["2026-06-01", "2026-06-17"]);
  });

  test("moves displayed month only after day navigation leaves the visible grid", () => {
    expect(navigate("2026-06-01", "2026-05-31", "prevDay")).toEqual(["2026-05-01", "2026-05-30"]);
    expect(navigate("2026-06-01", "2026-07-11", "nextDay")).toEqual(["2026-07-01", "2026-07-12"]);
    expect(navigate("2026-06-01", "2026-06-30", "nextDay")).toEqual(["2026-06-01", "2026-07-01"]);
  });

  test("moves month selection with H and L equivalents in both directions", () => {
    expect(navigate("2026-06-01", "2026-06-15", "prevMonth")).toEqual(["2026-05-01", "2026-05-15"]);
    expect(navigate("2026-06-01", "2026-06-15", "nextMonth")).toEqual(["2026-07-01", "2026-07-15"]);
  });

  test("clamps month jumps to the last valid day of the target month", () => {
    expect(navigate("2026-03-01", "2026-03-31", "prevMonth")).toEqual(["2026-02-01", "2026-02-28"]);
    expect(navigate("2026-01-01", "2026-01-31", "nextMonth")).toEqual(["2026-02-01", "2026-02-28"]);
  });

  test("buckets visible events per day using existing day layout rules", () => {
    const weeks = layoutMonth([
      allDayEvent("all-day", "2026-06-10", "2026-06-11"),
      allDayEvent("multi-day", "2026-06-10", "2026-06-12"),
      timedEvent("timed", "2026-06-10T15:00:00Z", "2026-06-10T16:00:00Z"),
      timedEvent("cancelled", "2026-06-10T17:00:00Z", "2026-06-10T18:00:00Z", "cancelled"),
    ], DateTime.fromISO("2026-06-01"), "UTC");

    const days = weeks.flat();
    const june10 = days.find((day) => day.day.toISODate() === "2026-06-10");
    const june11 = days.find((day) => day.day.toISODate() === "2026-06-11");
    const june12 = days.find((day) => day.day.toISODate() === "2026-06-12");

    expect(june10?.events.map((event) => event.id)).toEqual(["all-day", "multi-day", "timed"]);
    expect(june11?.events.map((event) => event.id)).toEqual(["multi-day"]);
    expect(june12?.events.map((event) => event.id)).toEqual([]);
  });
});
