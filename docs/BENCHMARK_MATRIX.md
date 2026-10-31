# Supported Hardware and OS Test Matrix

This inventory follows the support tiers in [TECH_STACK.md §6](TECH_STACK.md#6-platform-support-matrix), the physical 4 GB requirement in [PRD.md §5](PRD.md#5-formal-measurable-acceptance-criteria), and the measurement method in [ARCHITECTURE.md §8](ARCHITECTURE.md#8-performance-budget-and-benchmark-method). It records development test-machine availability; it does not certify performance or release acceptance.

The reference requirement is **4 GB installed physical RAM and CPU-only operation**. A VM limit, cgroup limit, or other simulated memory constraint can supplement testing, but cannot replace the PRD's representative physical 4 GB certification machine. Wayland remains best-effort and depends on compositor and portal capabilities; use the documented UI and explicit Copy fallback when shortcuts or insertion are unavailable.

| Target ID | OS and supported version | Architecture | Existing support tier | Required physical RAM / mode | Availability | Verified physical machine | Outstanding verification |
|---|---|---|---|---|---|---|---|
| `macos_13_plus_arm64` | macOS 13+ | arm64 | Tier 1, subject to Phase 0 validation | 4 GB; CPU-only | **AVAILABLE** for macOS platform work; **not** a 4 GB reference machine | `MAC-ARM64-01`: macOS 26.6.2 (25G83), MacBook Pro (Mac15,6), Apple M3 Pro, arm64, **18 GB installed physical RAM** | Identify a physical 4 GB Apple Silicon reference machine or use the PRD's nearest available low-memory Apple Silicon plus constrained validation as supplemental evidence. Benchmark CPU-only and retain the 4 GB certification gap until representative hardware is tested. Verify installer, audio, focus/permission behavior, and performance gates. |
| `macos_13_plus_intel_x64` | macOS 13+ | x64 | Tier 2, gated | 4 GB; CPU-only | **UNAVAILABLE** in the current task environment | None identified | Acquire/identify a physical Intel Mac; verify the eligible OS/build, CPU model, installed RAM, Intel compatibility, CPU-only performance, installer, and platform interactions. |
| `windows_10_11_x64` | Windows 10/11 | x64 | Tier 1 | 4 GB; CPU-only | **UNAVAILABLE** in the current task environment | None identified | Acquire/identify physical 4 GB hardware; record exact Windows version/build, CPU and RAM. Verify WebView2/runtime, WASAPI, UIPI, installer, permissions, focus safety, and performance. |
| `ubuntu_family_x11_x64` | Ubuntu-family Linux with X11; exact supported release is not specified in TECH_STACK.md | x64 | Tier 1 | 4 GB; CPU-only | **UNAVAILABLE** in the current task environment | None identified | Confirm a supported distro release; acquire/identify physical 4 GB hardware and record OS/build, CPU and RAM. Verify X11, audio, tray, package dependencies, installer, and performance. |
| `fedora_family_x11_x64` | Fedora-family Linux with X11; exact supported release is not specified in TECH_STACK.md | x64 | Tier 2 | 4 GB; CPU-only | **UNAVAILABLE** in the current task environment | None identified | Confirm a supported distro release; acquire/identify physical 4 GB hardware and record OS/build, CPU and RAM. Verify X11, audio, tray, package/runtime naming, installer, and performance. |
| `gnome_kde_wayland_x64` | GNOME/KDE Linux with Wayland; compositor/portal dependent | x64 | Best-effort | 4 GB; CPU-only | **UNAVAILABLE** in the current task environment | None identified | Test real GNOME and KDE sessions and portal versions on identified physical hardware. Record shortcut, insertion, clipboard fallback, OS/build, CPU and physical RAM per tested compositor; do not imply unconditional Wayland support. |

## Current machine facts

The local host is `MAC-ARM64-01`. `sw_vers` reported macOS 26.6.2, build 25G83; `uname -m` reported `arm64`. Filtered `system_profiler SPHardwareDataType` output reported MacBook Pro, model identifier Mac15,6, Apple M3 Pro, and 18 GB memory. The 18 GB value is the installed physical memory reported by the hardware profile. It is **not** a physical 4 GB machine, and no simulated memory limit is recorded as physical RAM.

The requested `sysctl -n hw.memsize`, `sysctl -n hw.model`, and `sysctl -n machdep.cpu.brand_string` commands each returned `Operation not permitted` in this environment. The model, CPU, and physical-memory values above come from the filtered hardware profile, not from substituting estimates for those denied commands.

## Status and evidence interpretation

- **AVAILABLE** means physical test hardware has been identified and its specifications recorded. It does not mean that it meets the 4 GB reference requirement or has passed any benchmark. The available Mac is 18 GB.
- **PENDING** means hardware exists or is planned but required details are not verified. There are no PENDING rows in the current inventory.
- **UNAVAILABLE** means no suitable physical test machine is available in the current task environment; it makes no claim about hardware outside that environment.
- All targets require a physical 4 GB, CPU-only reference test under the project criteria. No such machine is verified in this inventory. Memory-constrained tests on the 18 GB Mac would remain supplemental.
- Hardware identification is not performance certification. No RSS, installed-size, latency, real-time-factor, speech-quality, battery, or release certification measurement is reported here.

The machine-readable source is [`benchmark-matrix.json`](benchmark-matrix.json). Validate it and run its unit tests from the repository root:

```sh
python3 scripts/validate_benchmark_matrix.py
python3 -m unittest discover -s scripts/tests -v
```

The validator checks inventory structure and evidence fields only. It does not discover hardware, run benchmarks, or establish that an identified host passed an application test.
