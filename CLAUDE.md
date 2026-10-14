@AGENTS.md

## Claude Code harness notes

`AGENTS.md` (imported above) is the single source of project instructions. This section only
maps the harness pieces to their Claude Code equivalents.

- **Hooks** (`.claude/settings.json`): thin adapters in `.claude/hooks/` that call the shared
  gates in `.agents/scripts/`. Change gate logic there, not in the adapters.
  - `PreToolUse` on Bash/PowerShell: any `git commit` runs `check-pre-commit.ps1`
    (no `unwrap`/`expect`/`panic!`/`todo!`/`dbg!`, clippy, `cargo test --workspace`, reference-data
    separation). The hook judges the **whole** command: if it denies, nothing in that command runs.
    Run generators (`corpus-manifest --write`, `parity --write-baseline`) and `git add` in a
    separate command before `git commit`.
  - `Stop`: when Rust/Cargo files are dirty, runs `stop-hook.ps1` (`cargo fmt` + clippy
    `-D warnings -D clippy::indexing_slicing`) and blocks finishing until clean (max 3 attempts).
- **MCP** (`.mcp.json`): `opendesk` (`opendesk-mcp` from `~/.cargo/bin`) for local and remote
  computer use. The `opendesk` CLI (`opendesk exec --peer old-pc "..."`) works from Bash too.
- **Skill**: `.claude/skills/jaws-reference/` adapts `skills/jaws-reference/` and documents how
  to run it on `old-pc` and move files via the shared Dropbox.
