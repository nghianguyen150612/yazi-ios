# Task 010 device test plan: opener

Status: **NOT RUN.** Every row below is `UNVERIFIED` until a physical jailbroken
device executes it. Task 010 is compile-validated for `aarch64-apple-ios` only;
compile success proves that `NSURL` links, not that a shell process can make
LaunchServices hand a document to an app.

Run this on the jailbroken iPad. Record the iOS version, jailbreak type
(rootful/rootless), jailbreak tool and version, terminal or SSH client, and
whether the device was locked or unlocked at the time of each run.

## Environment to record first

| Item | Value |
| --- | --- |
| iOS version | |
| Jailbreak (rootful/rootless, tool, version) | |
| Launch method (SSH / local terminal) | |
| Terminal or SSH client + version | |
| `yazi --version` | |
| `ya open` on `PATH` (`command -v ya`) | |
| Apps installed that can open JPEG/PNG/PDF/audio/video | |
| Device locked or unlocked during the run | |

Run rows 1–10 from a **local jailbreak terminal** and rows 11–13 over **SSH**.
Rows 14–15 apply to both.

## Local terminal: open and play

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 1 | Open a JPEG | `yazi`, hover a `.jpg`, <kbd>Enter</kbd> (Open) | The associated viewer launches on the device; the Yazi process is unaffected | UNVERIFIED |
| 2 | Open a PNG | Same, on a `.png` | The associated viewer launches | UNVERIFIED |
| 3 | Open a PDF | Same, on a `.pdf` | A PDF app opens the document, or a clear "No application on this device can open …" is reported | UNVERIFIED |
| 4 | Edit a text file | Hover a `.md`, choose Edit, confirm the picked opener is `$EDITOR` | The terminal editor opens in the foreground and Yazi blocks, then resumes | UNVERIFIED |
| 5 | Play audio/video | Hover an `.mp4`, choose Play | A media app launches; the option list shows both `Play` and `Show media info` | UNVERIFIED |
| 6 | Open a directory | Enter the parent, select a folder, <kbd>Enter</kbd> | Files opens that folder; a folder URL is not mistaken for a document | UNVERIFIED |
| 7 | Reveal | Select a file, choose `Reveal in Files` | Files opens the **containing folder**. It must not silently open the file itself, and there is no selection highlight to verify | UNVERIFIED |
| 8 | Filename with spaces | Create `a b c.png`, open it | The correct file opens; no argument splitting, no `file not found` | UNVERIFIED |
| 9 | Unicode filename | Create `图片 テスト.png`, open it | The correct file opens; the bytes reach LaunchServices unchanged | UNVERIFIED |
| 10 | Multiple selected files | Select three images, <kbd>Enter</kbd> | Every file is handed off, not only the first. `OpenDo` chunks `%s1` into one invocation per file, so Yazi spawns three `ya open` processes | UNVERIFIED |

## SSH

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 11 | Device-local file opens on the device | `ssh user@device`, `yazi`, open a photo from the device filesystem | The app launches **on the iPhone/iPad**. The PC must not receive an OSC sequence, a URL-open request, or any other "open this on my machine" attempt — the PC has never seen the file | UNVERIFIED |
| 12 | No helper command is needed | Over SSH, open a file, then from a second session `ps -o pid,ppid,comm -A \| grep -E 'uiopen\|open$'` | Only the short-lived `ya open` process; no `uiopen`, no `open`, and no dependency on any jailbreak package | UNVERIFIED |
| 13 | Return to the terminal | After a handoff, return to the SSH session and keep using Yazi | Yazi is still running, still redrawing, and the SSH session is intact | UNVERIFIED |

## Rootful and rootless

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 14 | No jailbreak layout is assumed | Run `ya open <file>` and `yazi`'s opener on both a rootful and a rootless device | Identical behavior. No code path may reference `/usr/bin/uiopen`, `/var/jb/usr/bin/uiopen`, or any other absolute helper path | UNVERIFIED |
| 15 | Failure is a diagnostic, not a crash | Open a file type no installed app handles; then open a path that does not exist | A named, non-fatal error in Yazi's task list; Yazi keeps running and no helper is required | UNVERIFIED |

## Notes on interpretation

- Rows 1–3 and 10 are the ones that prove the feature. A build cannot show that
  a device with no app for a type reports anything at all, which is row 3's
  second half and row 15.
- Row 7 is the row that can look like a bug. iOS has no equivalent of Finder's
  "select the item in the enclosing window": Files is handed a folder and shows
  the folder. The expected behavior is the folder opening, and the option is
  named `Reveal in Files` rather than `Reveal` for that reason. Do not mark it
  FAIL for lacking a highlight.
- Row 8 and row 9 are the encoding checks. A shell-splitted path or a
  percent-escaped Unicode path would both look like "opening does nothing".
- Row 11 is the SSH-semantics check. The failure mode to watch for is an
  implementation that forwards the open request to the SSH client; the file
  exists only on the device, so any such forwarding is wrong by construction.
- Row 14 is a code-review check that a device run confirms: `yazi-ffi` resolves
  only `/System/Library/Frameworks/…` paths, which live in the read-only system
  volume and are identical on both jailbreak layouts.
- Do not mark any row PASS from a simulator, a host machine, or a successful
  build. Record the device and jailbreak details alongside each result.
