declare module "node-notifier" {
  export interface NotificationOptions {
    title?: string;
    subtitle?: string;
    message: string;
    appID?: string;
    sound?: boolean;
    wait?: boolean;
    timeout?: number;
  }

  export type NotifyCallback = (
    error: Error | null,
    response?: string | boolean,
    metadata?: unknown
  ) => void;

  export interface Notifier {
    notify(options: NotificationOptions, callback?: NotifyCallback): void;
  }

  const notifier: Notifier;
  export default notifier;
}
