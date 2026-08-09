# Agent Guidelines

Shared Yazelix agent workflow and release policy live in the main repo:

- https://github.com/Yazelix/nova/blob/main/AGENTS.md
- In sibling local checkouts, read `../yazelix/AGENTS.md` first

Only popup-plugin-specific guidance belongs here.

## Local Scope

- This repo owns the standalone `yzpp` Zellij popup plugin.
- Keep popup specs generic and argv-based; main Yazelix owns generated popup specs, close hooks, and config UI policy.
- Preserve the `yzpp` alias and wasm artifact contract.

## Local Commands

- `cargo fmt --all -- --check`
- `cargo test --lib`
- `cargo build --target wasm32-wasip1 --profile release`
- `nix build .#yazelix_zellij_popup --no-link`

## Integration Notes

The package artifact is `share/yazelix_zellij_popup/yzpp.wasm`. For coupled runtime changes, publish this child commit before updating the main repo lock.
