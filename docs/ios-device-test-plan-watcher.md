# iOS filesystem watcher device test plan

Every row is **UNVERIFIED** until run on a jailbroken iOS device. A successful
target build only proves compilation; it does not prove kqueue event delivery or
PollWatcher timing. Record device model, iOS version, rootful/rootless layout,
launch mode, and Yazi commit for each run. Use an independent shell/process to
make external changes, and verify both the visible listing and the backing file
metadata after Yazi converges.

The local file watcher uses notify 8.2.0, whose iOS `RecommendedWatcher` is
KqueueWatcher. Yazi currently routes all local iOS paths to the one-second
PollWatcher because its nonrecursive watch set cannot observe child-file edits
and kqueue vnode watches do not follow replacement paths. No row below is
pre-marked passed.

## Local terminal

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | Edit existing file | Change file contents from a second local shell; Yazi updates size, mtime, and preview/list state. |
| UNVERIFIED | Create file | Create a new file in the current directory; Yazi adds exactly one entry. |
| UNVERIFIED | Remove file | Remove an existing entry; Yazi removes it without restarting. |
| UNVERIFIED | Rename file | Rename an entry within the watched directory; old and new names converge. |
| UNVERIFIED | Move file into watched directory | Move a file from a sibling directory into the current directory; the new entry appears. |
| UNVERIFIED | Move file out | Move a listed file into a sibling directory; the old entry disappears. |
| UNVERIFIED | Replace same pathname | Atomically replace a file, or remove then create a new inode at the same path; Yazi refreshes the new metadata/content. |
| UNVERIFIED | Delete/recreate watched directory | Remove the active or hovered directory tree, recreate the same path, then populate it; the parent listing and directory contents converge without restart. |
| UNVERIFIED | Metadata-only change | Run `chmod` and `touch` without changing contents; displayed permissions and timestamps update. |
| UNVERIFIED | Symlink target update | Change the target of a symlink visible in a watched directory; the displayed link metadata/target state converges. |
| UNVERIFIED | Linked-path refresh | Watch a symlinked directory, modify its canonical target from another shell, and verify both the target view and linked alias view update. |

## SSH

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | External edit from another SSH shell | Launch Yazi in one SSH session and edit a file in a second SSH session; the first session updates. |
| UNVERIFIED | Yazi launched over SSH | Start Yazi through SSH, modify its watched directory from a local device shell, and verify the remote PTY session updates. |
| UNVERIFIED | Rename/delete/recreate from another SSH process | Rename, delete, and recreate entries from a separate SSH process; Yazi converges without a blind retry or restart. |

## Resource and fallback behavior

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | Many watched paths | Exercise many tabs, current/parent directories, hovered folders, and explicit file watches; record responsiveness and watch failures. |
| UNVERIFIED | Observe descriptor count | Record Yazi's open descriptor count before and after watch changes and repeated file/directory replacements; verify it does not grow without bound. |
| UNVERIFIED | Approach descriptor limit | In a controlled environment, approach the process fd limit without changing it from Yazi; confirm PollWatcher registration failures remain nonfatal and report the affected path. Yazi does not register iOS local paths with kqueue. |
| UNVERIFIED | Non-directory path policy | Register an explicit file path; confirm the one-second PollWatcher sees modification, rename, deletion, and same-path recreation. |
| UNVERIFIED | Per-path polling state | Exercise multiple local Watchees and confirm each uses PollWatcher independently, with no process-wide fd-limit change. |
| UNVERIFIED | Primary registration policy | Confirm startup reports the source-based iOS PollWatcher policy and does not allocate per-path kqueue vnode watches. The generic primary-error fallback is covered by host policy tests; iOS does not attempt kqueue registration. |
| UNVERIFIED | Force primary registration failure if reproducible | If a diagnostic build can enable the KqueueWatcher attempt, force a registration error and verify the affected path degrades to PollWatcher. The normal iOS policy bypasses kqueue; do not change production behavior to run this row. |
| UNVERIFIED | PollWatcher refresh interval | With a PollWatcher path, externally create, modify, and remove entries; verify convergence within the configured one-second scan interval plus event batching. |
| UNVERIFIED | Unwatch/re-watch | Change the active/hovered watch set, then return to the original path; verify one effective watch per path and no stale duplicate reports. |

## Filesystem layout

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | Rootful path | Repeat create/rename/delete/recreate cases on a rootful jailbreak path outside the ordinary home directory where permitted. |
| UNVERIFIED | Rootless path | Repeat the same cases on a rootless jailbreak path, including the device's actual redirected root layout. |
| UNVERIFIED | Writable path outside home | Watch an allowed writable system/data location outside the ordinary home directory; verify permission-denied cases degrade without crashing. |

## Results log

Keep all entries above **UNVERIFIED** until physically executed. For each run,
record the exact trigger, elapsed convergence time, watcher log lines, Yazi fd
count before/after, and whether the run used a local terminal or SSH. Do not use
Linux host tests or an iOS cross-build to mark a row device-validated.
