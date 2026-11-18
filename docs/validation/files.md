# Validation: Files

| Date | Entry |
|---|---|
| 2026-11-18 | Git marks on rows already listed |

## 2026-11-18 · Git marks on rows already listed

Uncommitted work based on `235de52`. A file or directory row is drawn before Git status returns. The tree label gains a private suffix when that path is marked, so the row is drawn again. The visible name stays the file name. This covers the first level under the group root and a later change of the same mark.

- `./scripts/nsh test tests/files/GitMark.test.tsx`: **1/1**. The root directory `src` and the root file `readme.md` have no mark until status is released, then both show one.
- `./scripts/nsh test tests/files/GitStatus.test.tsx`: **5/5**. `./scripts/nsh test tests/files/Tree.test.tsx`: **4/4**.
- `./scripts/nsh check`, native AppKit, and a manual look at a real repository were not run.
