# Task 015 device test plan: terminal graphics

Status: **NOT RUN.** Every row below is `UNVERIFIED` until a physical jailbroken
device executes it. Task 015 is compile-validated for `aarch64-apple-ios` only;
compile success proves the probe, driver, SHM, and Lua fallback code links, not
that any terminal on either side of SSH displayed an image.

Run this on the jailbroken iPhone/iPad. Record the iOS version, jailbreak type
(rootful/rootless), jailbreak tool and version, terminal or SSH client, and
whether the device was locked or unlocked at the time of each run.

## Environment to record first

| Item | Value |
| --- | --- |
| iOS version | |
| Jailbreak (rootful/rootless, tool, version) | |
| Launch method (SSH / local terminal) | |
| Terminal or SSH client + version | |
| `yazi --version` | |
| `ya env` output (Brand, Emulator.probe, Drivers.candidates, Kgp SHM, chafa) | |
| `chafa --version` (or "absent") | |
| `ueberzugpp --version` (or "absent") | |
| `tmux -V` (or "absent") | |
| Device locked or unlocked during the run | |

Run rows 1–9 from a **local jailbreak terminal**, rows 10–17 over **SSH**,
rows 18–21 under a **multiplexer**, rows 22–29 for **fallbacks**, rows 30–33
for **shared memory**, and rows 34–37 on each **device mode**.

## Local terminal

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 1 | Unknown terminal, no graphics | `yazi`, hover a `.png`, observe preview | Metadata/text preview (Image, Format, Dimensions, Color) with a "No terminal image renderer available" reason; no hang, no repeated spawn errors | UNVERIFIED |
| 2 | Terminal reporting Sixel | Same, on a Sixel-capable terminal | Image renders via Sixel; `ya env` shows `sixel=true` and a Sixel-led candidate chain | UNVERIFIED |
| 3 | Kitty-compatible terminal | Same, on a Kitty-compatible terminal | Image renders via KGP; modern `Kgp` for a known Kitty brand, legacy `KgpOld` placement for an unknown terminal with only a successful KGP query | UNVERIFIED |
| 4 | Known IIP terminal if available | Same, on an iTerm2/WezTerm-compatible terminal | Image renders via IIP; candidate chain leads with `Iip` ahead of `Sixel` | UNVERIFIED |
| 5 | Cell-size response | `ya env`, check `TERM.dimension` and preview geometry | Non-zero cell pixel size; image area is scaled to cells rather than raw config maxima | UNVERIFIED |
| 6 | No cell-size response | Same, on a terminal that never answers `CSI 16 t` | Preview still works using configured maxima / job rectangle; no panic or zero-size area | UNVERIFIED |
| 7 | Image show | Preview several images in sequence | Each image appears in the preview pane at the right position and size | UNVERIFIED |
| 8 | Image erase | Move selection off an image, switch tabs | Previous image is erased; no ghost output remains in the preview area | UNVERIFIED |
| 9 | Repeated preview switching | Rapidly move across images and directories | No stale `shown` area, no overlapping renders, UI stays responsive | UNVERIFIED |

## SSH

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 10 | Kitty client over SSH | `ssh user@device`, `yazi`, hover an image | Image renders via KGP base64 transport; KGP itself works, SHM is not attempted | UNVERIFIED |
| 11 | Sixel client over SSH | Same, from a Sixel-capable client (e.g. Foot) | Image renders via Sixel; capability comes from the terminal report, not from SSH absence | UNVERIFIED |
| 12 | iTerm2 client over SSH | Same, from iTerm2 | Image renders via IIP; identity-based selection still applies over SSH | UNVERIFIED |
| 13 | Plain xterm over SSH | Same, from a plain xterm with no graphics | Metadata/text preview, not a blank pane or a fatal error | UNVERIFIED |
| 14 | No TERM_PROGRAM forwarded | SSH without `TERM_PROGRAM`, capable terminal | Correct protocol selected from active probes (KGP/Sixel queries), not from env branding | UNVERIFIED |
| 15 | Misleading TERM_PROGRAM | SSH with a wrong `TERM_PROGRAM` set | Active probe evidence wins over the misleading brand; preview still correct or gracefully degraded | UNVERIFIED |
| 16 | SSH KGP base64 transport | Row 10, then inspect `ya env` | `Kgp SHM: reported=…, ssh=true, permitted=false`; image still renders | UNVERIFIED |
| 17 | SSH SHM policy | Same | No attempt to share an iOS-device POSIX SHM object with the PC-side terminal; no SHM error surfaces | UNVERIFIED |

## Multiplexer

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 18 | tmux passthrough | `tmux`, then `yazi`, hover an image | `allow-passthrough` is configured automatically; image renders through the passthrough framing | UNVERIFIED |
| 19 | tmux Sixel support | Same, where tmux reports sixel support | Sixel path is selected when the mux sixel flag and DA1 agree | UNVERIFIED |
| 20 | tmux missing | Uninstall or hide `tmux`, run over a plain terminal | No fatal error from `Mux::tmux_setup`; preview works as without a multiplexer | UNVERIFIED |
| 21 | Zellij | `zellij`, then `yazi`, hover an image | Probe-driven selection (same Unknown/Zellij KGP/Sixel rules); no crash, sensible fallback | UNVERIFIED |

## Fallbacks

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 22 | Chafa installed | `chafa` on `PATH`, no graphics protocol | Symbol/text rendering of the image via `chafa -f symbols`; `ueberzugpp` is never spawned for it | UNVERIFIED |
| 23 | Chafa absent | Remove `chafa` from `PATH`, no graphics protocol | Single clean fallback to the metadata preview; no endless spawn loop, Yazi stays usable | UNVERIFIED |
| 24 | ueberzugpp absent | No `ueberzugpp`, no graphics protocol | Chafa path (if present) or metadata preview; selecting Chafa never attempts to spawn `ueberzugpp` | UNVERIFIED |
| 25 | No graphics + no Chafa → metadata | Neither protocol nor `chafa` | Metadata preview with Image/Format/Dimensions/Color plus the renderer-unavailable reason | UNVERIFIED |
| 26 | Malformed image | Hover a truncated/corrupt image file | The real decode diagnostic is shown; it is not replaced by a generic "no renderer" message | UNVERIFIED |
| 27 | Unsupported image format | Hover a format the `image` crate cannot decode | Same: real format/decode error, non-fatal, UI intact | UNVERIFIED |
| 28 | Image with alpha | Hover a transparent PNG | Renders with alpha (KGP RGBA / Sixel palette-zero / IIP PNG) or degrades to metadata without corruption | UNVERIFIED |
| 29 | Large image under allocation limits | Hover an image near `image_alloc`/`image_bound` | Downscaled preview, or the configured allocation diagnostic; no OOM kill of the session | UNVERIFIED |

## Shared memory

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 30 | POSIX SHM succeeds locally | Local terminal, Kitty-compatible, `ya env` shows `kgp_shm=true` | Image renders via the SHM transport (`t=s`) | UNVERIFIED |
| 31 | POSIX SHM denied/unavailable | Local terminal where `shm_open` fails (sandbox denial) | Automatic base64 fallback; preview still renders | UNVERIFIED |
| 32 | SHM failure falls back to base64 | Same as 31 | No fatal error, no leaked shared-memory diagnostic in the UI | UNVERIFIED |
| 33 | No leaked/shared-memory fatal error | Rows 30–32 | No crash, no hang, no leftover `/yazi-*` SHM objects that break later previews | UNVERIFIED |

## Device modes

| # | Scenario | Command / action | Expected | Result |
| --- | --- | --- | --- | --- |
| 34 | Rootful jailbreak | Full matrix subset (1, 3, 7, 23, 25) as root | Same behavior as rootless; no absolute jailbreak helper path is assumed | UNVERIFIED |
| 35 | Rootless jailbreak | Same subset | Identical behavior; no `/var/jb`-style literal is required | UNVERIFIED |
| 36 | Root user | Same subset as `root` | Preview, fallback, and diagnostics behave identically | UNVERIFIED |
| 37 | Mobile/non-root user | Same subset as `mobile` | Preview, fallback, and diagnostics behave identically | UNVERIFIED |

## Notes on interpretation

- Rows 2–4 and 10–12 are the ones that prove SSH is not a "no graphics"
  mode. A metadata-only preview over SSH from a capable terminal is a
  failure of capability detection, not an acceptable fallback.
- Rows 22–25 prove the Chafa/ueberzugpp boundary: Chafa must render without
  `ueberzugpp`, and its absence must land on metadata, never on a loop.
- Rows 16–17 and 30–33 prove the SHM policy: SHM is an optimization for a
  terminal on the same machine, never a requirement, and never attempted
  across SSH.
- A terminal that silently ignores an unsupported escape sequence cannot be
  detected after the write: rows that "render nothing but report success"
  indicate a probe/brand gap, not a failover bug. Record the terminal's
  `ya env` output with such a finding.
