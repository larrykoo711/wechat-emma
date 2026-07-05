# wechat-emma Design Spec

- **Status**: Approved
- **Date**: 2026-07-06
- **Author**: @LarryKoo
- **Command**: `wxemma`
- **Language**: Rust
- **License**: MIT (for study and communication only)

## 1. Overview

`wechat-emma` is a macOS command-line efficiency tool that lets a user run several
independent WeChat instances at the same time — useful when you need to stay signed
in to more than one WeChat account (for example a personal account and a work
account) on a single Mac.

It is a Rust rewrite of an internal shell prototype (`wxm.sh`), redesigned as a
standalone open-source project with first-class support for LLM/agent usage
(structured JSON output, non-interactive mode), Chinese/English i18n, per-instance
notes, and a configuration file. Compatibility with the old shell script is **not**
a goal; `wxm.sh` will be retired once this tool is ready.

### How it works

Each instance is a copy of `/Applications/WeChat.app` with a modified bundle
identifier and an ad-hoc re-signature, which lets macOS run it as a separate process
alongside the original. Auto-update keys and the `weixin://` URL scheme registration
are stripped from each copy so official updates cannot silently overwrite a copy and
copies do not hijack "open in WeChat" links.

## 2. Goals & Non-Goals

### Goals
- Create / list / remove / rebuild / launch WeChat instances from one CLI.
- Smallest-free-index allocation: instances occupy slots `1..=MAX`, and a removed
  middle slot is refilled on the next `add` (no index conflicts, no runaway numbers).
- Agent-friendly I/O: `--json` structured output and `--yes` non-interactive mode on
  every command that would otherwise prompt.
- Bilingual (zh-CN / en) output driven by system locale or `--lang`.
- Per-instance human notes (ASCII/label only — not Chinese), shown in listings.
- User config file for max instances, naming prefix, and bundle-id base.
- Distributed as a prebuilt universal binary via a Homebrew tap.
- Ship a companion Claude skill and publish it to skills.sh.

### Non-Goals
- No compatibility layer for `wxm.sh` state or naming.
- No Windows/Linux support (macOS-only by nature).
- No modification of the original WeChat.app (must stay Apple-signed and updatable).
- No editing of WeChat's per-account encrypted data (keyboard shortcuts etc. remain
  a documented manual step).

## 3. CLI Surface

Command name: `wxemma`. Subcommands:

| Command | Sudo | Interactive | Description |
|---------|------|-------------|-------------|
| `wxemma add [--note <label>]` | yes | no | Create one instance at the smallest free index |
| `wxemma list` | no | no | List instances (index, name, version, note, status) |
| `wxemma status` | no | no | Original vs. copies version report; flags stale copies |
| `wxemma remove [<index>]` | yes | yes | Remove an instance; if no index, list and prompt |
| `wxemma rebuild` | yes | no | Rebuild all copies from the current WeChat version |
| `wxemma open [<index>]` | no | no | Launch all copies, or one by index |
| `wxemma kill` | no | no | Terminate all copy processes (original untouched) |
| `wxemma doctor` | no | no | Environment preflight (WeChat present, tools, CLT) |
| `wxemma completions <shell>` | no | no | Emit shell completion script |

Global flags: `--json`, `--yes` / `-y`, `--lang <zh|en>`, `--verbose`, `--version`.

### Behavioral rules
- `add`: fails with a clear message when all `MAX` slots are occupied.
- `remove` without an index: prints the instance list, prompts for an index,
  double-confirms deletion, then asks whether to also delete that instance's account
  data. With `--yes`, an index is required and data is preserved unless `--purge-data`.
- `remove --purge-data`: also removes the instance's account data (containers,
  preferences, caches, saved state, HTTP storage, WebKit data) for that bundle id.
- Notes are stored in the config/state file keyed by index; validation rejects
  non-ASCII input (Chinese instance names are disallowed by design).

## 4. Architecture

Single crate, thin binary over a testable library.

```
wechat-emma/
├── Cargo.toml
├── src/
│   ├── main.rs            # arg parsing → dispatch → exit code mapping
│   ├── lib.rs             # public API surface, re-exports
│   ├── cli.rs             # clap derive definitions, global flags
│   ├── commands/          # one module per subcommand (add, remove, ...)
│   ├── instance.rs        # Instance model, index allocation, slot scanning
│   ├── builder.rs         # build_copy: ditto → edit plist → strip keys → sign
│   ├── sysops.rs          # SystemOps trait + RealSystemOps (ditto/codesign/open/pkill)
│   ├── plist.rs           # typed Info.plist read/edit via `plist` crate
│   ├── data.rs            # account-data locations + purge logic
│   ├── config.rs          # config.toml + per-instance notes (serde)
│   ├── output.rs          # human (colored) vs JSON renderer
│   ├── i18n.rs            # rust-i18n init, locale detection
│   └── error.rs           # thiserror domain errors, exit-code mapping
├── locales/               # zh-CN.yml, en.yml
├── tests/                 # integration tests against a mock SystemOps
├── .github/workflows/     # ci.yml (test/clippy/fmt) + release.yml (build+tap)
└── skill/                 # companion Claude skill (SKILL.md + metadata)
```

### Key design choices (mixed strategy)
- **Data operations native**: Info.plist editing uses the `plist` crate (kills the
  PlistBuddy string-escaping class of bugs); config/notes/state use `serde`.
- **High-risk system operations via Apple's own tools**, wrapped in the `SystemOps`
  trait: `ditto` (APFS-clone copy), `codesign --force --deep --sign -` (ad-hoc
  re-sign), `open`, `pkill`. The trait lets tests inject a mock so full command logic
  runs without sudo or touching the real filesystem.
- **CLI**: clap v4 (derive) + `dialoguer` for prompts. `--yes`/`--json` disable all
  interaction so an agent can run unattended.
- **Errors**: `thiserror` domain errors surfaced through a top-level handler that maps
  to exit codes — `0` success, `1` runtime failure, `2` usage error.
- **i18n**: `rust-i18n` with YAML catalogs (zh-CN default-fallback + en), locale from
  `--lang` → `LANG`/`LC_ALL` → default.

### Copy build pipeline (`builder.rs`)
1. Remove any stale copy at the target path (kill process first).
2. `ditto` original → destination (APFS clone, near-instant).
3. Edit Info.plist: set `CFBundleIdentifier`, `CFBundleDisplayName`,
   `CFBundleName`; delete `CFBundleURLTypes`, `SUPublicEDKey`,
   `SUEnableInstallerLauncherService`.
4. `xattr -cr` + remove `_CodeSignature`, then `codesign --force --deep --sign -`.
5. Verify with `codesign --verify --deep --strict`; fail loudly with captured stderr.

## 5. Data Flow

`main` parses args → resolves config + locale → constructs a command with a
`RealSystemOps` → command reads instance state by scanning `/Applications` and the
config notes → performs its action → returns a typed result → `output` renders it as
colored human text or JSON → `main` maps the outcome to an exit code.

## 6. Error Handling

- Every fallible boundary returns a domain `Error`; no silent failures.
- `codesign` stderr is captured and shown on failure (not swallowed).
- Sudo-required commands check EUID and fail with an actionable message.
- Under `--json`, errors render as `{"ok": false, "error": {"code", "message"}}` so
  agents can branch on failure.
- Locale/state file corruption degrades gracefully (fallback locale, empty notes).

## 7. Testing Strategy

- **Unit**: index allocation (fill/refill/full), plist edits (via `plist` round-trip),
  note ASCII validation, config load/save, i18n key coverage (no missing keys).
- **Integration**: drive each command against a mock `SystemOps` capturing the exact
  `ditto`/`codesign`/`open`/`pkill` invocations; assert order and arguments. No sudo,
  no real WeChat needed in CI.
- **CI** (`ci.yml` on every PR): `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test`.
- **Manual smoke** (documented): real `add`/`open`/`remove` on a Mac with WeChat.

## 8. Distribution

- **Release** (`release.yml` on `v*` tag): build `aarch64` and `x86_64` release
  binaries, `lipo`-combine into one universal `wxemma`, tar + sha256, attach to a
  GitHub Release.
- **Homebrew**: personal tap `larrykoo711/homebrew-tap`; `release.yml` updates the
  formula's url + sha256 automatically. Install:
  `brew install larrykoo711/tap/wxemma`.
- **Versioning**: SemVer, starting at `0.1.0` on first release; `1.0.0` once the full
  command surface is validated on-device.

## 9. Companion Skill

A Claude skill under `skill/` documents when and how an agent should use `wxemma`
(non-interactive flags, JSON parsing, exit codes, the manual shortcut/login steps it
cannot automate). Published to skills.sh so agents can discover and install it.

## 10. Repository & Process Conventions

- **Branching**: gitflow. `main` (releases) ← `develop` (integration) ← `feature/*`.
- **Language of artifacts**: all code, comments, commit messages, PR text, issues,
  and in-repo docs are in English (Conventional Commits).
- **User-facing docs**: `README.md` and the GitHub repo description are in **Chinese**,
  written for ordinary non-technical WeChat users — simple, friendly, easy to follow.
- **Compliance framing**: positioned strictly as an efficiency tool for users who need
  to stay signed in to multiple WeChat accounts at once; avoids any non-compliant
  or circumvention-style phrasing. README footer states MIT license, for study and
  communication only.
- **README** includes: what it does (plain language), Homebrew install, the companion
  skill install, basic usage, the manual login/shortcut note, and the license/notice.

## 11. Milestones (feature branches)

1. `feature/scaffold` — crate, CLI skeleton, i18n/config/error plumbing, CI.
2. `feature/core-instances` — instance model, index allocation, sysops trait + mock.
3. `feature/build-pipeline` — builder (ditto/plist/sign), doctor.
4. `feature/commands` — add/list/status/remove/rebuild/open/kill wired end-to-end.
5. `feature/distribution` — release workflow, Homebrew tap, universal binary.
6. `feature/skill-and-docs` — companion skill, Chinese README, skills.sh publish.
