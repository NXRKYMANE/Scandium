# Security Policy

## Supported Versions

The following versions receive security updates and vulnerability reports:

- The latest stable release

## Reporting a Vulnerability

Please **do not** disclose security vulnerabilities through public channels (Issues / Discussions / PRs). Report them privately instead:

- GitHub Security Advisory: repository page → **Security → Report a vulnerability**
- Maintainer email (NXRKYMANE)

Please provide:

- Affected version
- Vulnerability description and impact
- Reproduction steps (if possible)
- Suggested fix (optional)

## Handling Process

- Acknowledgment within 72 hours of receiving the report
- Fix as soon as possible after confirmation and release a patch version
- Vulnerability details are not disclosed publicly until the fix is released

## Security Design

- The service is registered by Osmium and runs with LocalSystem privileges, performing only memory cleanup with no network listening
- Single-instance mutex: `Global\Scandium_WRCS_SingleInstance` prevents multiple instances from cleaning memory concurrently
- No external engines or temporary files: cleanup is performed in-process via Toolhelp32 enumeration and `SetProcessWorkingSetSize(-1, -1)`, with `SeProfileSingleProcessPrivilege` elevated only for the Standby purge
- Rust single-file release (~114 KB, UPX-packed) with no runtime dependencies, reducing the supply-chain surface
