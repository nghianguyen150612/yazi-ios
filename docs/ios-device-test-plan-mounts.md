# iOS mount discovery device test plan

Every row is **UNVERIFIED** until run on a jailbroken iOS device. A successful
target build only proves that the `getfsstat` declarations link; it does not
prove the enumerated mounts, `st_dev` association, or callbacks are correct.
Record device model, iOS version, jailbreak name and version, rootful/rootless
layout, launch mode (local terminal vs SSH), and Yazi commit for each run.

Task 013 enumerates mounts with `getfsstat(MNT_NOWAIT)` and polls the snapshot
every 10 seconds. Callbacks fire only when the (source, mount point,
filesystem type) identity changes. Mount discovery is auxiliary: failures must
not block startup, and the Task 012 iOS PollWatcher policy is unchanged. No row
below is pre-marked passed. Core logic is layout-agnostic: the rows below may
*inspect* jailbreak paths, but the implementation must not depend on them.

Use `mount` (or `cat /etc/mtab` where present) in an independent shell as the
ground-truth mount table, and `stat`/`df` for `st_dev` and capacity sanity.
Compare against Yazi's `ya fs partitions` Lua output where the plugin API is
reachable.

## Baseline enumeration

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | Enumerate mounts from local terminal | Launch Yazi from a local jailbreak terminal; every `mount`-listed filesystem appears in the partitions output with no startup error. |
| UNVERIFIED | Enumerate mounts over SSH | Launch Yazi through an SSH PTY; the partitions output matches the local-terminal run for the same device state. |
| UNVERIFIED | Root filesystem represented correctly | The `/` mount appears with its real source, `apfs` type, and a sane capacity. |
| UNVERIFIED | Data/private-var filesystem represented correctly | The `/private/var` (data volume) mount appears with its real source and mount point. |
| UNVERIFIED | APFS filesystem type | APFS volumes report `apfs`, not an empty or invented type string. |
| UNVERIFIED | Mount source | Each record's `src` matches the kernel's `f_mntfromname` (e.g. `/dev/diskXsY`, `devfs`) verbatim. |
| UNVERIFIED | Mount destination | Each record's `dist` matches the kernel's `f_mntonname` verbatim. |
| UNVERIFIED | st_dev/rdev association | For a file on a mounted filesystem, the `Cha.dev` value matches that mount's record, so `timeless`/`soundless` classify the right filesystem. |
| UNVERIFIED | Capacity sanity | Reported capacity is within the `df` value for the same mount point (exact block-size accounting may differ). |

## Rootful / rootless layouts

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | Rootful jailbreak layout | On a rootful device, jailbreak mounts appear as reported by the kernel without hard-coded `/var/jb`-style assumptions. |
| UNVERIFIED | Rootless jailbreak layout | On a rootless device, the preboot/bootstrap content appears as reported by the kernel without hard-coded hash-path assumptions. |
| UNVERIFIED | Jailbreak preboot/bootstrap mount where present | Where the environment exposes a preboot or bootstrap mount, it is listed with its real source and destination. |

## Dynamic mount changes

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | Mount new filesystem if test environment permits | Mount a test filesystem (dmg, loop, or otherwise permitted); Yazi's snapshot gains exactly one record within the poll interval. |
| UNVERIFIED | Unmount it | Unmount the test filesystem; Yazi's snapshot loses exactly that record within the poll interval. |
| UNVERIFIED | Callback fires once per real snapshot change | Each real mount/unmount produces one watch/refresh/after-mount cycle, not a repeated storm. |
| UNVERIFIED | Identical poll does not trigger refresh | With no mount change, no mount-triggered refresh occurs between polls. |
| UNVERIFIED | Temporary enumeration failure preserves last good state | If enumeration transiently fails (e.g. under memory pressure), the previous snapshot stays in effect, a nonfatal diagnostic is logged, and the next poll recovers. |

## Optional storage

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | Network filesystem if available | If an NFS/SMB share is mounted, it is listed with its real type string and no fabricated `external`/`removable` flags. |
| UNVERIFIED | FUSE-like filesystem if available | If a FUSE-like filesystem is mounted, it is listed with its real type string. |
| UNVERIFIED | External/removable storage if device/environment supports it | If the environment exposes external storage, it is listed; `external`/`removable` remain unset (unknown) rather than guessed. |

## Integration

| Status | Scenario | Procedure and expected result |
| --- | --- | --- |
| UNVERIFIED | Yazi remains usable if mount discovery fails | With mount enumeration forced to fail (where reproducible), Yazi still starts and browses known paths. |
| UNVERIFIED | Watcher still uses Task 012 iOS PollWatcher policy | Local paths are watched with PollWatcher; mount data does not silently re-enable kqueue watches. |
| UNVERIFIED | No rapid background wake/poll loop | The process shows no sub-second periodic wakeups attributable to mount polling. |
| UNVERIFIED | No fd leak over repeated refreshes | Open descriptor count is stable across many poll intervals and mount changes. |
