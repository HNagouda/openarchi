# Contributing to openarchi

Thanks for helping. Bug reports, ideas and pull requests are all welcome. Support for a country outside the Netherlands is a good place to start: each data source is one self-contained module in `crates/core/src/sources/`.

## Getting set up

You need Rust (stable), Node 22 and pnpm. [DEVELOPMENT.md](DEVELOPMENT.md) explains how the code fits together, in what order to read it, and where a new diagram, theme or data source goes. The short version:

```sh
cargo test --workspace                                # the core and the MCP server, offline
cd app && pnpm install && pnpm typecheck              # the front end
OPENARCHI_LIBRARY_DIR=../.scratch/library pnpm tauri dev
```

`OPENARCHI_LIBRARY_DIR` points the app at a throwaway folder, so working on openarchi never touches your real projects.

## Where code goes

Everything a user sees comes out of `crates/core`: fetching a site, styling it, rendering the sheet and exporting it. The Tauri app in `app/` and the MCP server in `crates/mcp` are thin shells that call into it. Put new behaviour in the core with a test next to it, then call it from the shell, so the app and an assistant never disagree about what a sheet looks like.

## Branches

1. Branch off `dev`, for example `feature/belgian-data`.
2. Open a pull request into `dev`. CI checks formatting, runs clippy and the tests on macOS, Windows and Linux, type-checks the front end, and builds the app.
3. Once merged, the [dev build](https://github.com/HNagouda/openarchi/releases/tag/dev-build) is replaced with installers that include your change, so you can try it as a real app.
4. `dev` is merged into `main` when it is ready to ship, and a version tag on `main` publishes the release. A CI check rejects pull requests into `main` from any branch other than `dev`.

If users will notice your change, add a line to `CHANGELOG.md` under the next version, written as what they can now do.

## Style

Code reads top to bottom without tricks. Comments say why, not what. Names are full words. Commit messages are one plain sentence about what changed, without prefixes like `feat:`. `cargo fmt` and `cargo clippy -- -D warnings` must pass.

## Licence

openarchi is licensed under the [GNU General Public License v3.0 or later](LICENSE). By opening a pull request you agree that your contribution is released under the same licence. Code copied from elsewhere must come from a GPL-compatible licence, and fonts and data under their own terms (like the OFL fonts in `crates/core/assets/`) keep their notices.
