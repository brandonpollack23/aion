import notifier from "node-notifier";
import { appLogger } from "../lib/logger.ts";

export interface DesktopNotificationPayload {
  title?: string;
  subtitle?: string;
  message: string;
}

export interface DesktopNotificationOptions {
  enabled: boolean;
  terminal_bell_fallback: boolean;
}

type NotifyBackend = typeof notifier.notify;
type BellWriter = () => void;

function truncate(value: string, maxLength: number): string {
  if (value.length <= maxLength) return value;
  return `${value.slice(0, maxLength - 1)}...`;
}

export function writeTerminalBell(): void {
  try {
    process.stdout.write("\x07");
  } catch (error) {
    appLogger.warn("Failed to write terminal bell", { error });
  }
}

export async function sendDesktopNotification(
  payload: DesktopNotificationPayload,
  options: DesktopNotificationOptions,
  backend: NotifyBackend = notifier.notify.bind(notifier),
  writeBell: BellWriter = writeTerminalBell
): Promise<boolean> {
  if (!options.enabled) return false;

  const title = truncate(payload.title || "Aion", 80);
  const message = truncate(payload.message, 240);
  const subtitle = payload.subtitle ? truncate(payload.subtitle, 120) : undefined;

  try {
    const delivered = await new Promise<boolean>((resolve) => {
      backend(
        {
          title,
          subtitle,
          message,
          appID: "Aion",
          timeout: 10,
        },
        (error, response) => {
          if (error) {
            appLogger.warn("Desktop notification failed", { error });
            resolve(false);
            return;
          }

          if (response === false || response === "failed") {
            appLogger.warn("Desktop notification was not delivered", { response });
            resolve(false);
            return;
          }

          resolve(true);
        }
      );
    });

    if (!delivered && options.terminal_bell_fallback) {
      writeBell();
    }

    return delivered;
  } catch (error) {
    appLogger.warn("Desktop notification threw", { error });
    if (options.terminal_bell_fallback) {
      writeBell();
    }
    return false;
  }
}
