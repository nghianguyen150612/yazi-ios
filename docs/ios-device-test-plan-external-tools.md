# Yazi-iOS Task 017 external-tools device test plan

Every physical-device row intentionally starts as **UNVERIFIED**. Host tests,
source inspection, and target compilation do not change that label.

| # | Status | Physical validation |
| ---: | --- | --- |
| 1 | UNVERIFIED | `ya doctor` from a local terminal |
| 2 | UNVERIFIED | `ya doctor` over SSH |
| 3 | UNVERIFIED | `ya doctor --json` |
| 4 | UNVERIFIED | Doctor reports the process `PATH` accurately |
| 5 | UNVERIFIED | Empty/minimal `PATH` reports missing optional helpers and succeeds |
| 6 | UNVERIFIED | Repeated doctor runs do not alter the device |
| 7 | UNVERIFIED | `fd` installed and filename search works |
| 8 | UNVERIFIED | Only `fdfind` installed and filename search works |
| 9 | UNVERIFIED | Neither filename-search alias installed; feature error is actionable |
| 10 | UNVERIFIED | `rg` installed and content search works |
| 11 | UNVERIFIED | `rg` absent; Yazi remains usable |
| 12 | UNVERIFIED | `fzf` installed and interactive jump works |
| 13 | UNVERIFIED | `fzf` absent; Yazi remains usable |
| 14 | UNVERIFIED | `zoxide` installed with history |
| 15 | UNVERIFIED | `zoxide` installed with empty history reports no history |
| 16 | UNVERIFIED | `zoxide` absent reports failed start, not empty history |
| 17 | UNVERIFIED | `file` installed and MIME detection works |
| 18 | UNVERIFIED | Custom executable through `YAZI_FILE_ONE` works |
| 19 | UNVERIFIED | `file` absent uses MIME placeholder and preview diagnostic |
| 20 | UNVERIFIED | `jq` installed and JSON preview works |
| 21 | UNVERIFIED | `jq` absent falls back to code preview |
| 22 | UNVERIFIED | `ffmpeg` and `ffprobe` installed; video preview works |
| 23 | UNVERIFIED | `ffmpeg` absent reports frame-preview error only |
| 24 | UNVERIFIED | `ffprobe` absent reports metadata error only |
| 25 | UNVERIFIED | `pdftoppm` installed and PDF preview works |
| 26 | UNVERIFIED | `pdftoppm` absent reports PDF preview error only |
| 27 | UNVERIFIED | `magick` installed and affected image preview works |
| 28 | UNVERIFIED | `magick` absent reports affected image preview error only |
| 29 | UNVERIFIED | `resvg` installed and SVG preview works |
| 30 | UNVERIFIED | `resvg` absent reports SVG preview error only |
| 31 | UNVERIFIED | `chafa` installed and terminal fallback works |
| 32 | UNVERIFIED | `chafa` absent uses metadata/text fallback |
| 33 | UNVERIFIED | `7zz` installed and archive preview works |
| 34 | UNVERIFIED | Only `7z` installed and archive preview works |
| 35 | UNVERIFIED | Neither archive alias installed; error is actionable |
| 36 | UNVERIFIED | `git` installed and `ya pkg` works |
| 37 | UNVERIFIED | `git` absent; `ya pkg` reports an actionable command error |
| 38 | UNVERIFIED | `tmux` installed and passthrough/session behavior works |
| 39 | UNVERIFIED | `tmux` absent without core regression |
| 40 | UNVERIFIED | `zellij` installed and session behavior works |
| 41 | UNVERIFIED | `zellij` absent without core regression |
| 42 | UNVERIFIED | Desktop clipboard helpers absent on iOS without native clipboard regression |
| 43 | UNVERIFIED | `ueberzugpp` absent on iOS without image-preview regression |
| 44 | UNVERIFIED | `EDITOR` configured and edit opener works |
| 45 | UNVERIFIED | `EDITOR` unset with `vi` present |
| 46 | UNVERIFIED | `EDITOR` unset with `vi` absent; action error is scoped |
| 47 | UNVERIFIED | `mediainfo` absent; opener menu remains usable |
| 48 | UNVERIFIED | `exiftool` absent; reveal/open behavior remains usable |
| 49 | UNVERIFIED | Rootful-style `PATH` containing bootstrap directories |
| 50 | UNVERIFIED | Rootless-style `PATH` containing bootstrap directories |
| 51 | UNVERIFIED | Doctor as root |
| 52 | UNVERIFIED | Doctor as mobile/non-root |
| 53 | UNVERIFIED | SSH `PATH` differs from local terminal `PATH` and doctor reports each process |
| 54 | UNVERIFIED | Run doctor 20 times repeatedly |
| 55 | UNVERIFIED | No doctor helper probe hangs or waits for input |
| 56 | UNVERIFIED | Helper with unusual nonzero version exit is still `available` |
| 57 | UNVERIFIED | Executable path containing spaces |
| 58 | UNVERIFIED | Unicode `PATH` entry |
| 59 | UNVERIFIED | Missing optional helpers do not affect Yazi startup |
| 60 | UNVERIFIED | Full helper set available and every reported alias is correct |

## Capture requirements

For every run record the exact device OS, process identity, terminal/session,
`PATH`, doctor output, and the feature exercised. Do not record tokens or the
full environment. Do not convert a package name or guessed bootstrap path into
a support claim without a physical observation.
