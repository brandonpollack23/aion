import React, { useEffect, useRef } from "react";
import { DateTime } from "luxon";
import { useAtomValue } from "jotai";
import { filteredEventsArrayAtom, timezoneAtom } from "../state/atoms.ts";
import { getConfig } from "../config/config.ts";
import type { GCalEvent } from "../domain/gcalEvent.ts";
import { buildDueReminderNotifications } from "./reminders.ts";
import { sendDesktopNotification } from "./native.ts";

export function DesktopNotificationHost() {
  const events = useAtomValue(filteredEventsArrayAtom);
  const tz = useAtomValue(timezoneAtom);
  const eventsRef = useRef<GCalEvent[]>(events);
  const tzRef = useRef(tz);
  const sentKeysRef = useRef(new Set<string>());
  const startedAtRef = useRef(DateTime.now());

  useEffect(() => {
    eventsRef.current = events;
  }, [events]);

  useEffect(() => {
    tzRef.current = tz;
  }, [tz]);

  useEffect(() => {
    const config = getConfig()["desktop-notifications"];
    if (!config.enabled || !config.event_reminders) return;

    const pollMs = config.poll_seconds * 1000;

    const checkReminders = async () => {
      const notifications = buildDueReminderNotifications(
        eventsRef.current,
        DateTime.now(),
        startedAtRef.current,
        sentKeysRef.current,
        tzRef.current
      );

      for (const notification of notifications) {
        sentKeysRef.current.add(notification.key);
        await sendDesktopNotification(notification, {
          enabled: config.enabled,
          terminal_bell_fallback: config.terminal_bell_fallback,
        });
      }
    };

    const timer = setInterval(() => {
      checkReminders().catch(() => {});
    }, pollMs);

    return () => clearInterval(timer);
  }, []);

  return null;
}
