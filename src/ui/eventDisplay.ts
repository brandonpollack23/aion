import { DateTime } from "luxon";
import type { GCalEvent, ResponseStatus } from "../domain/gcalEvent.ts";
import { getEventEnd } from "../domain/time.ts";
import { theme } from "./theme.ts";

export function getSelfResponseStatus(event: GCalEvent): ResponseStatus | undefined {
  const selfAttendee = event.attendees?.find((a) => a.self);
  return selfAttendee?.responseStatus;
}

export function getAttendanceIndicator(status: ResponseStatus | undefined, hasAttendees: boolean): string {
  switch (status) {
    case "accepted": return "✓ ";
    case "declined": return "✗ ";
    case "tentative": return "? ";
    case "needsAction": return "! ";
    default: return hasAttendees ? "! " : "";
  }
}

export function getAttendanceColor(status: ResponseStatus | undefined, hasAttendees: boolean): string | undefined {
  switch (status) {
    case "accepted": return theme.status.accepted;
    case "declined": return theme.status.declined;
    case "tentative": return theme.status.tentative;
    case "needsAction": return theme.accent.warning;
    default: return hasAttendees ? theme.accent.warning : undefined;
  }
}

export function isEventPast(event: GCalEvent, tz: string): boolean {
  const end = getEventEnd(event, tz);
  return end < DateTime.now();
}

export function getEventColor(event: GCalEvent, calendarColorMap: Record<string, string>): string {
  const key = `${event.accountEmail}:${event.calendarId}`;
  const calendarColor = calendarColorMap[key] || "#4285f4";

  if (calendarColor !== "#4285f4") return calendarColor;

  switch (event.eventType) {
    case "outOfOffice": return theme.eventType.outOfOffice;
    case "focusTime": return theme.eventType.focusTime;
    case "birthday": return theme.eventType.birthday;
    default: return calendarColor;
  }
}
