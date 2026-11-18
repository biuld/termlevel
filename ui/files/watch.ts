import type { FilesTransport, WatchNote } from "./types";

/** Poll expanded-directory watches. Missing transport methods leave refresh manual. */
export function startDirectoryWatch(input: {
  transport: FilesTransport;
  root: () => string;
  expanded: () => readonly string[];
  onChanged: (directories: string[]) => void;
  onFailures: (notes: WatchNote[]) => void;
}): () => void {
  const { watchOpen, watchSet, watchPoll, watchClose } = input.transport;
  if (!watchOpen || !watchSet || !watchPoll || !watchClose) return () => {};
  const id = watchOpen();
  let stopped = false;
  let setting = false;
  let applied = "";
  let timer: ReturnType<typeof setTimeout> | undefined;

  const sync = () => {
    if (stopped || setting) return;
    const root = input.root();
    const directories = ["", ...input.expanded()];
    const key = `${root}\0${directories.join("\0")}`;
    if (key === applied) return;
    setting = true;
    void watchSet(id, root, directories).then((notes) => {
      if (stopped || input.root() !== root) return;
      applied = key;
      input.onFailures(notes);
    }).catch(() => {
      applied = "";
    }).finally(() => {
      setting = false;
      if (stopped) return;
      const now = `${input.root()}\0${["", ...input.expanded()].join("\0")}`;
      if (now !== key) sync();
    });
  };

  const tick = () => {
    if (stopped) return;
    sync();
    try {
      const changed = watchPoll(id);
      if (changed.length) input.onChanged(changed);
    } catch {
      // The refresh button still reloads listings.
    }
    timer = setTimeout(tick, 200);
  };

  timer = setTimeout(tick, 0);
  return () => {
    stopped = true;
    if (timer !== undefined) clearTimeout(timer);
    watchClose(id);
  };
}
