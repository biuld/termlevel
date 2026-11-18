/** One listed name. `kind` is directory, file, symlink or other. */
export interface FileEntry {
  name: string;
  kind: string;
  hidden: boolean;
  bytes?: number | null;
}

export interface DirectoryListing {
  entries: FileEntry[];
  truncated: boolean;
  unreadable?: string | null;
}

export interface FileText {
  text: string;
  binary: boolean;
  truncated: boolean;
  lossy: boolean;
  bytes: number;
  modifiedMs: number;
}

export interface WatchNote {
  directory: string;
  message: string;
}

/** Disk access for the file view. Preview and tests supply a fixture. */
export interface FilesTransport {
  list: (root: string, directory: string) => Promise<DirectoryListing>;
  read: (root: string, path: string) => Promise<FileText>;
  save: (root: string, path: string, text: string, modifiedMs: number, bytes: number) => Promise<FileText>;
  absolute: (root: string, path: string) => Promise<string>;
  previewUrl?: (root: string, path: string) => Promise<string>;
  terminalDirectory: (root: string, path: string) => Promise<string>;
  reveal: (root: string, path: string) => Promise<void>;
  openWithDefaultApp: (root: string, path: string) => Promise<void>;
  trash: (root: string, path: string) => Promise<void>;
  /** Absent in preview. The subscription watches nothing until `watchSet`. */
  watchOpen?: () => number;
  watchSet?: (id: number, root: string, directories: string[]) => Promise<WatchNote[]>;
  watchPoll?: (id: number) => string[];
  watchClose?: (id: number) => void;
}
