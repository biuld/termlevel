# Notifications architecture

The title-bar bell opens the in-app notification history and remains available when the sidebar is collapsed. `src/notifications.rs` is the sole owner of the host process's in-memory history, independent of session-view mounting and session closure. The history is not written to the workspace document and is cleared when the application restarts.

## Limits

- The store keeps at most the latest 100 records.
- Each record carries its source, level, session ID, time and unread state.
- The same content from the same session within two seconds merges into one record with a count and is marked unread again.
- Titles are limited to 160 characters and bodies to 4,096 characters. Control characters are filtered out while newlines and tabs are preserved.

## Sources

| Source | Behavior |
| --- | --- |
| OSC 9 / OSC 777 | The Ghostty safe wrapper registers the official desktop-notification callback, copies owned title and body text, and passes it through a terminal BackendEvent to the Session, which writes the history. These are the notification sequences covered by the libghostty-vt 1.0.1 callback; OSC 99 and system desktop notifications are not supported. |
| OSC 7501 transitions | Each record entering a blocked / error transition writes an extra notification carrying the record identity and generation. Working, done and OSC 9;4 progress do not enter the history. Transitions are decided per record, not by whether the aggregated icon changes. |
| BEL | The existing sidebar marker is retained, and the terminal bell is also recorded in the history. |
| Application errors | Session start / I/O / encoding errors, input-rejection warnings, non-zero process exits and application folder-dialog errors enter the same history with an `app` source. |
| Fetch / Pull | Completion is written as a success or failure record by the application-level notification center and shows a toast in the top-right corner for 5 seconds. Switching views during the operation does not lose the result, and closing the toast does not remove the history record. |

The terminal no longer shows a status bar at the bottom; detailed errors and input rejections appear only in the notification panel. Clearing the history does not clear session errors or end a process. Status semantics, including which transitions produce notifications and what acknowledgement means, belong to [OSC integration and terminal status](../design/osc-status.md).

## Polling

`ui/notifications/notifications.ts` lives at the application root and checks the history revision every 250 ms. It reads bounded low-frequency data only when the revision changes and does not depend on any pane's refresh or event re-delivery, so hidden sessions can still notify.

## Panel behavior

`NotificationPanel.tsx` handles anchoring, the measured height of the first three entries and scrolling. `NotificationRow.tsx` owns message hierarchy, semantic icons, expansion and inline actions, and selects an existing tab through a session-ID callback supplied by the application; it never creates a session.

For OSC 7501 history, the `program` metadata carries `kind`, `app`, `title` and `message` from records sanitized by Rust. The UI consumes the actual message and does not infer a task type from the generated notification title.

Opening the panel marks the history as read. The panel supports removing individual records, clearing the history and returning to the source; if the source has been closed, its content remains but jumping to it is disabled. The history displays at most three entries by content height, limited by the available window height; additional content scrolls with a hint and a divider. While the panel is open it blocks terminal input, and Escape or an outside click closes it and restores input.

The visual treatment of notification rows is described in [UI and theme](../design/ui.md).

## Preview boundary

Product preview does not read or modify the real notification history. Preview Git and terminal sessions show unavailable messages, and no background Git tasks or terminal sessions run, so no new operational notifications are produced.
