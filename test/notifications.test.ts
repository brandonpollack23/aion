import { describe, expect, test } from "bun:test";
import { DateTime } from "luxon";
import type { NotificationOptions } from "node-notifier";
import type { GCalEvent } from "../src/domain/gcalEvent.ts";
import { parseICalendar } from "../src/api/ical.ts";
import {
  buildDueReminderNotifications,
  buildPendingInviteNotifications,
} from "../src/notifications/reminders.ts";
import { sendDesktopNotification } from "../src/notifications/native.ts";

type ReminderOverrides = NonNullable<NonNullable<GCalEvent["reminders"]>["overrides"]>;

function event(overrides: ReminderOverrides = []): GCalEvent {
  return {
    id: "evt-1",
    summary: "Planning",
    status: "confirmed",
    eventType: "default",
    start: { dateTime: "2026-06-04T10:00:00Z", timeZone: "UTC" },
    end: { dateTime: "2026-06-04T10:30:00Z", timeZone: "UTC" },
    reminders: { useDefault: false, overrides },
  };
}

describe("desktop notification reminder selection", () => {
  test("uses provider reminder overrides", () => {
    const now = DateTime.fromISO("2026-06-04T09:50:05Z");
    const startedAt = DateTime.fromISO("2026-06-04T09:00:00Z");
    const notifications = buildDueReminderNotifications(
      [event([{ method: "popup", minutes: 10 }])],
      now,
      startedAt,
      new Set(),
      "UTC"
    );

    expect(notifications).toHaveLength(1);
    expect(notifications[0]!.title).toBe("Calendar reminder");
  });

  test("does not use an app fallback when provider reminders are absent", () => {
    const now = DateTime.fromISO("2026-06-04T09:50:05Z");
    const startedAt = DateTime.fromISO("2026-06-04T09:00:00Z");
    const notifications = buildDueReminderNotifications(
      [event([])],
      now,
      startedAt,
      new Set(),
      "UTC"
    );

    expect(notifications).toHaveLength(0);
  });

  test("skips declined and cancelled events", () => {
    const now = DateTime.fromISO("2026-06-04T09:50:05Z");
    const startedAt = DateTime.fromISO("2026-06-04T09:00:00Z");
    const declined = {
      ...event([{ method: "popup", minutes: 10 }]),
      id: "declined",
      attendees: [{ email: "me@example.com", self: true, responseStatus: "declined" as const }],
    };
    const cancelled = {
      ...event([{ method: "popup", minutes: 10 }]),
      id: "cancelled",
      status: "cancelled" as const,
    };

    const notifications = buildDueReminderNotifications(
      [declined, cancelled],
      now,
      startedAt,
      new Set(),
      "UTC"
    );

    expect(notifications).toHaveLength(0);
  });
});

describe("desktop notification invite selection", () => {
  test("fires when an invite newly needs action", () => {
    const changed = {
      ...event(),
      id: "invite-1",
      attendees: [{ email: "me@example.com", self: true, responseStatus: "needsAction" as const }],
    };

    const notifications = buildPendingInviteNotifications([changed], {}, "UTC");

    expect(notifications).toHaveLength(1);
    expect(notifications[0]!.title).toBe("New calendar invite");
  });

  test("does not fire for an invite already pending locally", () => {
    const changed = {
      ...event(),
      id: "raw-id",
      accountEmail: "me@example.com",
      calendarId: "primary",
      attendees: [{ email: "me@example.com", self: true, responseStatus: "needsAction" as const }],
    };
    const previous = {
      "me@example.com:primary:raw-id": {
        ...changed,
        id: "me@example.com:primary:raw-id",
      },
    };

    const notifications = buildPendingInviteNotifications([changed], previous, "UTC");

    expect(notifications).toHaveLength(0);
  });
});

describe("CalDAV VALARM parsing", () => {
  test("parses relative and absolute VALARM triggers", () => {
    const events = parseICalendar(`BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:abc
DTSTART:20260604T100000Z
DTEND:20260604T110000Z
SUMMARY:CalDAV Event
BEGIN:VALARM
ACTION:DISPLAY
TRIGGER:-PT10M
END:VALARM
BEGIN:VALARM
ACTION:EMAIL
TRIGGER;VALUE=DATE-TIME:20260604T093000Z
END:VALARM
END:VEVENT
END:VCALENDAR`);

    expect(events).toHaveLength(1);
    expect(events[0]!.reminders?.overrides).toEqual([
      { method: "popup", minutes: 10 },
      { method: "email", minutes: 30 },
    ]);
  });
});

describe("native notification fallback", () => {
  test("sends desktop notifications with the Aion logo and supported native sound option", async () => {
    let sentOptions: NotificationOptions | undefined;

    const delivered = await sendDesktopNotification(
      { title: "Aion", message: "Test" },
      { enabled: true, terminal_bell_fallback: false },
      (options, callback) => {
        sentOptions = options;
        callback?.(null, "delivered");
      }
    );

    expect(delivered).toBe(true);
    expect(sentOptions!.icon).toEndWith("images/logo.png");
    expect(sentOptions!.sound).toBe(process.platform === "darwin" || process.platform === "win32" ? true : undefined);
  });

  test("emits terminal bell when native notifier fails and fallback is enabled", async () => {
    let bells = 0;

    const delivered = await sendDesktopNotification(
      { title: "Aion", message: "Test" },
      { enabled: true, terminal_bell_fallback: true },
      (_options, callback) => callback?.(new Error("no notifier")),
      () => {
        bells++;
      }
    );

    expect(delivered).toBe(false);
    expect(bells).toBe(1);
  });
});
