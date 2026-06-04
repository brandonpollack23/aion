import React, { useMemo, useRef } from "react";
import { Box, Text, useApp, useInput, type Color } from "@semos-labs/glyph";
import { DateTime } from "luxon";
import { useAtom, useAtomValue, useSetAtom } from "jotai";
import {
  calendarColorMapAtom,
  filteredEventsArrayAtom,
  focusAtom,
  selectedDayAtom,
  selectedEventIdAtom,
  timezoneAtom,
  viewAnchorDayAtom,
} from "../state/atoms.ts";
import {
  initiateDeleteAtom,
  newEventAtom,
  openCommandAtom,
  openDetailsAtom,
  openEditDialogAtom,
  openGotoDialogAtom,
  openMeetWithDialogAtom,
  openNotificationsAtom,
  setViewModeAtom,
  toggleAllDayExpandedAtom,
  toggleCalendarSidebarAtom,
} from "../state/actions.ts";
import { layoutMonth, navigateMonthSelection, type MonthNavigationAction } from "../domain/monthLayout.ts";
import { formatTime, getEventStart, getLocalTimezone, isToday } from "../domain/time.ts";
import { getDisplayTitle, isAllDay, type GCalEvent } from "../domain/gcalEvent.ts";
import { handleKeyEvent } from "../keybinds/useKeybinds.tsx";
import { getAttendanceColor, getAttendanceIndicator, getEventColor, getSelfResponseStatus, isEventPast } from "./eventDisplay.ts";
import { theme } from "./theme.ts";

const WEEKDAY_LABELS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

function sameDay(a: DateTime, b: DateTime): boolean {
  return a.hasSame(b, "day");
}

function MonthKeybinds({
  dayEvents,
  displayedMonth,
}: {
  dayEvents: GCalEvent[];
  displayedMonth: DateTime;
}) {
  const [selectedDay, setSelectedDay] = useAtom(selectedDayAtom);
  const setViewAnchorDay = useSetAtom(viewAnchorDayAtom);
  const [selectedEventId, setSelectedEventId] = useAtom(selectedEventIdAtom);
  const setViewMode = useSetAtom(setViewModeAtom);
  const openDetails = useSetAtom(openDetailsAtom);
  const openEditDialog = useSetAtom(openEditDialogAtom);
  const deleteEvent = useSetAtom(initiateDeleteAtom);
  const openNotifications = useSetAtom(openNotificationsAtom);
  const newEvent = useSetAtom(newEventAtom);
  const toggleAllDayExpanded = useSetAtom(toggleAllDayExpandedAtom);
  const toggleCalendarSidebar = useSetAtom(toggleCalendarSidebarAtom);
  const openGotoDialog = useSetAtom(openGotoDialogAtom);
  const openMeetWithDialog = useSetAtom(openMeetWithDialogAtom);
  const openCommand = useSetAtom(openCommandAtom);

  const lastKeyRef = useRef<string>("");
  const lastKeyTimeRef = useRef<number>(0);

  const navigate = (action: MonthNavigationAction) => {
    const next = navigateMonthSelection(displayedMonth, selectedDay, action);
    setSelectedDay(next.selectedDay);
    setViewAnchorDay(next.displayedMonth);
    setSelectedEventId(null);
  };

  const moveEvent = (direction: "next" | "prev") => {
    if (dayEvents.length === 0) return;
    const currentIndex = selectedEventId
      ? dayEvents.findIndex((event) => event.id === selectedEventId)
      : -1;
    const delta = direction === "next" ? 1 : -1;
    const fallback = direction === "next" ? 0 : dayEvents.length - 1;
    const nextIndex = currentIndex === -1
      ? fallback
      : (currentIndex + delta + dayEvents.length) % dayEvents.length;
    const nextEvent = dayEvents[nextIndex];
    if (nextEvent) setSelectedEventId(nextEvent.id);
  };

  const openSelection = () => {
    const hasSelectedEvent = selectedEventId && dayEvents.some((event) => event.id === selectedEventId);
    if (hasSelectedEvent) {
      openDetails();
    } else {
      setViewMode("day");
    }
  };

  const handlers = useMemo(() => ({
    prevDay: () => navigate("prevDay"),
    nextDay: () => navigate("nextDay"),
    prevWeek: () => navigate("prevWeek"),
    nextWeek: () => navigate("nextWeek"),
    prevMonth: () => navigate("prevMonth"),
    nextMonth: () => navigate("nextMonth"),
    firstDay: () => navigate("firstDay"),
    lastDay: () => navigate("lastDay"),
    nextEvent: () => moveEvent("next"),
    prevEvent: () => moveEvent("prev"),
    jumpToNow: () => {
      const today = DateTime.now().startOf("day");
      setSelectedDay(today);
      setViewAnchorDay(today.startOf("month"));
      setSelectedEventId(null);
    },
    openSelection,
    editEvent: () => {
      if (selectedEventId) openEditDialog();
    },
    deleteEvent: () => {
      if (selectedEventId) deleteEvent();
    },
  }), [selectedDay, selectedEventId, dayEvents, displayedMonth, setSelectedDay, setViewAnchorDay, setSelectedEventId, openDetails, openEditDialog, deleteEvent, setViewMode]);

  const globalHandlers = useMemo(() => ({
    openNotifications: () => openNotifications(),
    newEvent: () => newEvent(),
    toggleAllDay: () => toggleAllDayExpanded(),
    toggleCalendars: () => toggleCalendarSidebar(),
    openGoto: () => openGotoDialog(),
    openMeetWith: () => openMeetWithDialog(),
    openCommand: () => openCommand(),
  }), [openNotifications, newEvent, toggleAllDayExpanded, toggleCalendarSidebar, openGotoDialog, openMeetWithDialog, openCommand]);

  // Always keep refs current so the stable useInput closure sees fresh handlers
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;
  const globalHandlersRef = useRef(globalHandlers);
  globalHandlersRef.current = globalHandlers;

  useInput((key) => {
    if (handleKeyEvent("global", key, globalHandlersRef.current)) return;

    const name = key.name?.toLowerCase();

    if (name === "g" && !key.shift) {
      const now = Date.now();
      if (lastKeyRef.current === "g" && now - lastKeyTimeRef.current < 500) {
        handlersRef.current.firstDay();
        lastKeyRef.current = "";
        return;
      }
      lastKeyRef.current = "g";
      lastKeyTimeRef.current = now;
      return;
    }

    if (name !== "g") lastKeyRef.current = "";
    handleKeyEvent("month", key, handlersRef.current);
  });

  return null;
}

function MonthEventLine({
  event,
  isSelected,
  calendarColorMap,
  tz,
}: {
  event: GCalEvent;
  isSelected: boolean;
  calendarColorMap: Record<string, string>;
  tz: string;
}) {
  const eventColor = getEventColor(event, calendarColorMap) as Color;
  const responseStatus = getSelfResponseStatus(event);
  const hasAttendees = (event.attendees?.length ?? 0) > 0;
  const attendanceIndicator = getAttendanceIndicator(responseStatus, hasAttendees);
  const attendanceColor = getAttendanceColor(responseStatus, hasAttendees) as Color | undefined;
  const shouldDim = responseStatus === "declined" || isEventPast(event, tz);
  const title = getDisplayTitle(event);
  const timePrefix = isAllDay(event) ? "" : `${formatTime(getEventStart(event, tz))} `;
  const bg = isSelected ? theme.selection.background : undefined;
  const textColor: Color = isSelected ? theme.selection.text : (shouldDim ? theme.text.dim : eventColor);

  return (
    <Box style={{ flexDirection: "row", bg, clip: true }}>
      <Text style={{ color: isSelected ? theme.selection.text : eventColor, dim: shouldDim }}>●</Text>
      <Text style={{ color: isSelected ? theme.selection.text : theme.text.dim }}>{isSelected ? "▸" : " "}</Text>
      {timePrefix && <Text style={{ color: isSelected ? theme.selection.text : theme.text.dim }}>{timePrefix}</Text>}
      {attendanceIndicator && <Text style={{ color: isSelected ? theme.selection.text : attendanceColor }}>{attendanceIndicator}</Text>}
      <Text style={{ color: textColor, bold: isSelected, dim: shouldDim }} wrap="truncate">{title}</Text>
    </Box>
  );
}

export function MonthView() {
  const { rows: terminalHeight } = useApp();
  const selectedDay = useAtomValue(selectedDayAtom);
  const viewAnchorDay = useAtomValue(viewAnchorDayAtom);
  const allEvents = useAtomValue(filteredEventsArrayAtom);
  const tz = useAtomValue(timezoneAtom);
  const focus = useAtomValue(focusAtom);
  const selectedEventId = useAtomValue(selectedEventIdAtom);
  const calendarColorMap = useAtomValue(calendarColorMapAtom);

  const displayedMonth = viewAnchorDay.startOf("month");

  const weeks = useMemo(
    () => layoutMonth(allEvents, displayedMonth, tz),
    [allEvents, displayedMonth, tz]
  );

  const selectedDayEvents = useMemo(() => {
    for (const week of weeks) {
      const match = week.find((day) => sameDay(day.day, selectedDay));
      if (match) return match.events;
    }
    return [];
  }, [weeks, selectedDay]);

  const isFocused = focus === "month";
  const cellHeight = Math.max(3, Math.floor((terminalHeight - 6) / 6));
  const eventRowsPerCell = Math.max(0, cellHeight - 2);

  return (
    <Box style={{ flexGrow: 1, height: "100%", flexDirection: "column", clip: true }}>
      {isFocused && <MonthKeybinds dayEvents={selectedDayEvents} displayedMonth={displayedMonth} />}

      <Box style={{ flexDirection: "row", justifyContent: "space-between" }}>
        <Text style={{ color: isFocused ? theme.accent.primary : theme.text.dim, bold: isFocused }}>
          {isFocused ? "▶ " : "  "}{displayedMonth.toFormat("MMMM yyyy")}
        </Text>
        <Text style={{ color: theme.text.dim }}>
          M:day  h/j/k/l:navigate  H/L:month
        </Text>
      </Box>

      <Box style={{ flexDirection: "row", height: 1 }}>
        <Text style={{ color: theme.text.dim, dim: true }}>│</Text>
        {WEEKDAY_LABELS.map((label, index) => (
          <React.Fragment key={label}>
            <Box style={{ flexGrow: 1, width: 0, paddingLeft: 1 }}>
              <Text style={{ color: index === 0 || index === 6 ? theme.text.weekend : theme.text.dim, bold: true }}>
                {label}
              </Text>
            </Box>
            <Text style={{ color: theme.text.dim, dim: true }}>│</Text>
          </React.Fragment>
        ))}
      </Box>

      {weeks.map((week, weekIndex) => (
        <Box key={weekIndex} style={{ flexDirection: "row", height: cellHeight }}>
          <Text style={{ color: theme.text.dim, dim: true }}>│</Text>
          {week.map((dayLayout) => {
            const day = dayLayout.day;
            const isSelectedDay = sameDay(day, selectedDay);
            const isCurrentDay = isToday(day);
            const isWeekend = day.weekday === 6 || day.weekday === 7;
            const selectedInCell = selectedEventId
              ? dayLayout.events.some((event) => event.id === selectedEventId)
              : false;
            const visibleEvents = dayLayout.events.slice(0, eventRowsPerCell);
            const hiddenCount = Math.max(0, dayLayout.events.length - visibleEvents.length);

            const dayColor = isSelectedDay && isFocused
              ? theme.selection.text
              : isCurrentDay
                ? theme.accent.success
                : !dayLayout.inCurrentMonth
                  ? theme.text.dim
                  : isWeekend
                    ? theme.text.weekend
                    : theme.text.primary;

            return (
              <React.Fragment key={day.toISODate()}>
                <Box
                  style={{
                    flexGrow: 1,
                    width: 0,
                    height: cellHeight,
                    flexDirection: "column",
                    paddingLeft: 1,
                    paddingRight: 1,
                    bg: isSelectedDay && isFocused ? theme.selection.background : undefined,
                    clip: true,
                  }}
                >
                  <Text style={{ color: dayColor as Color, bold: isSelectedDay || isCurrentDay }}>
                    {isSelectedDay && isFocused ? "▸" : " "}{day.day.toString().padStart(2, " ")}
                    {isCurrentDay && !isSelectedDay ? " •" : ""}
                  </Text>
                  {visibleEvents.map((event) => (
                    <MonthEventLine
                      key={event.id}
                      event={event}
                      isSelected={selectedEventId === event.id && isFocused}
                      calendarColorMap={calendarColorMap}
                      tz={tz || getLocalTimezone()}
                    />
                  ))}
                  {hiddenCount > 0 && (
                    <Text style={{ color: selectedInCell ? theme.accent.warning : theme.text.dim }}>
                      {selectedInCell && !visibleEvents.some((event) => event.id === selectedEventId) ? "▸" : " "}+{hiddenCount} more
                    </Text>
                  )}
                </Box>
                <Text style={{ color: theme.text.dim, dim: true }}>│</Text>
              </React.Fragment>
            );
          })}
        </Box>
      ))}
    </Box>
  );
}
