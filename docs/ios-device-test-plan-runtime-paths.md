# iOS runtime paths device test plan

Every row is **UNVERIFIED** until run on a jailbroken iOS device. A successful
target build only proves that the path policy, `getpwuid_r` home lookup, and
`sockaddr_un` calculations link; it does not prove the resolved locations are
writable, UID-private, or shared across local and SSH sessions. Record device
model, iOS version, jailbreak name and version, rootful/rootless layout, launch
mode (local terminal vs SSH), UID (`id -u`), and Yazi commit for each run.

Task 014 hardens `yazi-fs/src/xdg.rs` and `yazi-dds/src/stream.rs` without
hard-coding any jailbreak manager path:

- Config: `YAZI_CONFIG_HOME` → `XDG_CONFIG_HOME/yazi` → `HOME/.config/yazi`,
  with iOS `HOME` falling back to the native `getpwuid_r` home directory.
- Cache/assets: `XDG_CACHE_HOME/yazi` → `HOME/.cache/yazi`, same home fallback.
- State: `XDG_STATE_HOME/yazi` → `HOME/.local/state/yazi`, same home fallback.
- Runtime: `XDG_RUNTIME_DIR` if absolute, else fixed `/tmp` on iOS
  (not `TMPDIR`), plus `yazi+UID`.
- Temp: `env::temp_dir()` if absolute else `/tmp`, plus `yazi-UID`.
- DDS socket: `<runtime>/.dds.sock` when its byte length fits the Darwin
  `sun_path` limit (104 bytes of `sun_path` storage including the terminating
  NUL, 103 pathname bytes excluding the terminating NUL), otherwise a
  deterministic `/tmp/yazi-dds-UID-<hash>/.dds.sock` fallback that hashes the
  original runtime's raw bytes plus UID.
- Parent directories are created with mode 0700, opened with
  `O_DIRECTORY | O_NOFOLLOW`, checked for current-UID ownership, and forced to
  0700 via `fchmod`. Relative XDG values remain rejected.

No row below is pre-marked passed. Core logic is layout-agnostic: rows may
*inspect* `/var/mobile`, `/var/root`, `/var/jb`, or preboot paths as ground
truth, but the implementation must not depend on them.

## Local terminal: home and overrides

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | 1. Normal HOME environment | Launch from a local jailbreak terminal with normal `HOME`; config, cache, and state resolve under that home with no startup error. |
| UNVERIFIED | 2. HOME unset | Launch with `env -u HOME yazi`; on iOS the native passwd home is used and Yazi starts without the old `.expect` panic. |
| UNVERIFIED | 3. HOME empty | Launch with `HOME= yazi`; empty is treated as absent, passwd fallback is used, no panic. |
| UNVERIFIED | 4. XDG_CONFIG_HOME override | Set absolute `XDG_CONFIG_HOME`; `yazi.toml`, `keymap.toml`, and `theme.toml` load from `$XDG_CONFIG_HOME/yazi`. |
| UNVERIFIED | 5. XDG_CACHE_HOME override | Set absolute `XDG_CACHE_HOME`; packages and assets resolve under `$XDG_CACHE_HOME/yazi`. |
| UNVERIFIED | 6. XDG_STATE_HOME override | Set absolute `XDG_STATE_HOME`; `yazi.log` and `.dds` state resolve under `$XDG_STATE_HOME/yazi`. |
| UNVERIFIED | 7. XDG_RUNTIME_DIR override | Set absolute `XDG_RUNTIME_DIR` that fits; DDS socket is `$XDG_RUNTIME_DIR/yazi+UID/.dds.sock`. |
| UNVERIFIED | 8. Invalid relative XDG values | Set relative `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME`, `XDG_RUNTIME_DIR`; each is rejected and the HOME/passwd or `/tmp` fallback is used, never a cwd-relative path. |
| UNVERIFIED | 9. Config loads | With each home/override combination above, Yazi loads user config without a startup panic or silent fallback to an unrelated directory. |
| UNVERIFIED | 10. Plugin/theme paths load | `init.lua`, `plugins/`, `flavors/`, and package `package.toml` resolve under the selected config/asset roots. |

## DDS

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | 11. First instance binds socket | First Yazi binds its DDS socket with no `ENAMETOOLONG` or permission error; parent directory is mode 0700 and owned by the current UID. |
| UNVERIFIED | 12. Second process connects | A second `yazi`/`ya` process for the same UID connects to the same socket and receives pubsub/state. |
| UNVERIFIED | 13. Stale socket restart | Kill Yazi without cleanup, relaunch; the stale `.dds.sock` is removed and the new server binds successfully. |
| UNVERIFIED | 14. Local-terminal to local-terminal DDS | Two instances both launched from a local terminal share hover/yank/mount events. |
| UNVERIFIED | 15. SSH to SSH DDS | Two instances both launched over SSH share events. |
| UNVERIFIED | 16. Local-terminal to SSH DDS for same UID | One instance on a local terminal and one over SSH as the same UID derive the same socket path and interoperate; differing `TMPDIR` alone must not fragment them. |
| UNVERIFIED | 17. Very long runtime path | Set an absolute `XDG_RUNTIME_DIR` long enough that `<runtime>/.dds.sock` exceeds 103 bytes; Yazi falls back to the deterministic short socket and still binds/connects. |
| UNVERIFIED | 18. Socket path within Darwin limit | With normal and long runtimes, the bound socket pathname is at most 103 bytes; `python3 -c` byte-length checks confirm, not character counts. |
| UNVERIFIED | 19. Permissions are 0700 on parent | Both normal and fallback socket parents are mode 0700, owned by the current UID, created with `O_NOFOLLOW`; the shared `/tmp` parent itself is not rechmodded. |
| UNVERIFIED | 20. Socket inaccessible to different UID where testable | Where two UIDs can be tested, each UID's socket namespace is separate and one UID cannot hijack the other's parent directory. |

## Users

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | 21. Root execution | As uid 0, runtime is `.../yazi+0`, temp is `.../yazi-0`, socket binds, no collision with non-root namespaces. |
| UNVERIFIED | 22. Mobile/non-root execution | As mobile/non-root UID, runtime/temp carry that UID, socket binds, no collision with uid 0. |
| UNVERIFIED | 23. Rootful jailbreak | On a rootful device, all of the above resolve from env/passwd/`/tmp` with no hard-coded `/var/jb` branch. |
| UNVERIFIED | 24. Rootless jailbreak | On a rootless device, long preboot/bootstrap `XDG_RUNTIME_DIR` values trigger only the length fallback, never a jailbreak-specific path branch. |

## Temp

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | 25. Temp previews/files work | Image/text previews, remote cache buckets, and stamp files under the temp root are created and read. |
| UNVERIFIED | 26. Temp directory creation | The `yazi-UID` temp child is created on demand, absolute, private, and UID-separated. |
| UNVERIFIED | 27. Reboot/relaunch behavior | After reboot, temp content may be gone but Yazi recreates its temp child without treating it as persistent state; config/state persist under home. |
| UNVERIFIED | 28. No collision between UIDs | Root and mobile temp directories coexist as distinct `yazi-0` vs `yazi-<mobile>` entries. |

## Failure

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | 29. Unwritable explicit runtime directory | Point `XDG_RUNTIME_DIR` at an unwritable absolute directory with a fitting length; DDS reports a useful ownership/permission failure rather than binding elsewhere silently. |
| UNVERIFIED | 30. Ownership mismatch | Pre-create the runtime parent owned by a different UID; `create_owned_dir` rejects it with a permission error. |
| UNVERIFIED | 31. Symlink final directory rejection | Replace the runtime parent with a symlink; creation opens with `O_NOFOLLOW` and fails rather than following it. |
| UNVERIFIED | 32. Useful failure rather than unexplained panic | With no valid home and no passwd entry (where reproducible), Yazi reports a clear config/state directory failure naming HOME/passwd rather than an opaque `.expect` panic; all other misconfigurations above degrade to explicit fallbacks or typed I/O errors. |
