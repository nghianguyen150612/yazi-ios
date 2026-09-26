# Yazi-iOS feature parity baseline

## How to read this document

This is the Task 001 inventory of behavior found in the upstream tree at
`b8973fb4c2b9b184aad03cfc717c1fefe4140c40`. It is a tracking baseline, not a
support claim. No row is marked **Supported** until it has been compiled and
exercised on the intended iOS runtime, with the relevant external capabilities
available.

The status vocabulary is:

- **Uninvestigated** — the feature is present in upstream, but its iOS boundary
  has not yet been audited.
- **Portable (source-level)** — no target-specific implementation was found in the
  initial audit; this is not a runtime validation.
- **Expected platform adapter** — the upstream abstraction can be retained, but
  iOS needs an explicit provider or capability implementation.
- **External capability** — behavior depends on a helper, protocol, or environment
  that may be absent and must degrade gracefully.
- **Known blocker (source audit)** — the baseline source already exposes an
  iOS-specific compilation or semantic gap that must be confirmed by CI/device
  work.
- **Build unvalidated** — no iOS build has been performed for that path yet.
- **iOS compile path implemented** — an iOS-specific implementation exists, but
  compilation or runtime claims still require the corresponding target evidence.

Rows touched by Task 002 use `002` in the planned-task column; unrelated rows
remain `TBD`. A row may be revisited without changing its upstream behavior.

## Task 002 evidence

Task 001's real device-target workflow,
[`aarch64-apple-ios` run 36090726252](https://github.com/nghianguyen150612/yazi-ios/actions/runs/36090726252),
stopped in `uzers 0.12.2` with seven errors in `src/base.rs`: missing iOS
`UserExtras` and `GroupExtras` definitions plus the `getgrouplist` UID/GID ABI
mismatch. Yazi's direct path is `yazi-shim -> uzers`; no other Yazi crate uses
the dependency directly.

Task 002 isolates iOS from that dependency and uses the iOS-supported
`getuid`/`getgid` and reentrant `getpwuid_r`/`getgrgid_r` declarations from
`libc 0.2.189`. The existing facade, real IDs, lossless OS-byte names, ownership
checks, runtime/temp UID suffixes, and Lua identity APIs remain in place. The
shared host tests cover result mapping, unknown IDs, errors, buffer growth, and
non-UTF-8 name copying; they do not prove the libc behavior of an iOS runtime.
The target workflow result must be used for compile evidence, and a jailbroken
physical device is still required for runtime validation.

## Task 007 evidence: terminal event waiting

The Unix event source in `yazi-term/src/source/unix.rs` waits on exactly three
descriptors, in this order: the terminal input descriptor from `yazi-tty`
(stdin, or `/dev/tty` when stdin is not a tty), the nonblocking
`signal_hook` SIGWINCH pipe, and the internal waker pipe pair. It reduces them
to a three-element readiness array and preserves the existing contract: wakeup
wins over input, input wins over resize, resize reads
`termios::tcgetwinsize`, and the parser expires on every other return.

Before Task 007, `target_os = "ios"` selected the `poll(2)` path while macOS
selected `select(2)`. The upstream reason for the split was stated only as "macOS
`poll(2)` doesn't work on file descriptors to `/dev/tty`", so the iOS choice was
unjustified rather than measured. Task 007 verified the limitation instead of
assuming it, because the same `cfg(unix)`/Darwin reasoning does not otherwise
transfer automatically to iOS.

The limitation is documented by Apple for both platforms. The `poll(2)` man
page in Apple's iOS archive states under BUGS that "the `poll()` system call
currently does not support devices", and a terminal is a character device. This
is a property of the Darwin/XNU kernel rather than of macOS specifically, and
`select(2)` has no equivalent restriction in the iOS man page. Rustix confirms
the required API is available on iOS: `select`, `FdSetElement`, `FdSetIter`,
`fd_set_insert`, and `fd_set_num_elements` are gated on `bsd`, and rustix's
`build.rs` enables `bsd` for `macos`, `ios`, `tvos`, `visionos`, and `watchos`.
No new dependency was added; the workspace already enables rustix's `event`
feature.

Task 007 therefore extended the existing macOS `cfg` to cover iOS and reused the
single `select3` implementation rather than duplicating it. The `poll(2)` path,
its timeout conversion, the nonblocking semantics, and the three-descriptor
contract are unchanged for every other target, so Linux and the BSDs are
untouched.

Evidence is still incomplete in two respects. The new unit tests drive the
abstraction with connected socket pairs, which both `poll(2)` and `select(2)`
report, so they cover readiness, timeout, multi-descriptor, and drained-descriptor
handling but cannot reproduce a character device and therefore cannot demonstrate
the `poll(2)` limitation itself. Nor can they exercise the iOS `select(2)` branch
on a Linux host. Physical validation of terminal input, SIGWINCH resize, and
waker wakeup through a real `/dev/tty` on a jailbroken device, over both a local
terminal and SSH, remains required.

## Task 008 evidence: process and shell execution

Every shell task — blocking `:shell`, background, orphan, openers, hooks, and the
configured editor — funnels through one function, `shell()` in
`yazi-scheduler/src/process/shell.rs`, which runs `sh -c` with the working
directory, stdio, and detachment derived from `ShellOpt`. That single choke point
is what Task 008 audited.

**Which spawn path is actually taken.** This is worth stating precisely, because
it is not the one the API surface suggests. Rust's standard library reaches
`posix_spawn` only when no `pre_exec` closure is registered: `pre_exec` pushes a
closure, and a non-empty closure list makes the `posix_spawn` attempt return
`Ok(None)` and fall back to `fork(2)` plus `execvp(2)`. Because `shell()`
registers a `pre_exec` closure for every spawn, Yazi's shell path uses
`fork`+`exec` on *every* Unix target, iOS included — not just on iOS. A working
directory alone would not have forced this; it is the `setsid()` call that does.

**Why the fork path is kept on iOS.** Replacing `pre_exec` is not a way to reach
`posix_spawn` while preserving behavior:

- `std::os::unix::process::CommandExt::setsid` exists but is unstable
  (`process_setsid`, issue #105376), so it is unavailable on the stable toolchain
  this repository targets.
- Even if it were stable, the standard library only maps it to
  `POSIX_SPAWN_SETSID` on `linux-gnu`. On Apple targets the `posix_spawn` attempt
  bails out to `fork`+`exec`, so a session-creating `posix_spawn` does not exist
  on Darwin at all.
- `process_group(0)` *is* stable and does map to `POSIX_SPAWN_SETPGROUP`, but it
  creates a new process group, not a new session. That is a different guarantee
  from the `setsid()` the event loop and orphan handling depend on, and it cannot
  be validated without a device.

Trading a verified working path for an unverifiable semantic change is the worse
outcome, so the fork+exec path is retained. The `pre_exec` closure is also
async-signal-safe (it calls only `setsid()` and `last_os_error()`), which is what
makes it legitimate between `fork` and `exec`.

**Apple's documented API surface, and what it does and does not imply.** Apple's
iOS manual-page archive documents `posix_spawn(2)` and `vfork(2)` but has no
`fork(2)` page, and `libc` still declares `fork`, `setsid`, and `execvp` for
every Unix target including iOS. The absent page reflects Apple's *supported*
API surface for iOS rather than a removed kernel capability, and Yazi-iOS
deliberately targets the jailbroken command-line environment rather than a
stock App Store process sandbox. No runtime claim is made here: only a jailbroken
device can confirm that `fork`+`exec` and `setsid` are permitted.

**Shell resolution.** The shell is resolved as `sh` through `PATH` by
`execvp(3)`, so the environment of whoever launched Yazi decides which shell
interprets the command. It is intentionally not pinned to `/bin/sh` or to a
jailbreak-manager path, because rootful and rootless layouts differ.

**Failure behavior.** A missing shell and an unreachable working directory both
surface from `spawn()` as `NotFound`, so the two were previously
indistinguishable in the user-facing notification. `spawn_error()` now names the
actual cause — missing shell, inaccessible working directory, denied process
creation — and preserves the original error as the source for anything else.
Failures stay non-fatal: the UI reports them and stays usable, which is the
upstream contract.

The new host tests cover the exit status, working directory, non-blocking and
orphan detachment, and each diagnostic branch. They run `sh` on the host and
therefore prove nothing about process creation on a jailbroken iOS device.

## Task 009 evidence: clipboard

Upstream's clipboard is one `Clipboard` type with an in-process mirror and one
system-clipboard backend per platform, and the audit found four distinct
consumers: `mgr:copy` (file paths/URLs as OS bytes), `spot:copy` (table cell
text), the input layer's yank/cut and its paste, and the `ya.clipboard()` Lua
API. Nothing else reads or writes it. That is the whole contract, and it is a
byte buffer, not a string — `mgr:copy` can hand it a filename that is not valid
UTF-8.

**Three targets, not one.** The audit separated three things upstream treats as
one operation:

- *The in-process mirror.* Always written on `set()`, always available. It is
  the only clipboard that cannot be absent, and it is the answer whenever a
  system clipboard cannot reply. Task 009 made this explicit as
  `Clipboard::mirror`/`Clipboard::mirrored` so every backend shares one
  definition of it.
- *The terminal client's clipboard.* OSC 52, emitted on every `set()`,
  unchanged. Over SSH this is the only channel to the clipboard of the machine
  the user is actually typing into, and it is also what Blink Shell-style
  terminals turn into a real device-pasteboard write.
- *The device system pasteboard.* Absent on iOS before Task 009, which is what
  left local yanks invisible to every other app on the device.

**Why the generic Unix probing was wrong here.** `pbpaste`/`pbcopy` are macOS
binaries that an iOS rootfs does not ship, `termux-clipboard-get`/`-set` are
Android, and `wl-*`/`xclip`/`xsel` need Wayland or X11. A read therefore spawned
five processes that could not succeed, and a write never reached the pasteboard
at all. `clipboard/unix.rs` keeps that probe list verbatim for every other Unix
target but is compiled only for `all(unix, not(target_os = "ios"))`;
`clipboard/ios.rs` replaces it with the native pasteboard.

**Which iOS API, and why.** `UIPasteboard` is the only public pasteboard API on
iOS. `NSPasteboard` is AppKit, and the Core Foundation pasteboard calls behind
`pbcopy` on macOS are not in the iOS SDK, so there is no Foundation-only
alternative and no justification for a private API. It requires no
`UIApplication` lifecycle, which is what makes it reachable from a process with
no app bundle, but it does require an autorelease pool, so
`yazi-ffi/src/pasteboard.rs` pushes one around every call. It is reached through
`objc2-ui-kit` with only the `UIPasteboard` feature — typed bindings over
handwritten `msg_send!`, and iOS-only dependencies, so the existing macOS
`objc2` build is unchanged.

**What the adapter guarantees.** The class is looked up through the Objective-C
runtime before it is used, so a system that cannot provide `UIPasteboard`
reports unavailable instead of aborting. Both directions run on a blocking
thread, because the pasteboard daemon is XPC-backed and the first call in a
process can block noticeably. Every failure — no text, an image-only
pasteboard, a locked device, a declined prompt, a missing class — resolves to the
in-process mirror, with no retry, no panic, and no per-operation diagnostic,
matching how the desktop backend degrades when no helper is installed.

**Read and write, per session type.**

- *Local terminal, read:* device pasteboard, then the in-process mirror. No
  helper command is spawned.
- *SSH, read:* the in-process mirror only, exactly as upstream. There is no
  protocol by which a remote process can read its client terminal's OS
  clipboard, so the only honest answer is what Yazi itself last wrote. The
  device pasteboard is deliberately *not* consulted: it would describe the
  phone, not the PC the user is typing into, and reading it can raise the
  system paste prompt at the wrong moment. No OSC 52 query was added for
  feature parity, because a client that ignores the response would turn a paste
  into a hang.
- *Write, either session type:* mirror, then OSC 52, then the device
  pasteboard. Writing the device pasteboard over SSH as well is deliberate and
  matches macOS, where `pbcopy` also runs in an SSH session: OSC 52 reaches the
  PC's clipboard and says nothing about the clipboard the rest of the device
  shares, so both are useful and neither replaces the other.

**Encoding.** The pasteboard is reached through its string API, so raw bytes are
converted with `String::from_utf8_lossy` — the conversion the Windows backend
already used — and invalid sequences become U+FFFD instead of panicking or
dropping the yank. The in-process mirror always keeps the original bytes, so
nothing is lost for Yazi itself. `UIPasteboard` also supports URLs, images, and
colors, but Yazi's contract is text, and a richer MIME story is not implemented
rather than half-implemented.

**What the host tests do and do not prove.** The new tests cover that the mirror
stores bytes verbatim including invalid UTF-8 and that a later write replaces an
earlier one; that a text-only conversion preserves valid text and replaces
invalid sequences; that a read prefers an answering backend, falls back when the
backend is silent, and never consults a backend over SSH; and that the Unix probe
list is exactly the five desktop/Android commands. `clipboard/ios.rs` is
compiled by host tests so its selection and encoding rules are exercised, but
the UIKit calls are behind `target_os = "ios"`. **No host test exercises
`UIPasteboard`.** Whether a non-app process can load UIKit on a jailbroken
device, whether the pasteboard is reachable from a shell at all, and whether a
read raises the iOS 16+ paste prompt are open questions that only a device can
answer.

**Status: COMPILE-VALIDATED / DEVICE-UNVERIFIED.** The `aarch64-apple-ios` build
is the compile evidence, including that UIKit links. Runtime success is not
claimed; see `docs/ios-device-test-plan-clipboard.md`.

## Feature inventory

| Subsystem / Feature | Upstream behavior | Current iOS status | Expected implementation | Required external dependency | Requires real-device validation | Planned task |
| --- | --- | --- | --- | --- | --- | --- |
| Main file-manager executable | `yazi-fm` starts the TUI, initializes services, and serves the file manager | Build unvalidated; allocator and startup seams are platform-sensitive | Keep the `yazi-fm` lifecycle; add only iOS initialization adapters | Rust std; Apple SDK | Yes | TBD |
| Companion `ya` CLI | `yazi-cli` provides version/environment diagnostics, DDS emit/exec, and package commands | Portable (source-level); iOS binary not built | Retain the CLI and defer `ya-ios` packaging/name changes to the installer task | Rust std; Git for package operations | Yes | TBD |
| Startup arguments and client identity | `yazi-boot` parses entry paths, chooser files, client IDs, and runtime options | Portable (source-level) | Preserve argument semantics and validate path conversion on device | None | Yes | TBD |
| Async runtime and core state | Shared local set, actor/core state, reconciler, invalidator, and proxy layers coordinate the UI | Portable (source-level) | Keep upstream event architecture; avoid platform branching in actors | Tokio | Yes | TBD |
| Tabs and multiple working directories | Multiple tabs, per-tab CWD, peek, and cross-directory selection are supported | Portable (source-level) | Preserve tab/state model and adapt only CWD/path discovery | Rust std | Yes | TBD |
| File selection and yank/copy state | Select, toggle, yank, paste, and cross-directory selection update the manager and DDS state | Portable (source-level) | Retain file identity and URL semantics | Rust std | Yes | TBD |
| Sorting, filtering, hidden files, symlinks | Configurable natural sorting, filtering, hidden/symlink display, and transliteration are built in | Portable (source-level) | Keep portable algorithms; verify metadata and filename byte handling on iOS | Rust std; optional `tr` behavior | Yes | TBD |
| Search and find | `fd`, `rg`, `fzf`, and path completion integrate with the manager and input layer | External capability | Detect each helper at operation time; retain a usable non-helper path where upstream has one | `fd`, `rg`, `fzf` | Yes | TBD |
| Input, pickers, confirmation, which | Vim-like input, pick, confirm, which-key, completion, and modal components are built in | Portable (source-level), with TTY/SSH validation pending | Preserve UI and parser behavior; use the terminal capability adapter | Terminal input; optional helpers | Yes | TBD |
| Task scheduling and progress | Async workers, priorities, progress, cancellation, retries, hooks, and task summaries are implemented | Expected platform adapter | Retain scheduler; make process/file capabilities explicit and non-fatal | Tokio; shell for some tasks | Yes | TBD |
| Bulk copy, move, delete, link, and hardlink | File workers traverse trees, preserve metadata, and report progress | Expected platform adapter | Keep the operation model; adapt permissions, case folding, and unavailable device paths | Rust filesystem; platform permissions | Yes | TBD |
| Unix user/group identity and Lua APIs | Current UID/GID and optional UID/GID name lookup support secure paths, ownership checks, and `ya.uid()`/`gid()`/`user_name()`/`group_name()` | iOS compile path implemented through native `getuid`/`getgid` and reentrant lookup; target and device validation required | Preserve the `Uzers` facade, regular-Unix cache, real iOS IDs, and lossless names | `uzers` off iOS; `libc` on iOS | Yes | 002 |
| File metadata and attributes | `Cha` models kind, mode, ownership, timestamps, device IDs, and link counts | iOS compile path implemented for current identity lookup; metadata and device behavior still require validation | Keep real Unix UID/GID metadata and map iOS/jailbreak filesystem capabilities honestly | Rust std/libc metadata | Yes | TBD |
| Case-insensitive filename handling | Linux, Android, macOS, Windows, NetBSD, and OpenBSD have specialized case-folding paths | iOS implementation added sharing Darwin F_GETPATH / O_SYMLINK path; target compilation validated; real-device behavior pending | Add an iOS path/final-path implementation or a deliberately tested fallback | Apple filesystem APIs or libc | Yes | 004 |
| Path expansion and URL normalization | XDG, home, tilde, absolute/relative, view URLs, and path cleaning are centralized in `yazi-fs`/`yazi-shared` | Expected platform adapter | Preserve `Url`/`Path` contracts; provide iOS platform roots and permission-aware expansion | Rust std; jailbreak path layout | Yes | TBD |
| XDG, config, state, runtime, and temp directories | Unix builds use XDG variables and home-directory fallbacks; other platforms have separate paths | iOS current-UID suffix has a native lookup path; the overall iOS path policy remains unvalidated | Define rootful/rootless iOS locations and a safe runtime/temp policy without replacing the real UID | Rust std; jailbreak environment | Yes | TBD |
| Mount and device discovery | Linux monitors `/proc`; macOS uses disk arbitration; generic partition metadata is used for refresh/sound decisions | Expected platform adapter; iOS monitor is not implemented | Add a provider or explicit unavailable implementation while keeping the partition contract | iOS/device APIs or an agreed empty provider | Yes | TBD |
| Trash browsing and restore | Platform trash implementations support list, metadata, remove, restore, rename, and empty operations | iOS backend implemented in yazi-fs; target compilation validated; host functional tests passed; real-device runtime validation pending | Implement the iOS trash contract using ~/.local/share/Trash and Freedesktop .trashinfo format with collision resolution and copy fallback | Platform trash policy; no mandatory helper | Yes | 003 |
| Local file watcher | `notify::RecommendedWatcher` is primary, with `PollWatcher` fallback and mount refresh callbacks | Expected platform adapter | Validate FSEvents/dispatch availability; retain polling and report capability | `notify`; optional platform notifications | Yes | TBD |
| Virtual watcher | Remote/VFS URLs are watched through a separate virtual backend | Portable (source-level); remote behavior unvalidated | Preserve virtual watch model and capability errors | VFS provider | Yes | TBD |
| Blocking shell commands | Scheduler runs `sh -c` with inherited stdio and pauses/resumes the app | iOS compiles the same `fork`+`exec` Unix path; spawn failures now name the actual cause; jailbreak runtime still unverified | Keep the Unix path and make spawn failure diagnostics actionable per cause | `sh`, libc process APIs | Yes | 008 |
| Background and orphan commands | Background commands stream stdout/stderr; orphan commands detach with `setsid` | iOS retains `setsid` via `pre_exec`; the alternative `posix_spawn` route cannot express a new session on Apple; jailbreak runtime unverified | Keep the detach contract and validate it on a device | `sh`, libc, jailbreak process policy | Yes | 008 |
| Openers and file reveal | MIME rules invoke configured commands such as `xdg-open`, `open`, `start`, or Termux tools | Expected platform adapter | Add an iOS opener/share provider and bulk-operation semantics | Platform opener or user-installed command | Yes | TBD |
| Clipboard get/set | Unix tries pbcopy/Termux/Wayland/X11 tools; Windows uses a native API; OSC 52 is also emitted | iOS backend added: native `UIPasteboard` via `yazi-ffi`, no helper probing, mirror and OSC 52 preserved; target compilation validated, device runtime unverified | Reach the device pasteboard natively over a local terminal; keep the in-process mirror over SSH and always emit OSC 52 | UIKit `UIPasteboard`; OSC 52 support for the PC clipboard | Yes | 009 |
| TTY handles and raw mode | Unix opens stdin/stdout or `/dev/tty`, uses termios, and restores terminal state | Expected platform adapter | Validate local device and SSH descriptors; keep fallback and restoration guarantees | `/dev/tty`, termios/rustix | Yes | TBD |
| Terminal input parser | CSI u, Kitty keyboard, bracketed paste, mouse, resize, DND, and terminal reports are parsed | Expected platform adapter | Reuse parser and make unsupported reports non-fatal over SSH and local terminals | Terminal protocol support | Yes | TBD |
| Terminal emulator probes | Yazi detects brand, version, cell size, color, cursor, Sixel, Kitty graphics, clipboard, and mux passthrough | Portable (source-level), with runtime validation pending | Extend capability-based probing; never assume a local iOS terminal protocol | Terminal responses; optional `tmux` | Yes | TBD |
| Kitty graphics protocols | KGP and legacy KGP encode images directly and optionally use shared memory | External capability; shared-memory path is unvalidated on iOS | Use only after probe; fall back without breaking text mode | Kitty-compatible terminal; POSIX shared memory | Yes | TBD |
| iTerm2 and WezTerm inline images | IIP emits inline image escape sequences | External capability | Keep protocol driver and validate over SSH/local terminal | iTerm2/WezTerm-compatible terminal | Yes | TBD |
| Sixel graphics | Sixel encoding and terminal capability detection are built in | External capability | Keep capability probe and text fallback | Sixel-capable terminal | Yes | TBD |
| Überzug++/X11/Wayland graphics | An external long-lived `ueberzugpp` process draws images for X11/Wayland | External capability | Detect runtime support; do not make it a launch requirement | `ueberzugpp`, X11/Wayland compositor | Yes | TBD |
| Chafa fallback | `chafa` renders an image to terminal text | External capability | Keep as an optional fallback; if absent, continue with a non-graphical preview | `chafa` | Yes | TBD |
| Text and code preview | Scrollable text preview, syntax highlighting, wrapping, and tab sizing are built in | Portable (source-level) | Preserve preview pipeline and metadata detection | Rust highlighter; optional helpers for some types | Yes | TBD |
| Image decoding and scaling | `image`, color conversion, and preview bounds support many raster formats | Portable (source-level), with memory policy unvalidated | Preserve decoding; revisit allocator and memory limits on device | Rust image codecs | Yes | TBD |
| Directory preview | Directory contents, metadata, and recursive previews are rendered through plugins | Portable (source-level) | Retain plugin contract and filesystem adapter | Rust std; watcher/VFS | Yes | TBD |
| PDF preview | PDF previewers invoke configurable external tooling | External capability | Detect PDF tools and provide a clear non-graphical fallback | `pdftoppm` or equivalent | Yes | TBD |
| Video preview | Video preview/metadata uses configurable media helpers and terminal image output | External capability | Keep feature; detect media tools and graphics path independently | `ffmpeg`/`ffprobe` or equivalent; terminal protocol | Yes | TBD |
| Archive preview and extraction | Archive metadata, listing, and extraction are plugin/tool integrations | External capability | Preserve archive UX; fail per operation when no extractor exists | `7z`/`7zz`, `bsdtar`, or equivalent | Yes | TBD |
| JSON preview | JSON formatting/inspection is provided by a Lua/plugin path and may use `jq` | External capability | Keep JSON preview without requiring `jq` | Lua; optional `jq` | Yes | TBD |
| SVG preview | SVG rendering/preview is plugin/tool based | External capability | Detect renderer and fall back to text/metadata | `resvg` or equivalent | Yes | TBD |
| MIME detection and file typing | MIME rules select previewers/openers; `file` is commonly used for detection | External capability | Treat MIME detection as a replaceable capability and preserve configured rules | `file` or platform MIME API | Yes | TBD |
| Fetcher, preloader, and spotter plugins | Lua extensions can fetch, preload, and spot file data asynchronously | Portable (source-level); native helper paths need validation | Preserve runner and scheduler APIs; isolate process/filesystem calls | Lua runtime; optional helpers | Yes | TBD |
| Lua runtime | Vendored Lua is enabled by default and powers UI, configuration, preview, and plugins | Expected platform adapter; native Lua build is unvalidated | Keep full Lua feature set and fix target/toolchain issues rather than removing it | `mlua` with vendored Lua; C compiler/SDK | Yes | TBD |
| Lua filesystem and process APIs | Plugins can read/write files, spawn processes, use URLs, and access runtime state | Expected platform adapter | Preserve API surface and return capability-aware errors | Lua; shell/process provider | Yes | TBD |
| UI plugin components | Upstream preset components compose the app, tabs, rails, preview, status, tasks, and modals | Portable (source-level) | Keep component presets and full custom UI support | Lua; terminal renderer | Yes | TBD |
| User `init.lua` and custom plugins | User initialization and installed plugins extend the UI and behavior | Portable (source-level); package/runtime paths unvalidated | Preserve config discovery and plugin loading on iOS paths | Lua; optional Git/package manager | Yes | TBD |
| VFS abstraction | Local, SFTP, Lua, and custom providers share file operations and URL semantics | Expected platform adapter | Keep `yazi-vfs` engine/provider contract and adapt path/auth capabilities | Provider-specific runtime | Yes | TBD |
| SFTP client | `yazi-sftp` implements SFTP sessions, files, directories, links, metadata, and transfers | Portable (source-level), with real SSH validation pending | Preserve client and make authentication/network errors actionable | Network; OpenSSH key/password/agent configuration | Yes | TBD |
| SSH authentication agent | Unix SFTP authentication can use a Unix-domain agent socket | Expected platform adapter | Validate SSH agent availability and provide key/password fallbacks | SSH agent; OpenSSH-compatible keys | Yes | TBD |
| HTTP VFS writes | Lua/binding HTTP helpers can write remote content through the VFS | External capability | Preserve optional HTTP capability with bounded errors | Network/HTTP client | Yes | TBD |
| DDS client/server | `yazi-dds` provides Unix-socket client/server state, pubsub, and cross-instance events | Expected platform adapter | Validate socket/runtime paths or provide an explicit unavailable transport | Unix-domain socket or later transport | Yes | TBD |
| Cross-instance state and pubsub | Commands, hover, yank, mount, trash, and peer events synchronize Yazi instances | Expected platform adapter | Retain protocol and state semantics independent of transport | DDS transport | Yes | TBD |
| Package manager | `ya pkg` adds, deletes, installs, lists, upgrades, and pins plugin/theme packages using Git/filesystem operations | External capability | Keep package management available when a Git/package environment exists; never gate launch | `git`, filesystem, archive/hash helpers | Yes | TBD |
| Package deployment and pinning | Package assets are copied/deployed and revisions can be pinned | External capability | Preserve semantics and report unavailable deployment targets | Git; filesystem permissions | Yes | TBD |
| Configuration parsing and merging | TOML defaults and user config are merged with validation and error recovery | Portable (source-level) | Keep config behavior and platform path handling | Rust TOML | Yes | TBD |
| Themes, icons, and custom styles | Dark/light themes, file-type styles, icons, and custom fields shape the UI | Portable (source-level) | Preserve complete theme system; only adapt asset/path loading | Rust std; Lua UI | Yes | TBD |
| Layouts and panes | Configurable ratios, headers, preview, status, tabs, and task views are built in | Portable (source-level) | Preserve layout model and terminal dimension behavior | Terminal size | Yes | TBD |
| Keymaps and chords | Section-specific keymaps support chords, modifiers, and custom bindings | Portable (source-level), with keyboard protocol unvalidated | Preserve parser and capability detection | Terminal input | Yes | TBD |
| CSI u keyboard protocol | Yazi queries and uses enhanced keyboard flags where supported | External capability | Probe and use when available; retain legacy input fallback | Terminal CSI u support | Yes | TBD |
| Mouse support | Mouse modes and events are parsed and routed to manager/input actions | External capability | Preserve parser; validate local and SSH terminal reporting | Terminal mouse protocol | Yes | TBD |
| Drag and drop | OSC 72 DND sequences exchange paths and data with terminals/plugins | External capability | Preserve protocol; degrade if terminal does not support it | Terminal OSC 72 support | Yes | TBD |
| Terminal clipboard OSC 52 | Clipboard writes and capability probes use OSC 52 | External capability; still emitted on every `set()` on iOS, including alongside the new device pasteboard write | Use when supported and retain in-process/SSH fallback | Terminal OSC 52 support | Yes | 009 |
| Multiplexer support | tmux and Zellij passthrough/sixel behavior is detected and adjusted | External capability | Preserve mux abstraction and capability probing | `tmux`/Zellij and terminal passthrough | Yes | TBD |
| Signals and resize | Unix signal hooks, SIGWINCH, and terminal restorers coordinate lifecycle events | iOS shares the Apple `select(2)` wait path for terminal, SIGWINCH pipe, and waker descriptors; polling abstraction unit-tested on the host; real-device behavior pending | Keep the shared Darwin wait path and validate SIGWINCH delivery on a physical device | Unix signals; terminal | Yes | 007 |
| Shared memory for graphics | Kitty image transport can use POSIX shared memory with a base64 fallback | Expected platform adapter; iOS availability unvalidated | Keep base64 fallback and gate shared memory by capability | POSIX shared memory; terminal | Yes | TBD |
| Foreign-function interfaces | `yazi-ffi` wraps libc/rustix shared memory and macOS Core Foundation/IOKit/Objective-C facilities | iOS has a first adapter: `yazi-ffi/src/pasteboard.rs` binds UIKit `UIPasteboard` with iOS-only `objc2-ui-kit`/`objc2-foundation`; UIKit link validated, device runtime unverified | Keep framework bindings in `yazi-ffi` behind typed crates, probe availability, and audit each remaining FFI dependency against the iOS SDK | Apple SDK; jailbreak APIs | Yes | 009 |
| Allocator and memory behavior | jemalloc is selected for non-macOS/non-Windows targets, which includes iOS in the baseline | iOS uses native system allocator; jemalloc excluded on iOS target; target compilation validated | Select a supported allocator or prove the existing one on iOS | Native allocator/build toolchain | Yes | 005 |
| Rust target and Apple linking | Task 001 added an unsuppressed macOS `aarch64-apple-ios` baseline workflow | Task 001 CI observed the `uzers` blocker; Task 002 adds the iOS identity source path and explicit branch dispatch | Confirm the old errors are gone and record the next independent target blocker without suppressing failures | Xcode/iOS SDK; Rust target | Yes | 002 |
| Existing desktop CI | Upstream tests and checks run on Linux, macOS, and Windows | Portable (source-level); must remain unchanged | Add an isolated iOS workflow and retain all existing jobs | GitHub Actions runners | No | TBD |
| Linux-specific filesystem/mount behavior | `/proc`, inotify-style watcher selection, device metadata, and Linux libc calls are selected behind Linux cfgs | Portable (source-level) for desktop; not an iOS path | Do not port Linux assumptions to iOS; use adapters or explicit unsupported results | Linux kernel interfaces | Yes | TBD |
| macOS-specific trash/mount/FFI | macOS has bespoke trash, disk arbitration, Core Foundation, and Objective-C code | Trash (003) and clipboard (009) have their own iOS implementations; disk arbitration and Core Foundation modules remain macOS-only | Reuse only APIs proven available on iOS; otherwise provide a separate adapter | Apple frameworks | Yes | TBD |
| Android-specific paths | Android has Termux opener/clipboard and unsupported trash cases | Portable (source-level) as a reference pattern | Use as a precedent for capability detection, not as an iOS implementation | Termux tools on Android | Yes | TBD |
| iOS device system pasteboard | No upstream equivalent; the device pasteboard is what other iOS apps share | iOS-only adapter added in `yazi-ffi`; target compilation validated, device runtime and paste-prompt behavior unverified | Reach `UIPasteboard` natively, treat it as auxiliary, and keep the in-process mirror as the fallback | UIKit; jailbreak pasteboard access | Yes | 009 |
| Rootful jailbreak environment | Upstream has no jailbreak-specific path or permission model | Uninvestigated | Discover and validate rootful paths, permissions, process policy, and service integration | Jailbreak-specific APIs/tools | Yes | TBD |
| Rootless jailbreak environment | Upstream has no rootless jailbreak-specific path or permission model | Uninvestigated | Discover and validate rootless paths, permissions, process policy, and service integration | Jailbreak-specific APIs/tools | Yes | TBD |
| Real-device SSH session | SSH is an intended deployment mode but has not been validated on iOS | Expected platform adapter | Validate TTY, resize, input, process, clipboard, preview, and remote filesystem behavior together | SSH client/server and terminal | Yes | TBD |
| Local-device terminal session | A local terminal may differ materially from SSH and desktop terminals | Uninvestigated | Run the same capability and fallback matrix on a physical jailbroken device | Terminal emulator/SSH client | Yes | TBD |
| iOS Simulator validation | A simulator can provide fast compile feedback but lacks jailbreak and full device runtime conditions | Build unvalidated | Add only as a separate compile/test aid after device blockers are understood | Xcode simulator runtime | Yes | TBD |
| Future binary names | Upstream currently emits `yazi` and `ya`; the product goal names `yazi-ios` and `ya-ios` | Not applicable to Task 001 source behavior | Add names and compatibility aliases in the later installer/packaging task | Installer/package layout | Yes | TBD |
| Optional dependency absence | `ya env` reports versions of external helpers and clipboard tools | External capability | Make every probe diagnostic-only; missing helpers must not prevent launch | Optional helper inventory | Yes | TBD |
| Installer and release packaging | Upstream build/packing workflows target desktop archives and Debian packages | Out of scope for Task 001 | Build a jailbreak-aware installer and rootful/rootless packaging in a later task | Package manager/installer | Yes | TBD |

## Updating the table

For each later task:

1. Change only the rows whose implementation or validation scope is covered.
2. Keep `External capability` distinct from `Expected platform adapter`; a helper
   being installed does not prove that a platform bridge exists.
3. Record the exact iOS device/runtime, jailbreak mode, terminal, and helper versions
   used for any validation claim.
4. Do not mark a row supported solely because a Simulator or host build succeeds.
5. Add newly discovered upstream features rather than silently dropping them from
   the inventory.
