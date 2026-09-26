# Task 009 device test plan: clipboard

Status: **NOT RUN.** Every row below is `UNVERIFIED` until a physical jailbroken
device executes it. Task 009 is compile-validated for `aarch64-apple-ios` only;
compile success proves that UIKit links, not that a shell process can drive the
device pasteboard.

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
| Device locked or unlocked during the run | |
| Third-party clipboard tools installed (e.g. a `pbcopy` replacement) | |

Run rows 1–9 from a **local jailbreak terminal** and rows 10–12 over **SSH**.
Rows 13–15 apply to both.

## Local terminal: device pasteboard

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 1 | Yazi set reaches another iOS app | `yazi`, select a file, `y yank` (copy path), switch to Notes/Safari, paste | The pasted text is the yanked path | UNVERIFIED |
| 2 | Another iOS app reaches Yazi | Copy text in Notes/Safari, return to Yazi, open a prompt (`/` or `:`) and paste with the configured paste key | The prompt contains the copied text | UNVERIFIED |
| 3 | Unicode survives a round trip | `y yank` on a file named with CJK/emoji, paste into Notes, then copy it back and paste into a Yazi prompt | Identical characters both ways; no `?` or U+FFFD for valid text | UNVERIFIED |
| 4 | Multiline text survives | `ya.clipboard()`-style set of `a\nb\nc`, paste into Notes, then copy it back into a Yazi prompt | All three lines present, newlines intact | UNVERIFIED |
| 5 | Empty clipboard | Copy an image in Photos, then paste into a Yazi prompt | No text is inserted; the in-process mirror is used, and Yazi does not hang or error | UNVERIFIED |
| 6 | Repeated set/get | Yank a path, paste it, yank a different path, paste again, five times | Each paste shows the most recent yank; no stale text, no growth in RSS | UNVERIFIED |
| 7 | The UI never blocks on the pasteboard | Yank and immediately paste; watch the TUI while the device is busy | The TUI stays responsive; the first paste after launch may take a moment but must not freeze the interface | UNVERIFIED |
| 8 | No helper process is spawned | `yazi`, yank, then from SSH `ps -o pid,ppid,comm -A \| grep -E 'pbcopy\|pbpaste\|xclip\|xsel\|wl-copy\|wl-paste\|termux'` | No such child of the Yazi PID | UNVERIFIED |
| 9 | A missing pasteboard degrades | Yank, then `y yank` again and paste into a Yazi prompt after clearing the device pasteboard in another app | The in-process mirror answers; no crash, no error dialog | UNVERIFIED |

## SSH

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 10 | Yazi set reaches the PC clipboard | `ssh user@device`, `yazi`, `y yank`, then paste into a PC application | The path is on the PC's clipboard via OSC 52, even though the PC terminal never asked for it | UNVERIFIED |
| 11 | SSH read uses the documented fallback | Over SSH, yank path A, then yank path B elsewhere; paste into a Yazi prompt | The mirror answers with the last yank, and the **device** pasteboard is not consulted | UNVERIFIED |
| 12 | No local helper loop over SSH | Over SSH, yank, then check the device process list as in row 8 | No clipboard helper process is spawned on the device | UNVERIFIED |

## Privacy and failure

| # | Scenario | How to force it | Expected | Result |
| --- | --- | --- | --- | --- |
| 13 | Paste prompt appears on read | Copy in another app, then paste into Yazi on iOS 16 or later | If iOS shows an "Allow Paste" prompt, record whether it appeared, whether it must be answered, and what Yazi pastes when it is declined | UNVERIFIED |
| 14 | Locked device | Lock the device, then paste into a Yazi prompt over SSH | No text or the mirror; no crash, no hang | UNVERIFIED |
| 15 | UIKit or pasteboard call fails | If the pasteboard is unavailable (unsupported jailbreak state, `pbs` unavailable), yank and paste | Yazi stays usable: the mirror answers, no panic, no repeated retry, no error spam | UNVERIFIED |

## Notes on interpretation

- Rows 1–9 are the ones that prove the feature. Rows 10–12 prove that nothing
  regressed for SSH, and 13–15 cover the privacy and failure paths.
- Row 3 is the check that matters most for a file manager: a mangled filename
  would look like a pasteboard bug and actually be an encoding one. Only valid
  UTF-8 is expected to round-trip exactly; a genuinely non-UTF-8 filename is
  expected to become U+FFFD in the device pasteboard while Yazi's own mirror
  keeps the original bytes.
- Row 8 and row 12 are the regression guard for the "five useless spawns per
  paste" behavior Task 009 removed. Check the device from a *second* session,
  since the spawn would be short-lived.
- Row 15 is not optional just because nothing is expected to fail. The point is
  that a pasteboard failure must cost a capability, not a session.
- Do not mark any row PASS from a simulator, a host machine, or a successful
  build. Record the device and jailbreak details alongside each result.
