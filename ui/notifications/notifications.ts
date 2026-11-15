import { cleanup, signal } from "nsh";
import { Notifications, type Notice } from "termlevel/native";

/** One app-level subscription; preview never reads the real notification history. */
export function createNotificationCenter(isPreview: boolean, isOpen: () => boolean) {
  const entries = signal<Notice[]>([]);
  let revision = -1;
  function refresh(force = false) {
    if (isPreview) return;
    let next = Notifications.revision();
    if (!force && revision === next) return;
    if (isOpen()) Notifications.readAll();
    next = Notifications.revision();
    entries.set(Notifications.list());
    revision = next;
  }
  if (!isPreview) {
    refresh();
    const timer = setInterval(refresh, 250);
    cleanup(() => clearInterval(timer));
  }
  return {
    entries,
    unread: () => entries().filter(entry => entry.unread).length,
    read: () => { if (!isPreview) Notifications.readAll(); refresh(true); },
    clear: () => { if (!isPreview) Notifications.clear(); entries.set([]); },
    dismiss: (id: string) => {
      if (!isPreview) Notifications.dismiss(id);
      entries.update(all => all.filter(entry => entry.id !== id));
    },
    report: (title: string, body: string) => {
      if (!isPreview) { Notifications.report(title, body); refresh(true); }
    },
  };
}
