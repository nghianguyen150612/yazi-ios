# Task 008 device test plan: process and shell execution

Status: **NOT RUN.** Every row below is `UNVERIFIED` until a physical jailbroken
device executes it. Task 008 is compile-validated for `aarch64-apple-ios` only;
compile success does not prove that process creation is permitted on a device.

Run this on the jailbroken iPad. Record the iOS version, jailbreak type
(rootful/rootless), jailbreak tool and version, terminal or SSH client, and
`sh` location/version for every session.

## Environment to record first

| Item | Value |
| --- | --- |
| iOS version | |
| Jailbreak (rootful/rootless, tool, version) | |
| Launch method (SSH / local terminal) | |
| Terminal or SSH client + version | |
| `command -v sh` | |
| `sh --version` | |
| `echo $PATH` | |

Run the matrix at minimum once over **SSH** and once from a **local jailbreak
terminal**. Rows 1–10 are the required set.

## Matrix

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 1 | Yazi launched through an SSH PTY | `yazi` over `ssh user@device` | TUI renders, input works | UNVERIFIED |
| 2 | Yazi launched from a local jailbreak terminal | `yazi` in the device terminal | TUI renders, input works | UNVERIFIED |
| 3 | Blocking shell command | `:shell echo hello; sleep 1; echo done` | TUI restores, output visible, returns to Yazi after exit | UNVERIFIED |
| 4 | Non-blocking shell command | `:shell -b 'for i in 1 2 3; do echo $i; sleep 1; done'` | Output streams into the task/log view, Yazi stays responsive | UNVERIFIED |
| 5 | Orphan / background command | Run an `orphan = true` opener rule, then quit Yazi | Process keeps running after Yazi exits | UNVERIFIED |
| 6 | Cwd-sensitive command | From a known directory, `:shell pwd` | Prints that directory, not Yazi's launch dir | UNVERIFIED |
| 7 | Inherited stdin/stdout/stderr | `:shell 'read -r x; echo "got:$x"'` then type a value | Child reads the terminal and writes to it | UNVERIFIED |
| 8 | Nonexistent command | `:shell definitely-not-a-real-command` | Clear failure reported; **Yazi stays usable**, no silent no-op | UNVERIFIED |
| 9 | Process cleanup / no zombies | `sleep 300` repeatedly, then `ps -o pid,stat,comm \| grep defunct` | No `Z`/defunct entries after children exit | UNVERIFIED |
| 10 | Terminal returns cleanly | After each child exits | No leftover raw mode, no stray echo, TUI redraws correctly | UNVERIFIED |

## Diagnostic checks for the new error paths

These validate `spawn_error()`. Row 8 above covers a command that does not
exist; these cover the *spawn itself* failing.

| # | Scenario | How to force it | Expected message | Result |
| --- | --- | --- | --- | --- |
| 11 | Missing shell | Launch Yazi with a `PATH` that excludes `sh`, e.g. `env PATH=/nonexistent yazi`, then `:shell echo hi` | Names that `` `sh` was not found in `PATH` `` | UNVERIFIED |
| 12 | Inaccessible working directory | Enter/remove a directory so the cached cwd no longer resolves, then run a shell task | Names that the working directory is not accessible | UNVERIFIED |
| 13 | Process creation denied | If the runtime denies `fork`, run `:shell true` | Names that process creation was denied | UNVERIFIED |

## `setsid` / detachment checks

Run these from a second SSH session while the command is in flight.

| # | Check | Command | Expected | Result |
| --- | --- | --- | --- | --- |
| 14 | Non-blocking command is in its own session | `ps -o pid,ppid,pgid,sid,tty -A \| grep <pid>` | `pgid`/`sid` differ from Yazi's, no controlling tty | UNVERIFIED |
| 15 | Ctrl-C in the TUI does not kill a detached child | Start a long `:shell -b 'sleep 300'`, press Ctrl-C in Yazi | Yazi handles the key; the child survives | UNVERIFIED |
| 16 | Orphan survives Yazi exit | Start an orphan, quit Yazi, `ps` from SSH | Process still present | UNVERIFIED |
| 17 | Blocking command keeps the terminal | `:shell 'read -r x; echo "got:$x"'` | Reads from the live tty, so it must **not** be detached | UNVERIFIED |

## Notes on interpretation

- Rows 11–13 are the only checks that exercise the new diagnostics. Rows 1–10
  and 14–17 validate that the *existing* fork+`setsid` path behaves on device.
- Row 9 matters most after several orphan runs, since zombies are reaped by
  tokio's orphan queue rather than by an explicit `wait`.
- If row 13 reports a denial, record the exact errno. That is the signal for a
  process capability policy change, not a reason to disable shell support.
- Do not mark any row PASS from a simulator, a host machine, or a successful
  build. Record the device and jailbreak details alongside each result.
