import { Files } from "termlevel/native";
import type { FilesTransport } from "./types";

/** Real disk access. Importing this module does not list anything. */
export const nativeFileTransport: FilesTransport = {
    list: (root, directory) => Files.list(root, directory),
    read: (root, path) => Files.read(root, path),
    save: (root, path, text, modifiedMs, bytes) => Files.save(root, path, text, modifiedMs, bytes),
    absolute: (root, path) => Files.absolute(root, path),
    terminalDirectory: (root, path) => Files.terminalDirectory(root, path),
    reveal: (root, path) => Files.reveal(root, path),
    openWithDefaultApp: (root, path) => Files.openWithDefaultApp(root, path),
    trash: (root, path) => Files.trash(root, path),
    watchOpen: () => Files.watchOpen(),
    watchSet: (id, root, directories) => Files.watchSet(id, root, directories),
    watchPoll: (id) => Files.watchPoll(id),
    watchClose: (id) => Files.watchClose(id),
};
