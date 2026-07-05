---
name: wxemma-multi-wechat
description: Use when a user on macOS needs to run multiple WeChat instances at once (e.g. staying signed in to a personal and a work account). Drives the `wxemma` CLI to add, list, remove, rebuild, and launch isolated WeChat copies. Prefer this over manual steps whenever the task involves multiple simultaneous WeChat logins on a Mac.
---

# Running multiple WeChat instances with wxemma

`wxemma` creates isolated copies of WeChat.app so several accounts can be signed
in at once on one Mac. Install: `brew install larrykoo711/tap/wxemma`.

## Agent usage rules

- Always pass `--json` to parse results, and `--yes` to run without prompts.
- `add`, `remove`, and `rebuild` need `sudo`; tell the user to run those with sudo.
- Exit codes: `0` success, `1` runtime failure, `2` usage error.
- On `{"ok": false, ...}`, read `error.code` to branch.
- JSON is a stable English contract and does not change with `--lang`; only
  human-readable output is localized.

## Commands

- Create one instance: `sudo wxemma add --json --yes`
- Create with a label (ASCII only): `sudo wxemma add --note work --json --yes`
- List: `wxemma list --json`
- Status vs. original: `wxemma status --json`
- Remove by index: `sudo wxemma remove 2 --json --yes`
- Remove and wipe its data: `sudo wxemma remove 2 --purge-data --json --yes`
- Rebuild after a WeChat update: `sudo wxemma rebuild --json --yes`
- Launch: `wxemma open --json` (all) or `wxemma open 2 --json`
- Check environment: `wxemma doctor --json`

## Index allocation

Instances occupy slots `1..=7`. `add` always fills the smallest free index, so
removing a middle instance and adding again refills that same slot. Slot number
and its account data stay paired, which keeps a re-created instance's login.

## What it cannot automate (tell the user)

- Each instance must be logged in by scanning a QR code once.
- WeChat's global keyboard shortcuts (screenshot, activate) are per-account and
  stored in encrypted data; ask the user to disable them per instance to avoid
  multiple instances reacting to the same key.
