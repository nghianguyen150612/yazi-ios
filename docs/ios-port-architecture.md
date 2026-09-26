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
| Companion CLI | `yazi-cli/src/main.rs`, `yazi-cli/src/args.rs` | Provides `ya` version/environment, DDS emit/exec, package commands, and `ya open`; eventual `ya-ios` name is deferred |
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
| Clipboard | `yazi-widgets/src/clipboard/{clipboard,lookup,unix,ios,windows}.rs` and terminal clipboard sequences | iOS has a native device-pasteboard backend; the desktop/Android helper probes are excluded there |
| Opener and external integrations | `yazi-config/src/open`, `opener`, preset rules, `yazi-cli/src/env/env.rs` | `xdg-open`, `open`, Windows, and Termux rules do not define an iOS opener; runtime integration must be explicit |
| Mount/device discovery | `yazi-fs/src/mounts` | Linux and macOS monitor implementations exist; iOS needs a provider or a deliberate empty capability set |
| Package manager | `yazi-cli/src/package` | Git, filesystem, archive, hashing, and deploy steps are optional runtime capabilities and not a reason to block launch |
| Shared paths, shell syntax, and metadata | `yazi-shared`, `yazi-shim` | Unix path/string conversion still needs iOS auditing; Task 002 isolates current user/group identity behind a native iOS lookup adapter |

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
   `/dev/tty`. These are the primary SSH and jailbroken-terminal seams. Task 007
   resolved the descriptor-waiting half of this seam for iOS; see the Darwin
   terminal waiting strategy below.
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
    Android, while the clipboard implementation used desktop/Termux/X11/Wayland
    commands plus OSC 52. The clipboard half was resolved by Task 009; openers
    still have no iOS policy.
11. **Lua and native helpers.** The default `yazi-fm` feature uses vendored Lua, and
    native image/process helpers are spread across several crates. Their iOS build
    and runtime behavior must be tested independently rather than hidden behind a
    feature reduction.

## Task 002 identity adapter

Yazi's only `uzers` path is the direct Unix dependency from `yazi-shim`; Yazi
uses it for the current UID/GID and UID/GID-to-name lookups. Version 0.12.2 does
not define its `UserExtras` or `GroupExtras` types for iOS, and its iOS
`getgrouplist` path also passes unsigned GIDs to the signed Apple ABI.
`uzers 0.12.2` is the newest published release, so a version update cannot
remove this blocker.

`yazi_shim::Uzers` now keeps the existing facade while selecting a narrow
backend:

- regular Unix continues to use the existing cached `uzers` implementation;
- iOS uses `getuid`, `getgid`, `getpwuid_r`, and `getgrgid_r` directly from
  `libc`;
- caller-owned lookup buffers grow on `ERANGE`, unknown IDs and lookup errors
  map to the facade's existing `None` result, and returned names are copied as
  lossless OS bytes before the buffer is released.

Yazi does not consume group-membership lists, so the iOS adapter does not
implement `getgrouplist`. The real IDs remain available to XDG runtime/temp
isolation, secure-directory ownership checks, filesystem metadata, and the
`ya.uid()`, `ya.gid()`, `ya.user_name()`, and `ya.group_name()` Lua APIs. This
is an identity adapter, not a claim that the full filesystem subsystem builds
or has run on a physical device; target CI and real-device validation remain
separate evidence.

## Task 007 Darwin terminal waiting strategy

Terminal event waiting is a reusable Apple-wide strategy rather than an iOS-only
special case, so it is recorded here as the seam other Apple subsystems should
follow when they must observe a terminal descriptor.

`yazi-term` waits on exactly three descriptors through one private `poll`
abstraction: the terminal input descriptor, the SIGWINCH pipe, and the waker
pair. Apple targets must use `select(2)`, because `poll(2)` cannot report
readiness for devices and a tty is a character device. Apple documents this in
the `poll(2)` BUGS section of both its macOS and its iOS manual pages, so the
restriction is a Darwin kernel property that iOS inherits rather than a macOS
quirk. Selecting a wait primitive by `target_os = "macos"` alone therefore left
iOS on a path that cannot observe its own terminal.

The strategy generalizes to any Apple target. Rustix gates `select` and the
`FdSet` helpers on its `bsd` feature, which its `build.rs` enables for `macos`,
`ios`, `tvos`, `visionos`, and `watchos`, so one `select`-based path can serve
all of them with no added dependency. `select3` is written against the
descriptors Yazi's TTY abstraction already supplies and keeps the poll interface
of exactly three descriptors, so the change is confined to the `cfg`
predicate. Consumers must not branch on platform themselves.

Two consequences are worth carrying forward. First, a Darwin limitation should
be verified against Apple documentation for the specific target rather than
inferred from a sibling platform, which is what Task 007 did. Second, the
host-side tests use socket pairs because both primitives report sockets, so
they validate the abstraction's contract but never the device limitation
itself; only a real terminal descriptor on a physical device closes that gap.

## Anticipated iOS adapters

These are boundaries for the remaining port tasks. The identity adapter above
is the only implementation recorded by Task 002.

### Filesystem and platform paths

Keep `Url`, `UrlBuf`, `Path`, and the filesystem engine contracts intact. Add an
iOS path/provider layer for rootful and rootless jailbreak layouts, accessible
storage locations, device data, and any permission or sandbox boundary. It should
report capabilities and actionable errors instead of silently rewriting logical
URLs. The same adapter must be used by the CLI, Lua filesystem API, scheduler, and
configuration/runtime directories.

### Trash

Upstream `trash` crate (v5.2.8) does not support iOS (`cannot find module or crate platform in this scope`).
Task 003 removes the external `trash` crate dependency for `target_os = "ios"` and replaces it with a Yazi-owned
iOS Trash backend in `yazi-fs` under `src/trash/ios/`.

The iOS Trash backend architecture:
- **Trash Root Location**: Resolves dynamically using `dirs::home_dir()` (e.g., `~/.local/share/Trash`), containing `files/` and `info/` subdirectories. This ensures consistent CLI/SSH behavior for both root and mobile users on jailbroken iOS devices.
- **Metadata Format**: Follows Freedesktop `.trashinfo` specification (`[Trash Info]` section with percent-encoded `Path=` and ISO-formatted `DeletionDate=`). Path traversal sequences (`../`) in metadata are strictly validated and rejected.
- **Collision Resolution**: When trashing an item whose basename already exists in the Trash `files/` directory, unique filenames (`foo 1.txt`, `foo 2.txt`) are generated while preserving original path metadata.
- **Move & Copy Fallback**: Uses atomic `fs::rename()` to move items into Trash `files/`. If `fs::rename()` fails with cross-device link error (`EXDEV`), a recursive copy + verify + remove fallback preserves user data across different mounts/filesystems.
- **Management Operations**: Implements `list`, `entry`, `metadata`, `revalidate`, `remove_file`, `remove_dir`, `restore`, `rename`, and `empty` in `yazi-fs/src/trash/ios/trash.rs`. Restoring an item returns it to its original path (creating parent directories if necessary) and refuses to overwrite existing files.

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
provider. Task 010 implemented this; the policy it settled is recorded below.

### Clipboard

Provide a local-device clipboard adapter for the jailbreak environment and retain
OSC 52/terminal capability handling. In an SSH session, preserve the upstream
remote-terminal behavior: prefer the terminal/remote path when appropriate and keep
an in-process fallback when the remote side cannot answer. Clipboard failure must
not make normal file operations fail. Task 009 implemented this; the policy it
settled is recorded below.

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

## Task 008 process capability policy

Task 008 settled the process backend policy, so it is recorded here as the rule
for this seam rather than as an anticipated adapter.

All process execution passes through one function, `shell()` in
`yazi-scheduler/src/process/shell.rs`. That is deliberate: blocking `:shell`,
background, orphan, openers, hooks, and the configured editor all derive their
working directory, stdio, and detachment from a single `ShellOpt`, so a
platform-specific backend belongs there and nowhere else.

The policy is to keep the Unix `fork`+`exec` path on iOS, and to report failure
precisely rather than to substitute a different spawn mechanism. The reasoning
is that Yazi-iOS targets the jailbroken command-line environment, where a shell
and ordinary POSIX process creation are the premise, and where trading a working
path for an unverifiable one would be a regression. Concretely:

- Do not reach for `posix_spawn` on iOS. The `setsid()` that detaches background
  and orphan commands must run between `fork` and `exec`, and the standard
  library offers no stable way to express a new session through `posix_spawn` on
  Darwin at all.
- Do not replace `setsid()` with `process_group(0)`. It is stable and does map to
  `POSIX_SPAWN_SETPGROUP`, but a new process group is a weaker guarantee than a
  new session, and the difference cannot be measured without a device.
- Do not pin the shell to an absolute path. It stays resolved through `PATH`, so
  rootful and rootless environments both work.
- Do not assume App Store process policy describes a jailbroken device. Apple's
  iOS manual pages omit `fork(2)`, but that reflects the supported app API
  surface rather than a removed kernel capability, and the port targets the
  jailbroken environment deliberately.
- Do not disable process support on iOS, and do not let a failure pass silently.
  When creation is denied, the diagnostic names the cause.

Compile success is not runtime evidence. `fork`+`exec`, `setsid`, and detachment
must each be confirmed on a physical jailbroken device over both SSH and a local
terminal before any supported claim is made.

### Allocator, FFI, and IPC

Choose an allocator deliberately for iOS instead of inheriting the broad
non-macOS/non-Windows condition. Audit every Apple framework and shared-memory path
against the iOS SDK and deployment model. DDS should either use a validated iOS
transport or report that cross-instance communication is unavailable; its Unix
socket is not an implicit requirement.

## Task 009 Apple pasteboard adapter policy

Task 009 turned the anticipated clipboard adapter into an implementation, and in doing
so established a policy for reaching an Apple-only API from a jailbreak command-line
process. That policy is reusable, so it is recorded here as the rule for this seam.

**Where the adapter lives.** A framework binding belongs in `yazi-ffi`, next to the
existing macOS Core Foundation and IOKit modules, and the caller keeps the platform
decision to itself. `yazi-ffi/src/pasteboard.rs` is the whole iOS surface: it owns the
Objective-C calls, the autorelease pool, and the "can this process reach the pasteboard
at all" probe, and it exposes plain `&str`/`Vec<u8>` values. Nothing above it sees an
Objective-C type, so a future opener, share sheet, or `ya` subcommand can reuse the
adapter instead of growing a second one. The consumer,
`yazi-widgets/src/clipboard/ios.rs`, owns only Yazi's policy: which clipboard answers, in
which order, and how bytes become text.

**Which API.** `UIPasteboard` is the only public pasteboard API on iOS. `NSPasteboard` is
AppKit, and the Core Foundation pasteboard calls behind `pbcopy`/`pbpaste` on macOS are
not part of the iOS SDK, so there is no Foundation-only or CoreFoundation-only
alternative and no reason to reach for a private API. It needs no `UIApplication`
lifecycle, which is what makes it usable from a process that has no app bundle, but it
does assume an autorelease pool that nothing in a Rust CLI provides, so the adapter
pushes one around every call.

**Which binding.** The workspace already depends on `objc2` for macOS disk arbitration,
and `objc2-ui-kit` with only its `UIPasteboard` feature is the maintained binding for this
class. Typed bindings were preferred over handwritten `msg_send!` because they own the
retain/release and `NSString` conversion a hand-rolled version would get wrong. The added
crates are `target_os = "ios"`-only, so no other build, including the macOS build that
already uses `objc2`, changes. `objc2-foundation` is requested with
`default-features = false` and only the `std`/`NSString` features, because its default
feature set is every Foundation class.

**Availability is probed, not assumed.** `objc2-ui-kit` links UIKit strongly, so a system
that cannot load it would fail at launch rather than at the pasteboard. The adapter
therefore looks the class up through the Objective-C runtime first and reports unavailable
instead of panicking. Whether a non-app process can load UIKit at all on a jailbroken
device, and whether a read shows the system paste prompt, are the two questions a build
cannot answer.

**Failure is a capability, not an error.** The pasteboard is auxiliary. Every failure mode
— no text on it, an image-only pasteboard, a locked device, a declined read prompt, a
missing class — resolves to the in-process mirror, which is recorded verbatim on every
`set()`. There is no retry, no panic, and no user-facing diagnostic, which matches how the
desktop backend already degrades when no helper command is installed. The pasteboard
daemon is reached over XPC and the first call in a process can block noticeably, so both
directions run on a blocking thread rather than the thread that drives the UI.

Compile success is not runtime evidence. Reading and writing the device pasteboard from a
jailbreak terminal, and the paste prompt, must be confirmed on a physical device in both
a local terminal and an SSH session.

## Task 010 application opener adapter policy

Task 010 turned the anticipated opener adapter into an implementation, and in
doing so extended the Task 009 rule — a framework binding belongs in
`yazi-ffi`, the caller keeps the platform decision — to a *command* rather than
an in-process operation. That is the one structural decision worth stating
first, because it is the one that shaped everything else.

**Why the seam is a `ya` subcommand.** An opener rule is a command string. The
alternatives were to branch inside `yazi-actor/src/mgr/open_do.rs`, or to add a
new kind of process to the scheduler. Both would make the opener system an iOS
special case and both would duplicate machinery that already works. Instead the
iOS rows in the default preset name `ya open`, and `yazi-cli/src/open/` is the
seam: `yazi-ffi/src/launch.rs` owns the Objective-C calls, `open/handoff.rs`
owns Yazi's policy, and `open/open.rs` owns the argument list. The rule is still
matched by `Platform`, still expanded by `Splatter`, and still run through
`ShellOpt` as a background process, so a failure lands in Yazi's task list
through the path that already exists for `xdg-open`. This is also why the
existing default rules could name `ya pub extract` and `ya emit download`
without anything new: an opener rule naming a `ya` subcommand is an established
pattern in this preset, not a Task 010 invention.

**Which API.** Not `UIApplication`, and not `UIDocumentInteractionController` or
`UIActivityViewController`: all three need an app lifecycle or a view, which a
process without a bundle does not have. `LSApplicationWorkspace` in LaunchServices
is a private CoreServices class that a daemon, an SSH session, and a terminal
can all reach, and it is what the jailbreak `uiopen` family is built on. It was
chosen over SpringBoard's `SBSOpenSensitiveURLAndUnlock` for one reason that
matters more than either of them: it returns a `BOOL`, so "nothing on this
device handles that document" is a fact rather than a guess.

**How the private surface is contained.** The framework is opened with `dlopen`
from a `/System/Library/Frameworks/…` path, which is in the read-only system
volume and therefore identical on rootful and rootless devices; the class is
looked up through the Objective-C runtime rather than linked, so a system that
does not have it reports unavailable instead of failing at launch; each selector
is checked with `respondsToSelector:` before it is sent; and an autorelease pool
is pushed around the call. This is the port's only private API, and it is
recorded as such rather than presented as a supported one.

**Why there is no helper fallback.** The preferred ladder was native → optional
detected helper → clear unsupported error, and the middle rung was investigated
rather than skipped. `uiopen` is the obvious candidate, but its documented and
observed use is bundle identifiers and URL schemes, and its acceptance of a
`file://` path is inconsistent across versions. Depending on it would make a
third-party package a de-facto requirement for something the system already
does, and it would require a `file://` URL that cannot carry a non-UTF-8
filename. The ladder is therefore native → clear unsupported error, and the
missing middle rung is a recorded decision rather than an omission. A future
task that finds a well-supported helper may add it *below* the native path
without changing this seam.

**Reveal is not `open -R`.** iOS exposes no way to select or highlight an item
inside a folder, so "reveal" on iOS means handing the containing folder to
Files. That is what the default rule does, what Android's existing
`termux-open %d1` row already does, and what the option is named for. The rule
must not be tightened into a claim it cannot keep.

**Encoding.** The handoff is a filesystem path, never a URL string:
`File::content_path()` already resolves an archive member to its backing file
and a remote URL to its local cache path, and `NSURL` is built through
`fileURLWithFileSystemRepresentation:` so the file's own bytes cross the
boundary. No `to_str()`, no `unwrap()`, and no percent-encoding is involved.

Compile success is not runtime evidence. Handing a JPEG, a PDF, a folder, and a
file with spaces or non-ASCII bytes to LaunchServices from a shell process, over
a local terminal and over SSH, must be confirmed on a physical jailbroken device
before any supported claim is made.

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
default binaries. It also permits an explicit `workflow_dispatch` on `iOS` while
retaining the Task 001 push/pull-request checks for `main`. It does not modify or
bypass the existing Linux, macOS, Windows, formatting, or lint workflows. The
build step is intentionally not marked `continue-on-error` and does not use
`|| true`: a red result is the first genuine baseline signal.

Task 001 makes documentation and CI changes only. It does not add an iOS trash,
watcher, clipboard, opener, process, FFI, or Lua implementation, and it does not
rename upstream internals or remove features to manufacture a green build.
