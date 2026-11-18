import { cleanup, Clipboard, computed, effect, signal, Text, token, untracked, View } from "nsh";
import { Editor, ResizablePanel, ResizablePanelGroup, Tree, type TreeEntry } from "nsh/base";
import { Menu } from "nsh/desktop";
import { Action, AppDialog, DialogActions, IconButton } from "../shared/controls";
import { monoFont } from "../shared/fonts";
import { Icon } from "../shared/icons";
import { Type } from "../shared/type";
import type { GitTransport } from "../git/git-types";
import { emptyGitOverlay, gitScope, startGitStatus, statusColor, type GitOverlay } from "./git-status";
import { buildFileTree, isMarker, rememberExpanded } from "./tree";
import type { DirectoryListing, FileEntry, FilesTransport, FileText } from "./types";
import { startDirectoryWatch } from "./watch";

export function FilesPanel(props: {
  root: string;
  showHidden: boolean;
  expanded: string[];
  selected: string | null;
  transport: FilesTransport;
  git?: GitTransport;
  notify: (level: "info" | "error", title: string, body: string) => void;
  onShowHidden: (value: boolean) => void;
  onExpanded: (paths: string[]) => void;
  onSelected: (path: string | null) => void;
  onOpenTerminal: (cwd: string) => void;
}) {
  const listings = signal<Record<string, DirectoryListing>>({});
  const watchNotes = signal<Record<string, string>>({});
  const gitMarks = signal<GitOverlay>(emptyGitOverlay());
  let refreshGit = () => {};
  const rootError = signal("");
  const text = signal("");
  const saved = signal("");
  const modifiedMs = signal(0);
  const savedBytes = signal(0);
  const fileNote = signal("");
  const readOnly = signal(true);
  const loadingFile = signal(false);
  const saving = signal(false);
  const confirmDiscard = signal(false);
  const confirmTrash = signal(false);
  const pendingSelect = signal<string | null>(null);
  const generations = new Map<string, number>();
  let seenRoot = "";
  let openedKey = "";
  let fileToken = 0;
  let alive = true;
  cleanup(() => { alive = false; });

  const tree = computed(() => buildFileTree(listings(), props.expanded, props.showHidden, watchNotes(), gitMarks()));
  const selectedGit = computed(() => props.selected ? gitMarks().files.get(props.selected) : undefined);
  const dirty = computed(() => text() !== saved());

  async function load(directory: string) {
    const tokenId = (generations.get(directory) ?? 0) + 1;
    generations.set(directory, tokenId);
    const root = props.root;
    try {
      const listing = await props.transport.list(root, directory);
      if (!alive || generations.get(directory) !== tokenId || props.root !== root) return;
      listings.update((all) => ({ ...all, [directory]: listing }));
      if (directory === "") rootError.set(listing.unreadable ?? "");
    } catch (error) {
      if (!alive || generations.get(directory) !== tokenId) return;
      const message = error instanceof Error ? error.message : "The folder cannot be read.";
      if (directory === "") rootError.set(message);
      listings.update((all) => ({ ...all, [directory]: { entries: [], truncated: false, unreadable: message } }));
    }
  }

  function refresh() {
    listings.set({});
    generations.clear();
    openedKey = "";
    rootError.set("");
    for (const directory of ["", ...props.expanded]) void load(directory);
    refreshGit();
  }

  effect(() => {
    const root = props.root;
    const expanded = props.expanded;
    if (root !== seenRoot) {
      seenRoot = root;
      listings.set({});
      watchNotes.set({});
      gitMarks.set(emptyGitOverlay());
      generations.clear();
      openedKey = "";
      rootError.set("");
    }
    untracked(() => {
      for (const directory of ["", ...expanded]) {
        if (!listings()[directory]) void load(directory);
      }
    });
  });

  async function openFile(root: string, path: string | null, entry: FileEntry | null) {
    const key = path ? `${root}\0${path}` : "";
    if (!path || !entry || (entry.kind !== "file" && entry.kind !== "symlink")) {
      openedKey = "";
      text.set("");
      saved.set("");
      fileNote.set(path && gitMarks().files.get(path)?.deleted ? "This file was deleted." : path && entry ? "Open a file to edit it." : "");
      readOnly.set(true);
      return;
    }
    if (openedKey === key) return;
    openedKey = key;
    const tokenId = ++fileToken;
    loadingFile.set(true);
    fileNote.set("");
    try {
      const file = await props.transport.read(root, path);
      if (!alive || tokenId !== fileToken) return;
      applyFile(file);
    } catch (error) {
      if (!alive || tokenId !== fileToken) return;
      text.set("");
      saved.set("");
      readOnly.set(true);
      fileNote.set(error instanceof Error ? error.message : "The file cannot be read.");
    } finally {
      if (tokenId === fileToken) loadingFile.set(false);
    }
  }

  function applyFile(file: FileText) {
    const blocked = file.binary || file.truncated || file.lossy;
    text.set(blocked ? "" : file.text);
    saved.set(blocked ? "" : file.text);
    modifiedMs.set(file.modifiedMs);
    savedBytes.set(file.bytes);
    readOnly.set(blocked);
    fileNote.set(file.binary
      ? "This file is binary. Open it with the default app."
      : file.truncated
        ? "This file is larger than 512 KiB."
        : file.lossy
          ? "This file is not valid UTF-8."
          : "");
  }

  effect(() => {
    const root = props.root;
    gitMarks.set(emptyGitOverlay());
    const git = startGitStatus({
      status: props.git ? (directory) => props.git!.status(directory) : undefined,
      root: () => root,
      absolute: () => props.transport.absolute(root, ""),
      onOverlay: (overlay) => gitMarks.set(overlay),
    });
    refreshGit = git.refresh;
    cleanup(() => { refreshGit = () => {}; git.stop(); });
  });

  effect(() => {
    const root = props.root;
    const stop = startDirectoryWatch({
      transport: props.transport,
      root: () => root,
      expanded: () => props.expanded,
      onChanged: (directories) => {
        const path = props.selected;
        const parent = path?.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
        if (path && directories.includes(parent) && text() === saved()) openedKey = "";
        const open = new Set(props.expanded);
        for (const directory of directories) {
          if (directory === "" || open.has(directory)) void load(directory);
        }
        refreshGit();
      },
      onFailures: (notes) => {
        const next: Record<string, string> = {};
        for (const note of notes) next[note.directory] = note.message;
        watchNotes.set(next);
      },
    });
    cleanup(stop);
  });

  effect(() => {
    const root = props.root;
    const path = props.selected;
    const entry = path ? tree().entries.get(path) ?? null : null;
    const parent = path && path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
    if (path && !entry && !listings()[parent]) return;
    untracked(() => { void openFile(root, path, entry); });
  });

  function choose(id: string) {
    if (!id || isMarker(id) || id === props.selected) return;
    if (dirty()) {
      pendingSelect.set(id);
      confirmDiscard.set(true);
      return;
    }
    props.onSelected(id);
  }

  async function save() {
    const path = props.selected;
    if (!path || readOnly() || saving() || !dirty()) return;
    saving.set(true);
    try {
      const file = await props.transport.save(props.root, path, text(), modifiedMs(), savedBytes());
      if (!alive) return;
      applyFile(file);
    } catch (error) {
      if (!alive) return;
      props.notify("error", "Could not save file", error instanceof Error ? error.message : "The file cannot be saved.");
    } finally {
      saving.set(false);
    }
  }

  async function run(title: string, action: () => Promise<void>) {
    try {
      await action();
    } catch (error) {
      props.notify("error", title, error instanceof Error ? error.message : "The action failed.");
    }
  }

  const pathOf = () => props.selected;
  function menu() {
    const path = pathOf();
    if (!path) return;
    void Menu.popup([
      { label: "Reveal in Finder", onSelect: () => { void run("Could not reveal file", () => props.transport.reveal(props.root, path)); } },
      { label: "Open with default app", onSelect: () => { void run("Could not open file", () => props.transport.openWithDefaultApp(props.root, path)); } },
      { label: "Open terminal here", onSelect: () => { void run("Could not open terminal", async () => props.onOpenTerminal(await props.transport.terminalDirectory(props.root, path))); } },
      { label: "Copy path", onSelect: () => { void run("Could not copy path", async () => { await Clipboard.writeText(await props.transport.absolute(props.root, path)); }); } },
      { label: "Move to Trash", onSelect: () => confirmTrash.set(true) },
    ]);
  }

  const renderItem = (entry: TreeEntry) => {
    const item = tree().entries.get(entry.id);
    const marker = isMarker(entry.id);
    const mark = marker ? undefined : gitMarks().files.get(entry.id);
    const folderDirty = !marker && !mark && !!entry.folder && gitMarks().folders.has(entry.id);
    return <View testID={marker ? undefined : `file-${entry.id.replace(/[^a-zA-Z0-9]+/g, "-")}`}
      class="h-(--height-tab) flex-row items-center gap-1.5"
      style={{ paddingLeft: 6 + entry.depth * 14, paddingRight: 8 }}
      onContextMenu={() => { if (!marker) { choose(entry.id); menu(); } }}>
      <If when={entry.folder && !marker}>
        <Icon name={props.expanded.includes(entry.id) ? "down" : "chevron"} size={10} color={token("subtle-foreground")} />
        <Icon name={item?.kind === "symlink" ? "file" : "folder"} size={13} color={token("icon")} />
      </If>
      <If when={!entry.folder || marker}>
        <View style={{ width: 10 }} />
        <If when={!marker}><Icon name="file" size={13} color={token("icon")} /></If>
      </If>
      <Type role="body" class="min-w-0 flex-1" lines={1} tone={marker ? "muted-foreground" : "foreground"}>{entry.label}</Type>
      <If when={!!mark}>
        <View style={{ width: 5, height: 5, borderRadius: 2.5, backgroundColor: mark!.staged ? token("primary") : "transparent" }} />
        <Text style={{ width: 12, textAlign: "center", fontFamily: monoFont(), fontSize: token("text-caption"), color: statusColor(mark!.status) }}>{mark!.status}</Text>
      </If>
      <If when={folderDirty}>
        <View style={{ width: 5, height: 5, borderRadius: 2.5, backgroundColor: token("notification-warning") }} />
      </If>
    </View>;
  };

  return <View testID="files-panel" class="h-full min-h-0 flex-col">
    <View class="h-(--height-toolbar) shrink-0 flex-row items-center gap-2 border-b border-border px-3">
      <Type role="body" family="mono" class="min-w-0 flex-1" lines={1}>{props.root}</Type>
      <IconButton icon={props.showHidden ? "check" : "file"} label={props.showHidden ? "Hide hidden files" : "Show hidden files"}
        testID="files-hidden" selected={props.showHidden} onPress={() => props.onShowHidden(!props.showHidden)} />
      <IconButton icon="refresh" label="Refresh files" testID="files-refresh" onPress={refresh} />
    </View>
    <If when={!!rootError()}>
      <View class="flex-1 items-center justify-center gap-2 px-6">
        <Type role="heading">The folder cannot be read.</Type>
        <Type role="body" tone="muted-foreground" class="text-center">{rootError()}</Type>
      </View>
    </If>
    <If when={!rootError()}>
      <ResizablePanelGroup class="min-h-0 flex-1" handleClass="bg-transparent group-hover:bg-sidebar-border group-active:bg-ring"
        handleAccessibilityLabel="Resize files and editor">
        <ResizablePanel size={280} minSize={200} maxSize={480}>
          <View class="h-full min-h-0 border-r border-border">
            <Tree class="h-full min-h-0" listClass="py-1" accessibilityLabel="Files"
              items={tree().items} expanded={props.expanded} selected={props.selected}
              itemClass="mx-1 rounded-(--radius-tab) hover:bg-tab-hover selected:bg-tab-selected"
              renderItem={renderItem}
              onExpandedChange={({ expanded: next }) => {
                const capped = rememberExpanded(props.expanded, next);
                props.onExpanded(capped);
                for (const id of capped) if (!listings()[id]) void load(id);
              }}
              onSelectedChange={({ selected: id }) => choose(id)} />
          </View>
        </ResizablePanel>
        <ResizablePanel minSize={320}>
          <View class="h-full min-h-0 flex-col">
            <View class="h-(--height-status) shrink-0 flex-row items-center gap-2 border-b border-border px-3">
              <Type role="body" family="mono" class="min-w-0 flex-1" lines={1}>{props.selected ?? "No file selected"}</Type>
              <If when={!!selectedGit()}>
                <Text testID="files-git-status" style={{ fontFamily: monoFont(), fontSize: token("text-caption"), color: statusColor(selectedGit()!.status) }}>{selectedGit()!.status}</Text>
                <Type role="caption" tone="muted-foreground">{gitScope(selectedGit()!)}</Type>
              </If>
              <If when={!!props.selected && !readOnly()}>
                <IconButton icon="check" label={dirty() ? "Save file" : "File saved"} testID="files-save" onPress={() => { void save(); }} />
              </If>
              <If when={!!props.selected}>
                <IconButton icon="more" label="File actions" testID="files-actions" onPress={menu} />
              </If>
            </View>
            <If when={loadingFile()}>
              <View class="flex-1 items-center justify-center"><Type role="body" tone="muted-foreground">Loading file…</Type></View>
            </If>
            <If when={!loadingFile() && !!fileNote()}>
              <View class="flex-1 items-center justify-center gap-2 px-6">
                <Type role="body" tone="muted-foreground" class="text-center">{fileNote()}</Type>
              </View>
            </If>
            <If when={!loadingFile() && !fileNote() && !!props.selected && !readOnly()}>
              <Editor testID="files-editor" class="min-h-0 flex-1" value={text()} language={props.selected}
                readOnly={false} autoFocus={false} fontFamily={monoFont()} fontSize={token("text-body")}
                lineHeight={token("leading-body") + 4}
                onChange={({ value }) => text.set(value)} />
            </If>
            <If when={!loadingFile() && !fileNote() && !props.selected}>
              <View class="flex-1 items-center justify-center"><Type role="body" tone="muted-foreground">Select a file to edit it.</Type></View>
            </If>
          </View>
        </ResizablePanel>
      </ResizablePanelGroup>
    </If>
    <AppDialog open={confirmDiscard()} label="Discard unsaved changes" role="alertdialog" frame="w-[380px]"
      title="Discard unsaved changes?" description="This file has edits that are not saved."
      dismiss={() => confirmDiscard.set(false)}>
      <DialogActions>
        <Action onPress={() => confirmDiscard.set(false)}>Keep editing</Action>
        <Action testID="files-discard" onPress={() => {
          const next = pendingSelect();
          confirmDiscard.set(false);
          saved.set(text());
          if (next) props.onSelected(next);
        }}>Discard</Action>
      </DialogActions>
    </AppDialog>
    <AppDialog open={confirmTrash()} label="Move to Trash" role="alertdialog" frame="w-[380px]"
      title="Move this to the Trash?" description={props.selected ?? ""}
      dismiss={() => confirmTrash.set(false)}>
      <DialogActions>
        <Action onPress={() => confirmTrash.set(false)}>Cancel</Action>
        <Action testID="files-trash" destructive onPress={() => {
          const path = props.selected;
          confirmTrash.set(false);
          if (!path) return;
          void run("Could not move to Trash", async () => {
            await props.transport.trash(props.root, path);
            openedKey = "";
            props.onSelected(null);
            refresh();
          });
        }}>Move to Trash</Action>
      </DialogActions>
    </AppDialog>
  </View>;
}
