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

The planned-task column intentionally uses `TBD` after Task 001. The detailed
Tasks 002–020 sequence is not specified by this baseline; this table is the
inventory against which that sequence should be planned. A row may be revisited
without changing its upstream behavior.

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
| File metadata and attributes | `Cha` models kind, mode, ownership, timestamps, device IDs, and link counts | Expected platform adapter | Keep the metadata model and map iOS/jailbreak filesystem capabilities honestly | Rust std/libc metadata | Yes | TBD |
| Case-insensitive filename handling | Linux, Android, macOS, Windows, NetBSD, and OpenBSD have specialized case-folding paths | Known blocker (source audit): no iOS `casefold_impl` branch is visible | Add an iOS path/final-path implementation or a deliberately tested fallback | Apple filesystem APIs or libc | Yes | TBD |
| Path expansion and URL normalization | XDG, home, tilde, absolute/relative, view URLs, and path cleaning are centralized in `yazi-fs`/`yazi-shared` | Expected platform adapter | Preserve `Url`/`Path` contracts; provide iOS platform roots and permission-aware expansion | Rust std; jailbreak path layout | Yes | TBD |
| XDG, config, state, runtime, and temp directories | Unix builds use XDG variables and home-directory fallbacks; other platforms have separate paths | Expected platform adapter | Define rootful/rootless iOS locations and a safe runtime/temp policy | Rust std; jailbreak environment | Yes | TBD |
| Mount and device discovery | Linux monitors `/proc`; macOS uses disk arbitration; generic partition metadata is used for refresh/sound decisions | Expected platform adapter; iOS monitor is not implemented | Add a provider or explicit unavailable implementation while keeping the partition contract | iOS/device APIs or an agreed empty provider | Yes | TBD |
| Trash browsing and restore | Platform trash implementations support list, metadata, remove, restore, rename, and empty operations | Known blocker (source audit): iOS is marked unsupported, while local delete cfg needs reconciliation | Implement the iOS trash contract or return explicit unsupported results without blocking launch | Platform trash policy; no mandatory helper | Yes | TBD |
| Local file watcher | `notify::RecommendedWatcher` is primary, with `PollWatcher` fallback and mount refresh callbacks | Expected platform adapter | Validate FSEvents/dispatch availability; retain polling and report capability | `notify`; optional platform notifications | Yes | TBD |
| Virtual watcher | Remote/VFS URLs are watched through a separate virtual backend | Portable (source-level); remote behavior unvalidated | Preserve virtual watch model and capability errors | VFS provider | Yes | TBD |
| Blocking shell commands | Scheduler runs `sh -c` with inherited stdio and pauses/resumes the app | Expected platform adapter | Use a validated shell/process provider; retain task and error semantics | `sh`, libc process APIs | Yes | TBD |
| Background and orphan commands | Background commands stream stdout/stderr; orphan commands detach with `setsid` | Known blocker (source audit): iOS process policy and `setsid` behavior are unverified | Make local/SSH process capability explicit and gracefully report denial | `sh`, libc, jailbreak process policy | Yes | TBD |
| Openers and file reveal | MIME rules invoke configured commands such as `xdg-open`, `open`, `start`, or Termux tools | Expected platform adapter | Add an iOS opener/share provider and bulk-operation semantics | Platform opener or user-installed command | Yes | TBD |
| Clipboard get/set | Unix tries pbcopy/Termux/Wayland/X11 tools; Windows uses a native API; OSC 52 is also emitted | Expected platform adapter | Add a jailbroken-device clipboard bridge while preserving SSH/terminal behavior | Platform pasteboard; OSC 52 support | Yes | TBD |
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
| Terminal clipboard OSC 52 | Clipboard writes and capability probes use OSC 52 | External capability | Use when supported and retain in-process/SSH fallback | Terminal OSC 52 support | Yes | TBD |
| Multiplexer support | tmux and Zellij passthrough/sixel behavior is detected and adjusted | External capability | Preserve mux abstraction and capability probing | `tmux`/Zellij and terminal passthrough | Yes | TBD |
| Signals and resize | Unix signal hooks, SIGWINCH, and terminal restorers coordinate lifecycle events | Expected platform adapter | Validate iOS signal/descriptor behavior without changing desktop paths | Unix signals; terminal | Yes | TBD |
| Shared memory for graphics | Kitty image transport can use POSIX shared memory with a base64 fallback | Expected platform adapter; iOS availability unvalidated | Keep base64 fallback and gate shared memory by capability | POSIX shared memory; terminal | Yes | TBD |
| Foreign-function interfaces | `yazi-ffi` wraps libc/rustix shared memory and macOS Core Foundation/IOKit/Objective-C facilities | Known blocker (source audit): iOS-specific framework availability is not established | Audit each FFI dependency against iOS SDK and entitlements | Apple SDK; jailbreak APIs | Yes | TBD |
| Allocator and memory behavior | jemalloc is selected for non-macOS/non-Windows targets, which includes iOS in the baseline | Known blocker (source audit): target suitability and build behavior are unverified | Select a supported allocator or prove the existing one on iOS | Native allocator/build toolchain | Yes | TBD |
| Rust target and Apple linking | The workspace has no iOS-specific target configuration or CI before Task 001 | Build unvalidated | Add the macOS `aarch64-apple-ios` baseline path without suppressing failures | Xcode/iOS SDK; Rust target | Yes | TBD |
| Existing desktop CI | Upstream tests and checks run on Linux, macOS, and Windows | Portable (source-level); must remain unchanged | Add an isolated iOS workflow and retain all existing jobs | GitHub Actions runners | No | TBD |
| Linux-specific filesystem/mount behavior | `/proc`, inotify-style watcher selection, device metadata, and Linux libc calls are selected behind Linux cfgs | Portable (source-level) for desktop; not an iOS path | Do not port Linux assumptions to iOS; use adapters or explicit unsupported results | Linux kernel interfaces | Yes | TBD |
| macOS-specific trash/mount/FFI | macOS has bespoke trash, disk arbitration, Core Foundation, and Objective-C code | Uninvestigated for iOS | Reuse only APIs proven available on iOS; otherwise provide a separate adapter | Apple frameworks | Yes | TBD |
| Android-specific paths | Android has Termux opener/clipboard and unsupported trash cases | Portable (source-level) as a reference pattern | Use as a precedent for capability detection, not as an iOS implementation | Termux tools on Android | Yes | TBD |
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
