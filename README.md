# ✨ Scandium — Windows RAM Clean Service

<p align="center">
  <img src="https://img.shields.io/github/followers/NXRKYMANE?style=social" />
  <img src="https://img.shields.io/github/forks/NXRKYMANE/Scandium" />
  <img src="https://img.shields.io/github/stars/NXRKYMANE/Scandium" />
  <img src="https://img.shields.io/badge/-Rust-FFFFFF?style=flat&logo=rust&logoColor=black" />
  <img src="https://img.shields.io/badge/Gitee-NXRKYMANE-FFFFFF?style=flat" />
  <img src="https://img.shields.io/badge/AtomGit-NXRKYMANEX-FFFFFF?style=flat" />
  <img src="https://img.shields.io/badge/Douyin-Ozones-FFFFFF?style=flat&logo=tiktok&logoColor=white" />
  <img src="https://vbr.nathanchung.dev/badge?page_id=NXRKYMANE.Scandium&color=FFFFFF&leftColor=555555&label=Views" />
</p>

A lightweight high-performance physical memory cleaner service, built entirely on native Win32 APIs. [中文文档](README_CN.md)

Built in high-performance **Rust** as a single native binary (~116 KB, UPX-packed) — no runtime or external engine required on the target machine.

> This project performs memory cleanup natively in-process: it enumerates system processes with the Toolhelp32 API and flushes their working sets via `SetProcessWorkingSetSize(-1, -1)` (EmptyWorkingSet), then purges the system Standby cache via `NtSetSystemInformation` — all in one self-contained binary.

## How It Works

1. Runs a 60-second cycle with four lightweight engines, each with its own per-minute budget computed from live CPU / memory / disk samples:
   - **WorkingSet** (kernel-level `MemoryEmptyWorkingSets`, falls back to per-process EmptyWorkingSet): 1 run/min below 50% memory, stepping up to 5 runs/min at the 50/70/85/95% tiers, logged in `Used` format
   - **Standby** (flush dirty pages to disk + Standby / low-priority Standby purge): 1 run/min, raised to 2/min above 80% memory; the disk-heavy dirty-page write-back is skipped while the disk is ≥60% busy, logged in `Standby` format
   - **FileCache** (system file cache): 1 run/min, only when memory ≥50% — below that, clearing the file cache hurts file performance more than it helps
   - **Maint** (registry cache reconciliation + combine physical memory lists): fixed 1 run/min
2. **Resource-aware gating:** all engines pause entirely when CPU ≥85%; WorkingSet additionally tiers down at ≥30% / ≥60% CPU, so older machines never suffer cleanup-induced load spikes.
3. Each WorkingSet cleanup empties all process working sets (temporarily paging out inactive memory), comparing memory before and after, spread evenly across the cycle.
4. The cache engines purge the system cache lists with elevated `SeProfileSingleProcessPrivilege` and `SeIncreaseQuotaPrivilege` (works under both the LocalSystem service account and a manually elevated console).
5. Single-instance mutex prevents conflicting concurrent cleanups.

## Efficiency Mode (EcoQoS)

Both the service process and the Osmium host run in Task Manager "efficiency mode" (ProcessPowerThrottling), switching on/off automatically by CPU load:

| Component                    | Setting                 | Behavior                                                                          |
| ---------------------------- | ----------------------- | --------------------------------------------------------------------------------- |
| Service (`scandium_svc.exe`) | `eco_qos = "auto"`      | Enters efficiency mode when idle (CPU < 10%), exits when busy (> 30%)             |
| Host (`os.exe`)              | `host_eco_qos = "auto"` | Enters when idle (CPU < 5%), exits when the host or the service gets busy (> 20%) |

Tuning thresholds: edit the deployed config at `ProgramData\Osmium\svcs\scandium_svc.osiml` (fields `eco_qos_idle_cpu_pct` / `eco_qos_busy_cpu_pct` / `host_eco_qos_*`), then `os.exe --refresh scandium_svc`.

## Project Structure

```
Scandium/
├── Project/                         # Rust service source and build (main implementation)
│   ├── service_core.rs              # Main program (multi-engine scheduling + monitoring + cleanup)
│   ├── main.rs                      # Program entry
│   ├── build.rs                     # Build script (version info + UAC manifest embedding)
│   ├── Cargo.toml                   # Project file (edition 2024 / extreme release optimization)
│   └── installer.iss                # Inno Setup installer script
├── Misc/                            # Assets
│   ├── Background.bmp / .png        # Wizard left-side background image (source + bitmap)
│   ├── Proj.bmp                     # Wizard small top-right image (from Proj.png)
│   ├── Proj.ico                     # Installer and program icon
│   └── Proj.png                     # Icon source image
├── Publish/                         # Build output (published exe and installer)
├── .github/                         # GitHub community templates (issues / PR)
├── app.manifest                     # UAC administrator manifest + DPI awareness
├── .release.ps1                     # One-click build script (compile → publish → package)
├── .gitattributes                   # Git language stats exclusion (installer / peripheral scripts)
├── CLAUDE.md                        # AI assistant rules
├── CHANGELOG.md                     # Development log / version history
├── CODE_OF_CONDUCT.md               # Code of Conduct
├── CONTRIBUTING.md                  # Contribution guide
├── LICENSE                          # License (Apache-2.0)
├── NOTICE                           # Attribution notice (copyright + third-party)
├── README.md                        # English documentation
├── README_CN.md                     # Chinese documentation
└── SECURITY.md                      # Security policy
```

## Requirements

**To run:**
- Windows 10 / 11 (or Windows Server equivalent)
- Administrator privileges (UAC manifest included)
- [Osmium](https://github.com/NXRKYMANE/Osmium) — prerequisite framework that registers Scandium as a Windows service (v26.12.1 or later recommended)

**To build:**
- [Rust](https://www.rust-lang.org/tools/install) (stable, edition 2024)
- Inno Setup 7 (only needed for packaging)

## Build

Run from the project root:

```bash
.\.release.ps1
```

The publish output is a single native executable: `scandium_svc.exe` (Rust, no runtime dependencies).

## Inno Setup Installer

Build the installer package:

```bash
# 1. Build the project (as above)
# 2. Install Inno Setup (https://jrsoftware.org/isdl.php)
# 3. Compile the installer
ISCC.exe Project\installer.iss
```

Output: `Publish\scandium-svc-win-x64-setup.exe`.

Installer features:
- Bilingual UI (English / Simplified Chinese), defaulting to system language
- Smart version comparison: silent upgrade, reinstall prompt for same version, downgrade warning
- Installs `scandium_svc.exe`; writes the service TOML config
- Registers and starts the service via Osmium with exit-code checks (Abort / Retry / Ignore on failure)
- Waits for the previous process to exit before replacing files (all install modes)
- On uninstall, deletes the service via Osmium and removes all files

> **Osmium Integration Notes:** TOML paths with backslashes must use single-quoted literal strings; Osmium is located via the registry key `HKLM\...\App Paths\os.exe`; `ExecAndCaptureOutput` captures exit codes and shows an Abort / Retry / Ignore dialog on failure.

## Deployment

Use the Inno Setup installer from [Releases](https://github.com/NXRKYMANE/Scandium/releases) for a complete setup with automatic service registration.

For manual deployment:
1. Copy `scandium_svc.exe` from `Publish/` to the target machine.
2. Install [Osmium](https://github.com/NXRKYMANE/Osmium) (registers `os.exe` to PATH automatically).
3. Register the service: `os.exe --install scandium_svc.toml`
4. Start the service: `os.exe --start scandium_svc`

## Disclaimer

> [!WARNING]
> **This project may cause high pulsed resource usage on some older hardware. Please make sure your computer uses DDR4/DDR5 memory and an SSD/NVMe drive; if the system feels noticeably laggy or even freezes while the service is running, uninstall this service as soon as possible.**

## Development History

> [!NOTE]
> The project is named **Scandium** — I hoped it could make a computer's memory more "durable"; it has proven stable and effective on all three of my machines.
>
> The abbreviation **Scan** also means "to scan", which echoes the idea of scanning memory and cleaning it.

> Back in seventh grade (around 2022, when GPT-3 had just come out), I got really interested in Python, and my parents signed me up for an online Python course. At the time I was using a three-to-four-year-old computer with only 8GB of RAM — even playing Minecraft Java Edition smoothly kept crashing.
>
> At first I couldn't afford a genuine MC account, so I learned about a launcher called HMCL that could play MC, but downloading mods was painfully slow, and I later switched to the PCL2 launcher — where I accidentally discovered that PCL2's memory cleaner worked surprisingly well: a single run could cut memory usage by half, though it bounced back after a few minutes. Since I had a decent grasp of Python, I wrote a script to run PCL2's cleaner a fixed number of times per minute.
>
> Later, to automate it and make it distributable, I hit countless pitfalls. All I wanted at first was auto-start on boot: I put a shortcut into the Start Menu startup folder, but it kept prompting "run as administrator"; repackaging it with UAC didn't help either.
>
> That frustrated me a lot. So I kept digging and learned about the "Windows service" mechanism — writing my Python program as a Win32 service should bypass the problem, right? It turned out the service never ran properly after PyInstaller packaging. After a ton of research I found out that PyInstaller misses the `win32timezone` module; adding it manually still wouldn't run. There was no way around it — Python's way of calling system APIs was just too painful.
>
> So I started wondering whether anything could bypass Python's inherent limitations. Pretty quickly I found the WinSW project on GitHub, and used it to successfully wrap my exe as a system service — the prototype of this project was born. I even founded a studio and named the project WRCS (Windows RAM Clean Service).
>
> But new problems followed: the installer tool I used back then was Advanced Installer — registering, starting and uninstalling services through WinSW was error-prone, administrator permission handling was messy, and installers I built would even run fine on my machine yet fail mysteriously on another.
>
> That left me lost again. By then I was in ninth grade, and preparing for the high school entrance exam forced me to put it aside.
>
> To live up to the principles of "native, high-performance, easy to distribute, lightweight", during the summer of 2025 I learned the basics of C# and Rust, which finally let me call system APIs and DLLs seamlessly, and with the help of AI I wrote the first genuinely usable version of this project, named Hydride (as in hydrogenation) — meaning to make the computer "lighter".
>
> This year, I deeply reworked WinSW into a superset project called Osmium (formerly Silanes, and even earlier WSF — Windows Service Framework, a Python glue project around WinSW). It became very powerful and stable.
>
> Hydride was then heavily reworked to integrate with Osmium natively: it broke away from PCL2, achieved an equivalent yet intelligent native automation service, and was renamed to today's Scandium. The installer is now built with Inno Setup — smaller and far more extensible.

## Sponsor

If this project helps you, feel free to [sponsor us](https://ifdian.net/a/NXRKYMANE).

## License

Licensed under the Apache License, Version 2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE) for details.
