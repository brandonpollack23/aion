import { DateTime } from "luxon";
import type { GCalEvent, Reminder } from "../domain/gcalEvent.ts";
import { getDisplayTitle } from "../domain/gcalEvent.ts";
import { formatDayShort, formatTime, getEventStart } from "../domain/time.ts";

export interface NotificationCandidate {
  key: string;
  title: string;
  message: string;
}

function selfResponseStatus(event: GCalEvent | undefined): string | undefined {
  return event?.attendees?.find((attendee) => attendee.self)?.responseStatus;
}

export function isPendingInvite(event: GCalEvent | undefined): boolean {
  return selfResponseStatus(event) === "needsAction";
}

function previousEventForChange(
  changedEvent: GCalEvent,
  previousEvents: Record<string, GCalEvent>
): GCalEvent | undefined {
  if (previousEvents[changedEvent.id]) return previousEvents[changedEvent.id];

  return Object.values(previousEvents).find((event) => {
    if (event.accountEmail !== changedEvent.accountEmail) return false;
    if (event.calendarId !== changedEvent.calendarId) return false;
    if (event.id === changedEvent.id) return true;
    return event.id.endsWith(`:${changedEvent.id}`);
  });
}

export function buildPendingInviteNotifications(
  changedEvents: GCalEvent[],
  previousEvents: Record<string, GCalEvent>,
  tz: string
): NotificationCandidate[] {
  const notifications: NotificationCandidate[] = [];

  for (const event of changedEvents) {
    if (!isPendingInvite(event)) continue;

    const previous = previousEventForChange(event, previousEvents);
    if (isPendingInvite(previous)) continue;

    const start = getEventStart(event, tz);
    const organizer = event.attendees?.find((attendee) => attendee.organizer);
    const organizerName =
      organizer?.displayName ||
      organizer?.email ||
      event.organizer?.displayName ||
      event.organizer?.email ||
      "Unknown organizer";

    notifications.push({
      key: `invite:${event.id}`,
      title: "New calendar invite",
      message: `${getDisplayTitle(event)} - ${formatDayShort(start)} ${formatTime(start)} from ${organizerName}`,
    });
  }

  return notifications;
}

function eventStartKey(event: GCalEvent, tz: string): string {
  return getEventStart(event, tz).toISO() || event.start.dateTime || event.start.date || "";
}

function shouldSkipReminderEvent(event: GCalEvent): boolean {
  if (event.status === "cancelled") return true;
  return selfResponseStatus(event) === "declined";
}

export function providerReminderOverrides(event: GCalEvent): Reminder[] {
  return event.reminders?.overrides?.filter((reminder) => {
    return (
      (reminder.method === "popup" || reminder.method === "email") &&
      Number.isFinite(reminder.minutes)
    );
  }) || [];
}

export function buildDueReminderNotifications(
  events: GCalEvent[],
  now: DateTime,
  startedAt: DateTime,
  sentKeys: Set<string>,
  tz: string
): NotificationCandidate[] {
  const notifications: NotificationCandidate[] = [];

  for (const event of events) {
    if (shouldSkipReminderEvent(event)) continue;

    const reminders = providerReminderOverrides(event);
    if (reminders.length === 0) continue;

    const start = getEventStart(event, tz);
    const startKey = eventStartKey(event, tz);

    for (const reminder of reminders) {
      const triggerAt = start.minus({ minutes: reminder.minutes });
      const key = `event:${event.id}:${startKey}:${reminder.minutes}`;

      if (sentKeys.has(key)) continue;
      if (triggerAt <= startedAt || triggerAt > now) continue;

      notifications.push({
        key,
        title: "Calendar reminder",
        message: `${getDisplayTitle(event)} - ${formatDayShort(start)} ${formatTime(start)}`,
      });
    }
  }

  return notifications;
}
