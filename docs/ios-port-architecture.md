# Yazi-iOS port architecture

## Status and scope

This is the Task 001 architecture baseline for the Yazi-iOS port. It describes the
shape of the port and the seams that later tasks may use; it does **not** claim that
Yazi-iOS currently builds or runs on iOS.

The port is intended to preserve upstream Yazi's behavior and feature set. The
primary deployment is a jailbroken iOS device reached over a terminal, with SSH as
a first-class use case. The port must not become a reduced “Yazi Lite” fork.

The baseline source is an upstream-identical tree:

- Working branch at the start of Task 001: `main`
- Starting commit: `b8973fb4c2b9b184aad03cfc717c1fefe4140c40`
- Starting commit subject: `fix: correct diacritic input for \`Option\` key combos in kitty keyboard protocol (#4346)`
- Starting commit date: `2026-09-12T08:02:46+08:00`
- Local tree: `d220c8d81f9ac1ae4236366e57237f89e178b103`
- Upstream project: <https://github.com/sxyazi/yazi>
- GitHub parent of this fork: `sxyazi/yazi`
- Local remote: `origin` → `https://github.com/nghianguyen150612/yazi-ios.git`
- Workspace version: `26.9.1`
- Workspace minimum Rust version: `1.95.0`

The repository has no separately configured `upstream` remote. GitHub identifies
`nghianguyen150612/yazi-ios` as a fork of `sxyazi/yazi`, and the upstream commit API
returns the same commit and tree hashes as the local `HEAD`. Task 001 therefore
starts from an existing, previously unmodified upstream fork rather than from an
already ported tree.

The current source still names the binaries and package `yazi` and `ya`. The
future product names `yazi-ios` and `ya-ios` are packaging/runtime names for a later
installer task; Task 001 does not perform a broad internal rename.

## Targets and toolchain

| Target | Role | What it proves | What it does not prove |
| --- | --- | --- | --- |
| `aarch64-apple-ios` | Primary runtime target for modern arm64 iOS devices | The real device architecture, Apple SDK headers, Rust target linkage, and all target-selected code | Jailbreak APIs, entitlements, process policy, or behavior on a physical device |
| `aarch64-apple-ios-sim` | Optional arm64 iOS Simulator target | Fast compile/link feedback on Apple Silicon and simulator-only smoke tests | Device-only frameworks, jailbreak integration, real TTYs, or SSH from a device |
| Host macOS | Build host for both targets | The installed Xcode/iOS SDK and Apple linker are available | Anything about an iOS runtime that was not executed |

The required Rust installation is a stable toolchain meeting the workspace's
`rust-version = 1.95.0`, with the `aarch64-apple-ios` standard-library target
installed. A useful developer setup is:

```sh
rustup toolchain install stable --profile minimal --target aarch64-apple-ios
rustup default stable
rustc --version --verbose
cargo --version
xcrun --sdk iphoneos --show-sdk-path
```

CI must run on a macOS runner with an installed Apple SDK. A Linux host can inspect
and validate the portable workspace, but it cannot honestly validate an iOS link.
The new `iOS Baseline` workflow therefore performs the first target build on macOS
and leaves its result visible, including a failing result when upstream code has not
yet been adapted.

The baseline workflow builds both default executables for the device target:

```sh
cargo build --locked --target aarch64-apple-ios --package yazi-fm --package yazi-cli
```

The simulator target is deliberately not treated as a substitute for the device
build. It may be added as a separate compile-only job in a later task, after the
device build blockers are understood.

## Port philosophy

Every subsystem belongs to one of three categories.

### 1. Portable upstream code

Keep upstream architecture, types, task flow, configuration, Lua API, and user-facing
behavior unchanged whenever the existing implementation can run on iOS. A source
audit is not a runtime validation: a row marked portable in the parity document
means that no platform-specific implementation was identified, not that it has
already been tested on iOS.

### 2. iOS platform adapters

Put unavoidable operating-system differences behind a small, explicit seam. An
adapter should preserve the upstream operation and error semantics where possible,
expose capability information where behavior differs, and avoid leaking iOS APIs
through portable configuration, parser, actor, or scheduler code.

The adapter should normally live at the lowest shared layer that owns the operation
(for example, the filesystem engine or terminal capability layer), not in a
platform-specific rewrite of the file manager.

### 3. Optional runtime capabilities

External command-line programs and terminal protocols are capabilities, not startup
requirements. Yazi-iOS may launch, browse files, edit configuration, and use SSH
without every optional helper being installed. A missing helper should disable or
degrade only the operation that needs it and produce a useful, non-fatal diagnostic.

## Workspace and subsystem map

The workspace is a Rust 2024 Cargo workspace with 31 `yazi-*` crates. The default
members are `yazi-fm` and `yazi-cli`; the remaining crates are libraries used by
those executables or by the build/package tooling.

| Area | Upstream implementation and entry points | iOS relevance |
| --- | --- | --- |
| File-manager executable | `yazi-fm/src/main.rs`; initializes shim, shared state, TTY, emulator, filesystem, VFS, Lua runner, image adapter, widgets, watcher, actors, and the app | Main runtime entry point; allocator, terminal, process, and platform initialization are high-risk |
| Companion CLI | `yazi-cli/src/main.rs`, `yazi-cli/src/args.rs` | Provides `ya` version/environment, DDS emit/exec, and package commands; eventual `ya-ios` name is deferred |
| Bootstrap and arguments | `yazi-boot`, `yazi-fm/src/root.rs`, `yazi-fm/src/executor.rs` | Startup path, client identity, working directory, and command-line behavior should remain portable |
| Build/package helper | `yazi-build` (`xtask` alias), `yazi-packing` | Build profiles, release staging, `.deb`, and archive behavior are desktop/release concerns; do not pull packaging into the device port |
| Core state and UI flow | `yazi-core`, `yazi-actor`, `yazi-fm`, `yazi-parser`, `yazi-proxy` | Mostly portable event/state logic; keep platform calls behind existing proxies |
| TTY and terminal I/O | `yazi-tty`, `yazi-term`, `yazi-tui` | Unix file descriptors, `/dev/tty`, termios, resize signals, terminal modes, and process stdio need device/SSH validation |
| Terminal capability detection | `yazi-emulator` (`Brand`, `Emulator`, `Probe`, `Mux`) | Existing probes are the model for capability-based graphics, clipboard, keyboard, and mux support; do not assume an iOS terminal protocol |
| Filesystem and paths | `yazi-fs` (`engine`, `path`, `file`, `cha`, `cwd`, `xdg`, `mounts`, `trash`) | Local POSIX-like APIs are likely reusable, but jailbreak path roots, sandbox restrictions, case folding, permissions, mounts, and trash are platform-sensitive |
| File watcher | `yazi-watcher`, `notify` with `macos_fsevent` feature, `PollWatcher` fallback | FSEvents availability, sandbox access, jailbreak filesystem notifications, and polling behavior need a real-device decision |
| Task scheduler and file operations | `yazi-scheduler` (`file`, `process`, `fetch`, `preload`, `hook`, `size`, `plugin`) | Preserves asynchronous progress, cancellation, hooks, and external process semantics; shell spawning is a major iOS concern |
| Process and shell integration | `yazi-scheduler/src/process`, `yazi-shared/src/shell`, `yazi-binding/src/process` | Uses `sh`, `cmd.exe`, libc process setup, inherited stdio, and background/orphan modes; capability adapter required rather than disabling the scheduler |
| Image decoding and terminal graphics | `yazi-adapter`, drivers `kgp`, `kgp_old`, `iip`, `sixel`, `ueberzug`, `chafa` | Protocols should be selected by probes; external tools and local compositor support are optional |
| Previewers and fetchers | `yazi-runner`, `yazi-plugin/preset/plugins`, `yazi-config` plugin rules | Text, image, directory, PDF, video, archive, JSON, SVG, and custom Lua preview/fetch paths must remain feature-complete where capabilities exist |
| Lua/plugin runtime | `yazi-plugin`, `yazi-runner`, `yazi-binding`, `yazi-shim/mlua` | Lua is a major product feature; vendored Lua compilation and every binding/process helper need iOS validation |
| Configuration and keymap | `yazi-config`, preset TOML, `yazi-fm/src/input`, `yazi-parser` | TOML/keymap/theme/layout behavior is portable; opener rules and platform selectors need an iOS policy |
| VFS and remote files | `yazi-vfs`, `yazi-sftp`, `yazi-vfs/src/engine/{lua,sftp}` | SFTP, Lua providers, HTTP writes, cache/stamp paths, and Unix authentication agents need device and SSH testing |
| DDS and cross-instance state | `yazi-dds` | Uses a Unix-domain socket and persistent state; verify jailbreak filesystem and lifecycle behavior |
| FFI and shared memory | `yazi-ffi` (`shm`, macOS disk arbitration/IOKit modules) | `cfg(target_os = "macos")` modules do not automatically cover iOS; Unix shared memory, IOKit, and entitlements need separate decisions |
| Clipboard | `yazi-widgets/src/clipboard.rs` and terminal clipboard sequences | Unix tries `pbcopy`/Termux/Wayland/X11 tools and emits OSC 52; iOS needs a bridge while preserving SSH behavior |
| Opener and external integrations | `yazi-config/src/open`, `opener`, preset rules, `yazi-cli/src/env/env.rs` | `xdg-open`, `open`, Windows, and Termux rules do not define an iOS opener; runtime integration must be explicit |
| Mount/device discovery | `yazi-fs/src/mounts` | Linux and macOS monitor implementations exist; iOS needs a provider or a deliberate empty capability set |
| Package manager | `yazi-cli/src/package` | Git, filesystem, archive, hashing, and deploy steps are optional runtime capabilities and not a reason to block launch |
| Shared paths, shell syntax, and metadata | `yazi-shared`, `yazi-shim` | Unix path/string conversions and user/UID lookups are cross-platform code that must be audited at the iOS boundary |

## Platform-sensitive findings from the baseline

The repository uses `cfg(unix)` extensively. For an Apple Unix target, that does
not imply that desktop Unix facilities are available or permitted. Conversely,
`cfg(target_os = "macos")` excludes iOS even when both platforms use some Apple
frameworks. iOS therefore needs explicit `target_os = "ios"` decisions at the same
seams where macOS and Android already differ.

The most important observations for later tasks are:

1. **Allocator selection.** `yazi-fm/src/main.rs` and `yazi-fm/Cargo.toml` select
   `tikv-jemallocator` for every target that is neither macOS nor Windows. That
   condition includes `aarch64-apple-ios`; allocator support, linker behavior, and
   memory policy must be checked rather than assumed from desktop builds.
2. **Filesystem case folding.** `yazi-fs/src/engine/local/casefold.rs` has explicit
   implementations for macOS, Windows, Linux/Android, NetBSD, and OpenBSD, but no
   iOS branch. The baseline build is expected to expose this or another concrete
   compile blocker; the eventual fix should be an iOS filesystem adapter or a
   deliberately portable fallback.
3. **Trash.** `yazi-fs/build.rs` marks iOS and Android as
   `trash_unsupported`, and the crate has an unsupported trash implementation.
   `yazi-fs/src/engine/local/local.rs` has separate Android, macOS, and generic
   Unix paths, so the actual local trash operation and the trash browsing API need
   to be reconciled during the trash task rather than inferred from the alias.
4. **Mount monitoring.** `yazi-fs/src/mounts` contains Linux and macOS monitor
   implementations only. The generic partition structure exists, but iOS device and
   volume discovery is not implemented.
5. **Watcher backend.** `yazi-watcher` enables the `macos_fsevent` feature of
   `notify`, creates a `RecommendedWatcher`, and retains a polling alternative. The
   source does not establish whether the FSEvents backend is usable in the target
   sandbox or on a jailbroken device; initialization and runtime behavior must be
   measured on-device.
6. **Terminal and SSH.** `yazi-term` uses Unix termios, signal hooks, poll/select,
   Unix stream pairs, and `/dev/tty`; `yazi-tty` opens standard file descriptors or
   `/dev/tty`. These are the primary SSH and jailbroken-terminal seams.
7. **Process spawning.** The scheduler launches `sh -c` and calls
   `libc::setsid` in `pre_exec` for detached commands. iOS process policy, shell
   availability, stdio inheritance, signals, and background/orphan semantics may
   differ substantially from a desktop Unix host.
8. **FFI and IPC.** `yazi-ffi` has macOS-only Core Foundation/Objective-C modules
   and a Unix shared-memory implementation. `yazi-dds` binds a Unix socket. Neither
   should be considered iOS-valid merely because `cfg(unix)` selects a source file.
9. **External tools.** `yazi-cli/src/env/env.rs` probes `file`, `ueberzugpp`,
   `ffmpeg`, `ffprobe`, `pdftoppm`, `magick`, `fzf`, `fd`/`fdfind`, `rg`, `chafa`,
   `zoxide`, `7zz`/`7z`, `resvg`, `jq`, and clipboard commands. These probes are
   diagnostics, not launch requirements.
10. **Openers and clipboard.** Default opener rules cover Linux, macOS, Windows, and
    Android, while the clipboard implementation uses desktop/Termux/X11/Wayland
    commands plus OSC 52. Neither area has an iOS policy yet.
11. **Lua and native helpers.** The default `yazi-fm` feature uses vendored Lua, and
    native image/process helpers are spread across several crates. Their iOS build
    and runtime behavior must be tested independently rather than hidden behind a
    feature reduction.

## Anticipated iOS adapters

These are boundaries for Tasks 002–020, not implementations in Task 001.

### Filesystem and platform paths

Keep `Url`, `UrlBuf`, `Path`, and the filesystem engine contracts intact. Add an
iOS path/provider layer for rootful and rootless jailbreak layouts, accessible
storage locations, device data, and any permission or sandbox boundary. It should
report capabilities and actionable errors instead of silently rewriting logical
URLs. The same adapter must be used by the CLI, Lua filesystem API, scheduler, and
configuration/runtime directories.

### Trash

Expose a capability-aware trash operation with the upstream browse, metadata,
restore, remove, rename, and empty operations where the device can provide them.
If a rootful/rootless device cannot offer a compatible trash, return an explicit
unsupported/permission result and keep ordinary delete/copy/move available. Do not
make trash support a prerequisite for starting Yazi-iOS.

### File watcher

Preserve the existing local/virtual watcher model. Investigate an iOS-appropriate
notification source (for example, a platform notification service when available)
and retain polling as a capability-based fallback. The adapter should report
whether watching is native, polling, or unavailable and should degrade directory
refresh behavior without losing the rest of the file manager.

### Opener

Replace the absence of an iOS opener rule with a small capability provider that can
hand a file to an installed jailbreak/application integration, share/open URL, or
report that no opener exists. It must not assume that `open`, `xdg-open`, or
Termux is present. Bulk open/reveal behavior should be defined in terms of the same
provider.

### Clipboard

Provide a local-device clipboard adapter for the jailbreak environment and retain
OSC 52/terminal capability handling. In an SSH session, preserve the upstream
remote-terminal behavior: prefer the terminal/remote path when appropriate and keep
an in-process fallback when the remote side cannot answer. Clipboard failure must
not make normal file operations fail.

### Terminal capability detection

Reuse and extend `yazi-emulator`'s probe model. Detect keyboard protocols, cell
size, color/background, clipboard, mouse, graphics, mux, and terminal identity at
runtime. A local iOS terminal, an SSH client terminal, a multiplexer, and a plain
pipe must each be able to produce a different capability set. Unsupported
capabilities should be represented as unavailable, not guessed from a product name.

### Mount and device discovery

Keep the `Partitions`/mount manager contract, but provide an iOS implementation or
an explicit empty provider backed by the available device/volume APIs. Do not
silently reuse Linux `/proc` or desktop macOS disk-arbitration code. Mount discovery
should be optional: browsing a known path must work when no provider is available.

### Process and shell integration

Make shell execution a capability. Preserve blocking, background, orphan, stdio,
progress, cancellation, and hook behavior when a permitted shell exists. If iOS
does not permit ordinary process creation, return a clear capability error and keep
the UI and non-process tasks usable. SSH execution must be considered separately
from local-device execution rather than hidden behind a desktop `sh` assumption.

### Allocator, FFI, and IPC

Choose an allocator deliberately for iOS instead of inheriting the broad
non-macOS/non-Windows condition. Audit every Apple framework and shared-memory path
against the iOS SDK and deployment model. DDS should either use a validated iOS
transport or report that cross-instance communication is unavailable; its Unix
socket is not an implicit requirement.

## Runtime capability philosophy

The following helpers may be installed independently on a device or supplied by an
SSH environment:

- `file` and other MIME detectors
- `fd` and `rg`
- `fzf`
- `zoxide`
- `ffmpeg` and `ffprobe`
- archive tools such as `7z`/`7zz`, `bsdtar`, or an equivalent
- PDF tools such as `pdftoppm`
- ImageMagick/`magick` or an equivalent
- `jq`
- `resvg`
- `chafa`, `ueberzugpp`, and other preview helpers
- `git` for package management
- shell, editor, archive, media, and user-configured opener commands

The availability check should happen at the operation or preview boundary. Use
bounded process startup, capture useful stderr, and return a non-fatal capability
result. A missing helper must not prevent Yazi-iOS from launching, loading a
configuration, browsing local files, or using supported SSH operations. The
`ya env` diagnostic can enumerate versions, but it is not a gate.

## Terminal and SSH philosophy

SSH is a first-class deployment mode, not an incidental environment variable.
Yazi-iOS should work over a local jailbroken-device terminal and over an SSH
connection, with behavior determined by the actual terminal and remote
capabilities.

Graphics selection should follow this order:

1. A supported protocol is detected and responsive → use that protocol.
2. No supported graphics protocol is available → use a viable text or external
   renderer if present.
3. No viable graphics path is available → continue with a non-graphical preview
   or a clear empty/fallback preview.

The port must not hardcode an assumption that a local iOS terminal supports Kitty,
iTerm2, Sixel, Überzug++, or any other image protocol. Likewise, SSH may expose a
different protocol from the local terminal, and a multiplexer may require a
passthrough path. Capability probes, timeouts, and clean fallback are more
important than protocol branding.

The same capability model should cover input encoding (including CSI u), mouse
events, resize handling, clipboard, drag-and-drop, bracketed paste, and terminal
restore. Real-device validation must include at least one local terminal and one
SSH session; simulator or host-terminal success is not a substitute.

## CI and change boundary

`.github/workflows/ios.yml` adds an isolated macOS job that prints the Apple and
Rust environment, installs `aarch64-apple-ios`, and runs the locked build for both
default binaries. It does not modify or bypass the existing Linux, macOS, Windows,
formatting, or lint workflows. The build step is intentionally not marked
`continue-on-error` and does not use `|| true`: a red result is the first genuine
baseline signal.

Task 001 makes documentation and CI changes only. It does not add an iOS trash,
watcher, clipboard, opener, process, FFI, or Lua implementation, and it does not
rename upstream internals or remove features to manufacture a green build.
