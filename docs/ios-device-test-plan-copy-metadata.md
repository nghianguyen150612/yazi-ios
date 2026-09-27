# Task 011 physical-device copy metadata plan

Status: **DEVICE-UNVERIFIED**. No row below has been executed on a physical device.
Use the exact Task 011 commit and record device/iOS/jailbreak versions, user,
rootful/rootless layout, source/destination mounts, filesystem types, and tools.
Run the matrix in both rootful and rootless writable test directories, including
another writable jailbreak filesystem/location. Never use valuable user files.

Capture source mode, atime, mtime, and birth time before starting Yazi. Capture
destination metadata immediately after copy, before hashing/reading it (which can
change atime). Use a metadata tool that reports creation time and subsecond
precision; record missing capabilities rather than substituting ctime. Compare
contents with an available hash tool or `cmp`. Record precision/rounding and any
atime mount policy. Mode checks concern permission bits, not ownership or ACLs.

| Case | Required evidence | Status |
| --- | --- | --- |
| Normal same-filesystem file copy | Contents/byte count match | UNVERIFIED |
| Mode preservation | Source/destination permission bits match | UNVERIFIED |
| Atime preservation where meaningful | Pre-copy source vs immediate destination; mount policy recorded | UNVERIFIED |
| Mtime preservation | Values and filesystem precision recorded | UNVERIFIED |
| Birth/creation time preservation | Values reported separately from ctime; missing source remains absent | UNVERIFIED |
| Optimized Apple copy success | Trace clone/copyfile success with device-supported tracing | UNVERIFIED |
| Reproducible PermissionDenied/Unsupported from optimized copy | Capture syscall, raw errno, and Rust ErrorKind; record setup | UNVERIFIED |
| Manual fallback succeeds | Trace entry into fallback and stream completion; contents/mode/times match | UNVERIFIED |
| Large-file progress | Nonzero advancing progress when copy lasts over three seconds, completion, exact bytes | UNVERIFIED |
| Other writable jailbreak location/filesystem | Contents/mode/times checked; clone EXDEV may use std copyfile without Yazi fallback | UNVERIFIED |
| SSH-driven Yazi copy | Real terminal session, progress/completion and metadata comparisons | UNVERIFIED |
| Rootful paths | Complete matrix in isolated writable rootful test paths | UNVERIFIED |
| Rootless paths | Complete matrix in isolated writable rootless test paths | UNVERIFIED |
| Read-only destination | Error remains visible; no fake completion or source deletion | UNVERIFIED |
| Destination creation/read/write failure | Hard failure remains an error; partial destination recorded | UNVERIFIED |

For the fallback case, first seek a reproducible native clone/copyfile rejection
on disposable device storage. A cross-filesystem copy alone is not proof of
Yazi fallback: std handles clone EXDEV internally. If native PermissionDenied or
Unsupported cannot be reproduced, leave that row UNVERIFIED. A temporary debug
build with scoped fault injection can separately exercise fallback plumbing, but
must be labeled simulated; it cannot establish native rejection behavior. Remove
all instrumentation before acceptance. Verify source preservation on failed
copy-based moves separately if exercising moves.

Record outputs and per-row PASS/FAIL/UNVERIFIED for the exact source SHA. CI
compilation and Linux filesystem tests do not satisfy this device matrix.
