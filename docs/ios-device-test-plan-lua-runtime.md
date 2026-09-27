# Yazi-iOS device test plan: Lua runtime (Task 016)

Every physical-device row starts **UNVERIFIED**. Host mlua tests are
**HOST-RUNTIME-VALIDATED**; the Apple-target build is **COMPILE-VALIDATED**.
Neither is device evidence. A simulator pass, if ever added, would be
**SIMULATOR-VALIDATED**, never **DEVICE-VALIDATED**.

Environment to record per run: device model, iOS version, jailbreak
type/version, rootful vs rootless, user (`mobile` vs `root`), terminal app
(local) or SSH client/server, network availability, and helper versions
(`resvg`, `ffmpeg`, etc. where relevant). Run each applicable row in both a
local jailbreak terminal and an SSH session unless the row says otherwise.

## BOOT / VM

| # | Case | Steps / oracle | Status |
| --- | --- | --- | --- |
| 1 | UNVERIFIED — yazi-ios reaches Lua setup | Launch debug and release device binaries; startup log shows `yazi_plugin::setup()` completed with no preset-load panic | UNVERIFIED |
| 2 | UNVERIFIED — Lua 5.5 executes simple expression | Plugin evaluating `return 1+2` yields `3`; `_VERSION` reports `Lua 5.5` | UNVERIFIED |
| 3 | UNVERIFIED — standard runtime initializes | Globals `ui, ya, fs, vf, ps, rt, km, th`, `Error/Cha/Command/File/Url/Path`, and `require` all non-nil | UNVERIFIED |
| 4 | UNVERIFIED — setup.lua executes | `require("dds"):setup()`, `require("extract"):setup()`, `require("trash"):setup()` complete; `os.setlocale("")` outcome noted (nil is nonfatal) | UNVERIFIED |
| 5 | UNVERIFIED — compat.lua executes | No error from `compat.lua`; deprecated shims behave as on desktop | UNVERIFIED |
| 6 | UNVERIFIED — built-in ya.lua executes | `ya.clamp`, `ya.readable_size`, and other `ya.lua` helpers callable | UNVERIFIED |

## PRESETS

| # | Case | Steps / oracle | Status |
| --- | --- | --- | --- |
| 7 | UNVERIFIED — release binary finds embedded config presets | Release device binary with no source tree: `yazi/keymap/vfs` presets parse | UNVERIFIED |
| 8 | UNVERIFIED — release binary finds embedded plugin presets | Release device binary: built-in plugins (`dds`, `extract`, `trash`, `empty`, `folder`) require successfully | UNVERIFIED |
| 9 | UNVERIFIED — debug device binary finds required presets without source tree | Debug device binary copied without the Mac source tree: config + plugin + theme presets all load (Task 016 iOS embedding) | UNVERIFIED |
| 10 | UNVERIFIED — theme preset available | Light and dark theme presets resolve; UI renders with themed colors | UNVERIFIED |

## USER PLUGINS

| # | Case | Steps / oracle | Status |
| --- | --- | --- | --- |
| 11 | UNVERIFIED — init.lua | `init.lua` in the iOS config dir executes; an intentional marker (e.g. `ya.dbg`) appears | UNVERIFIED |
| 12 | UNVERIFIED — simple user plugin | `plugins/hello.yazi/main.lua` with `entry()` runs via `:plugin hello` | UNVERIFIED |
| 13 | UNVERIFIED — plugin under path containing spaces | Config dir path containing a space; plugin loads and runs | UNVERIFIED |
| 14 | UNVERIFIED — Unicode plugin/config path | Config/plugin path with non-ASCII (e.g. `é`, CJK); plugin loads and runs; non-UTF8 filename carried without panic where applicable | UNVERIFIED |
| 15 | UNVERIFIED — nested plugin entry | `plugins/myplug.yazi/sub.lua` required as `myplug.sub` runs | UNVERIFIED |
| 16 | UNVERIFIED — relative require | Plugin requiring `.sibling` / `.main` resolves within its own module | UNVERIFIED |
| 17 | UNVERIFIED — missing plugin | Requiring a nonexistent plugin errors visibly; Yazi stays alive | UNVERIFIED |
| 18 | UNVERIFIED — malformed Lua | Plugin with a syntax error reports the error; Yazi stays alive | UNVERIFIED |
| 19 | UNVERIFIED — runtime Lua error | Plugin calling `error("boom")` reports `boom` with plugin/entry context; Yazi stays alive | UNVERIFIED |

## RUNNER

| # | Case | Steps / oracle | Status |
| --- | --- | --- | --- |
| 20 | UNVERIFIED — entry() | Custom `entry()` job runs to completion | UNVERIFIED |
| 21 | UNVERIFIED — previewer | Custom Lua previewer renders file content | UNVERIFIED |
| 22 | UNVERIFIED — preloader | Custom preloader caches and reports success | UNVERIFIED |
| 23 | UNVERIFIED — fetcher | Custom fetcher returns data for a file | UNVERIFIED |
| 24 | UNVERIFIED — provider | Custom spot/provider path responds | UNVERIFIED |
| 25 | UNVERIFIED — evaluator | `ya.async_blocking`-style evaluation returns a value | UNVERIFIED |
| 26 | UNVERIFIED — async Lua | `ya.async` block completes and delivers its result | UNVERIFIED |
| 27 | UNVERIFIED — coroutine yield/resume | Lua coroutine yields and resumes across ticks | UNVERIFIED |
| 28 | UNVERIFIED — cancellation | Cancelling a plugin job terminates it without panic; UI remains usable | UNVERIFIED |
| 29 | UNVERIFIED — nested runtime scope cleanup after error | Nested `require` that errors leaves no stale frame (subsequent requires resolve correctly) | UNVERIFIED |

## BINDINGS

| # | Case | Steps / oracle | Status |
| --- | --- | --- | --- |
| 30 | UNVERIFIED — ya.target_os() == "ios" | `ya.target_os()` returns `"ios"` | UNVERIFIED |
| 31 | UNVERIFIED — ya.target_family() == "unix" | `ya.target_family()` returns `"unix"` | UNVERIFIED |
| 32 | UNVERIFIED — UID/GID/name APIs | `ya.uid/gid/user_name/group_name/host_name` return plausible device values; unknown IDs map to nil, not panic | UNVERIFIED |
| 33 | UNVERIFIED — Command successful spawn | `Command("sh"):arg({"-c","echo hi"})` spawns and captures output | UNVERIFIED |
| 34 | UNVERIFIED — Command missing executable | Missing binary surfaces a `NotFound`-style error pair `(nil, err)`; no panic | UNVERIFIED |
| 35 | UNVERIFIED — stdout/stderr capture | Both streams captured; nonzero exit code reported | UNVERIFIED |
| 36 | UNVERIFIED — Command memory limit behavior | `:memory()` path uses `setrlimit(RLIMIT_AS)`; `svg.lua`-style poll loop behaves (limit enforced or gracefully unmeasured — record which) | UNVERIFIED |
| 37 | UNVERIFIED — proc_info result/absence behavior | `ya.proc_info(pid).mem_resident` returns a number for a live child with a complete `proc_taskinfo`, or nil (empty table) nonfatally on failure/partial result; record jailbreak permission behavior | UNVERIFIED |
| 38 | UNVERIFIED — HTTP basic request if networking available | `ya.http.request` against a reachable URL returns a response; offline failure is a `(nil, err)` pair, not a crash | UNVERIFIED |
| 39 | UNVERIFIED — UDS binding | DDS/UDS socket path binds under the Task 014 runtime dir; cross-instance message passes | UNVERIFIED |

## MODES

| # | Case | Steps / oracle | Status |
| --- | --- | --- | --- |
| 40 | UNVERIFIED — local jailbreak terminal | Rows 1–39 (where applicable) pass in a local terminal app | UNVERIFIED |
| 41 | UNVERIFIED — SSH session | Rows 1–39 (where applicable) pass over SSH; SHM-graphics rules do not affect Lua | UNVERIFIED |
| 42 | UNVERIFIED — rootful jailbreak | Boot + plugin + process rows pass rootful; record paths used | UNVERIFIED |
| 43 | UNVERIFIED — rootless jailbreak | Boot + plugin + process rows pass rootless; record paths used | UNVERIFIED |
| 44 | UNVERIFIED — root user | Full matrix as `root`; UID/GID APIs reflect `0` | UNVERIFIED |
| 45 | UNVERIFIED — mobile/non-root user | Full matrix as `mobile`; UID/GID APIs reflect the non-root user; no permission-gated Lua init failure | UNVERIFIED |

## ROBUSTNESS

| # | Case | Steps / oracle | Status |
| --- | --- | --- | --- |
| 46 | UNVERIFIED — repeated plugin execution | Run a plugin 50×; no leak-induced slowdown or frame error | UNVERIFIED |
| 47 | UNVERIFIED — repeated VM spawn/drop | Spawn/drop 50 worker VMs; process RSS stays bounded (record MB) | UNVERIFIED |
| 48 | UNVERIFIED — plugin error does not crash Yazi | Each error row (17–19) followed by normal browsing; no abort | UNVERIFIED |
| 49 | UNVERIFIED — missing optional CLI does not break Lua startup | With `resvg/ffmpeg/fd/rg/fzf` absent from `PATH`, Yazi boots and Lua initializes; only the feature using the helper degrades | UNVERIFIED |
| 50 | UNVERIFIED — memory use under repeated plugin execution | `svg.lua`-style memory polling under repeated image preloads; no runaway growth; record `mem_resident` samples | UNVERIFIED |
