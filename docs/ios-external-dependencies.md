# Yazi-iOS external dependency matrix

Task 017 records external programs as capabilities. None of the optional rows
below is a launch-time requirement for `yazi` or `ya`. Lookup is performed by
the current process through `PATH`; no jailbreak bootstrap directory is
hard-coded. `ya doctor` probes the same effective command names and aliases.

## Classification

| Class | Meaning |
| --- | --- |
| Core | Required for basic startup. Task 017 finds no external core executable. |
| Feature | Needed only when the named feature is invoked. |
| Fallback | An optional helper used when a built-in or alternate path is available. |
| Integration | Terminal, desktop, session, or clipboard integration. |
| Configured | A user action or environment-selected command. |
| Not applicable | Not part of the current platform/session implementation. |

The JSON status values are `available`, `missing`, `not_applicable`,
`configured`, and `unknown`. Missing optional capabilities are reported as
`OPTIONAL MISSING` by the human report and do not change the exit status.

## Runtime matrix

| Capability | Executable aliases | Source call sites | Feature | Class | Current fallback | iOS relevance | Doctor behavior | Task 018 profile candidate |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Core startup | none | `yazi-fm/src/main.rs`, CLI entrypoint | Browse, configure, and run basic Yazi | Core | No external helper required | Required built-in capability | Always `available`; doctor itself does not probe a helper | Minimal |
| Filename search | `fd`, then `fdfind` | `yazi-plugin/preset/plugins/fd.lua` | Filename search | Feature | No helper fallback; actionable feature error | Relevant when installed | One logical capability; first available alias is selected | Recommended |
| Content search | `rg` | `yazi-plugin/preset/plugins/rg.lua` | Content search | Feature | No helper fallback; feature-local error | Relevant when installed | `rg` probe | Recommended |
| Interactive jump | `fzf` | `yazi-plugin/preset/plugins/fzf.lua` | Interactive selection/jump | Feature | No helper fallback; feature-local error | Relevant when installed | `fzf` probe | Recommended |
| Directory history | `zoxide` | `yazi-plugin/preset/plugins/zoxide.lua` | Directory-history jump | Feature | No helper fallback; missing spawn now reports `Failed to start`; empty output reports no history | Relevant when installed | `zoxide` probe | Recommended |
| MIME detection | `YAZI_FILE_ONE`, otherwise `file` | `mime-local.lua`, `file.lua`, `yazi-cli/src/env/env.rs` | MIME detection and generic classification | Feature | Placeholder MIME `null/file1-not-found`; preview shows a textual start error | Relevant when installed; override is especially useful on jailbreak PATHs | Probes only the effective override/default, never both as separate requirements | Minimal / Recommended |
| JSON preview | `jq` | `yazi-plugin/preset/plugins/json.lua` | JSON and NDJSON preview | Fallback | Falls back to `code` preview | Relevant when installed | `jq` probe | Recommended |
| Video frames | `ffmpeg` | `yazi-plugin/preset/plugins/video.lua` | Video frame preview | Feature | Preview reports a feature-local start/exit error | Relevant when installed | `ffmpeg` probe | Full |
| Video metadata | `ffprobe` | `yazi-plugin/preset/plugins/video.lua` | Duration and stream metadata | Feature | Preview reports metadata error; no startup impact | Relevant when installed | `ffprobe` probe | Full |
| PDF preview | `pdftoppm` | `yazi-plugin/preset/plugins/pdf.lua` | PDF image rendering | Feature | Preview reports conversion/start error | Relevant when installed | `pdftoppm` probe | Full |
| ImageMagick preview | `magick` | `magick.lua`, `font.lua` | AVIF/HEIF/JXL and font/image rendering | Feature | Affected formats report a feature-local error | Relevant when installed | `magick` probe | Full |
| SVG preview | `resvg` | `yazi-plugin/preset/plugins/svg.lua` | SVG rendering | Feature | Preview reports renderer/start error | Relevant when installed | `resvg` probe | Full |
| Archive helper | `7zz`, then `7z` | `yazi-plugin/preset/plugins/archive.lua` | Archive listing/extraction | Feature | One alias is sufficient; neither produces an actionable 7-zip error | Relevant when installed | One logical `7zip` capability; first available alias selected | Recommended |
| Terminal image fallback | `chafa` | `yazi-adapter/src/drivers/chafa.rs`, driver selection | Text-mode image preview | Fallback | Task 015 metadata/text fallback; terminal-native protocols remain available | Relevant as an optional SSH/terminal fallback | `chafa` probe | Recommended |
| Desktop compositor renderer | `ueberzugpp` | `yazi-adapter/src/drivers/ueberzug.rs` | X11/Wayland compositor image layer | Integration | Other drivers or Chafa/metadata fallback | Normally not applicable on iOS | `not_applicable` on iOS; otherwise probe | Full |
| Package manager | `git` | `yazi-cli/src/package/git.rs` | `ya pkg` clone/fetch/checkout | Feature | Package operation returns the git execution error | Relevant to `ya pkg`; never core startup | Explicit `git` row | Recommended |
| tmux integration | `tmux` | `yazi-emulator/src/mux.rs` | Passthrough, sixel/session integration | Integration | Terminal behavior falls back without tmux | Relevant on device or SSH when tmux is used | `tmux` probe | Full |
| zellij integration | `zellij` | emulator/session environment and `ya env` | Session integration | Integration | No zellij-specific behavior | Relevant if supported by the device session | `zellij` probe | Full |
| macOS clipboard | `pbcopy`, `pbpaste` | `yazi-widgets/src/clipboard/unix.rs` | Desktop clipboard | Integration | Other backend or in-process mirror | Not an iOS requirement | Optional rows; never required on iOS | Full |
| Android clipboard | `termux-clipboard-set`, `termux-clipboard-get` | `yazi-widgets/src/clipboard/unix.rs` | Android terminal clipboard | Integration | Other backend or mirror | Not an iOS requirement | Optional rows; never required on iOS | Full |
| Wayland clipboard | `wl-copy`, `wl-paste` | `yazi-widgets/src/clipboard/unix.rs` | Wayland clipboard | Integration | Other backend or mirror | Not required for native iOS clipboard | Optional rows; not required on iOS | Full |
| X11 clipboard | `xclip`, `xsel` | `yazi-widgets/src/clipboard/unix.rs` | X11 clipboard | Integration | Other backend or mirror | Not required for native iOS clipboard | Optional rows; not required on iOS | Full |
| Native iOS clipboard | none | `yazi-widgets/src/clipboard/ios.rs` | Native iOS pasteboard path | Integration | In-process mirror if native backend is silent | Relevant only on iOS | Built-in `available` on iOS; `not_applicable` elsewhere | Minimal |
| Editor opener | `${EDITOR}`, otherwise `vi` | `yazi-config/preset/yazi-default.toml` | User edit action | Configured | Existing shell opener policy; no menu removal | Relevant as a configured user action | Configured value is reported; unset fallback probes `vi` | Minimal |
| Shell action | `sh` on Unix; `cmd.exe` on Windows; inherited `SHELL` is context | `yazi-scheduler/src/process/shell.rs` | User shell actions | Configured | Existing platform shell behavior | `sh` is resolved through device `PATH` | Reports configured `SHELL` context without changing runtime policy | Minimal |
| Media opener | `mediainfo` | `yazi-config/preset/yazi-default.toml` | User-selected media information action | Configured | Menu entry remains; command error is user-action scoped | Relevant if installed | Optional configured-command row | Full |
| Metadata opener | `exiftool` | `yazi-config/preset/yazi-default.toml` | User-selected metadata action | Configured | Menu entry remains; command error is user-action scoped | Relevant if installed | Optional configured-command row | Full |
| Desktop opener | `xdg-open`, `open`, `start`, `termux-open` | default opener preset | Platform desktop/Android open action | Configured | Platform-specific preset selection | Not an iOS dependency | Documented as non-iOS platform paths | Full |
| iOS opener | `ya open` self-invocation plus native handoff | `yazi-config/preset/yazi-default.toml`, `yazi-cli/src/open` | iOS application opening | Integration | Native iOS handoff reports an actionable result | Relevant on iOS; no `open`/`xdg-open`/`termux-open` needed | Built-in path; no external opener probe | Minimal |

## Non-runtime commands found by the audit

These commands are not runtime dependencies and must not be included in an
iOS installer profile merely because they occur in the repository:

- Build-only: `cargo`, `rustc`, `git` used by `yazi-build/build.rs`, `tar`, and
  `zip` used by `yazi-build/src/dest.rs`.
- Test-only: `sh`, `echo`, and `pwd` in `yazi-plugin/tests/lua_runtime.rs`;
  test fixtures do not define device dependencies.
- Self-invocation: `yazi` in `ya env`, and `ya` in iOS/default opener entries.
  These refer to Yazi/Ya themselves, not third-party packages.
- Runtime shell-string commands: `ls` embedded in the zoxide/FZF preview
  options, plus shell syntax/built-ins such as `read`, `pause`, `start`, and
  `command` in configured opener/plugin strings. They are interpreted only
  when that feature invokes its configured shell; they are not direct doctor
  capabilities and do not become startup requirements.
- Windows-only opener programs such as `code` are preset user actions, not
  Yazi core dependencies.

## Probe contract

`yazi-cli/src/capability/capability.rs` is the shared probe layer used by
`ya doctor` and the dependency portion of `ya env`. It resolves names directly
through `std::env::split_paths`, checks executable permissions on Unix, and
handles Windows executable extensions without invoking `which`, `command -v`,
or `where`. It preserves non-UTF-8 `OsString` values while rendering lossy
diagnostics only at the output boundary.

Resolution and version probing are separate. A resolved executable remains
`available` when its harmless version/help argument exits nonzero or produces
no useful line; `version` is then `null`. Probes use no interactive or
destructive arguments. The report includes only the effective `PATH` plus the
effective `YAZI_FILE_ONE` capability metadata; it never serializes the full
environment.

## Evidence

- Source-verified: call sites, aliases, fallback branches, iOS cfg paths, and
  preset commands.
- Host-runtime-validated: `ya doctor`, `ya doctor --json`, synthetic PATH and
  alias tests, and the zoxide missing-binary test.
- Compile-validated: host package checks and the target CI result reported in
  the Task 017 handoff.
- Device-unverified: execution of any listed helper on a jailbroken device,
  package availability, jailbreak repository names, and rootful/rootless
  installation paths.
