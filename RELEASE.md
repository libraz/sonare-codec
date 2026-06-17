# Release Checklist

This project publishes three artifacts:

- crates.io: `sonare-codec`
- npm: `@libraz/sonare-codec`
- PyPI: `sonare-codec`

## Preflight

Run the unified local gate:

```sh
cargo run -p xtask -- release-check
```

This also checks that the Rust, npm, and Python package names and versions are
kept in sync.

If `cargo deny` is installed as a standalone binary rather than a cargo
subcommand, point `xtask` at it:

```sh
SONARE_CARGO_DENY=/path/to/cargo-deny cargo run -p xtask -- release-check
```

Before publishing, run the package preflight. Unlike `release-check`, this
expects a real `HEAD` commit and the packaging tools to be installed. It also
runs `qa-check`, so optional QA tools that are installed must pass:

```sh
cargo run -p xtask -- package-preflight
```

The CI `package-preflight` job installs `cargo-nextest`, `cargo-audit`,
`cargo-machete`, and `cargo-semver-checks` before running this command. Local
runs may skip missing optional QA tools, but any tool found in `PATH` or through
the `SONARE_CARGO_*` environment variables must pass. Set
`SONARE_REQUIRED_QA_TOOLS=nextest,audit,machete,semver-checks` locally to make
those tools mandatory for `qa-check` and `package-preflight`.

Before first publish, run the mandatory first-publish preflight. It includes the
package preflight checks, makes registry name availability mandatory, and then
runs the final production readiness gate:

```sh
SONARE_REQUIRED_QA_TOOLS=nextest,audit,machete,semver-checks cargo run -p xtask -- publish-preflight
```

The production readiness portion is also available on its own:

```sh
cargo run -p xtask -- publish-readiness
```

This command must pass before public release. It currently fails while
non-silent MP3/AAC production encode paths are still guarded by
`EncodeMode::ProductionOnly`. It also requires
`SONARE_FFMPEG=/path/to/ffmpeg`; the readiness check asks FFmpeg to decode the
production MP3/AAC outputs to f32 PCM and rejects effectively silent or
uncorrelated output.

To check the local publish/tooling environment before that stricter preflight:

```sh
cargo run -p xtask -- tool-check
```

This reports required publish tools such as git `HEAD`, `cargo-deny`,
`wasm-pack`, and `maturin`. It also reports optional QA tools such as
`cargo-nextest`, `cargo-audit`, `cargo-semver-checks`, `cargo-machete`,
`cargo miri`, and `cargo-llvm-cov`.
Set `SONARE_CARGO_NEXTEST`, `SONARE_CARGO_AUDIT`,
`SONARE_CARGO_SEMVER_CHECKS`, `SONARE_CARGO_MACHETE`, or
`SONARE_CARGO_LLVM_COV` if those binaries are installed outside `PATH`. Set
`SONARE_REQUIRED_QA_TOOLS` to a comma-separated list such as
`nextest,audit,machete,semver-checks`, or to `all`, when skipped QA tools should
fail the run. `publish-readiness` separately requires
`SONARE_FFMPEG=/path/to/ffmpeg`.

Run the optional QA gate when those tools are installed:

```sh
cargo run -p xtask -- qa-check
```

This runs `cargo nextest run --workspace`, `cargo machete`, `cargo audit`,
`cargo semver-checks` against `HEAD` when a git baseline exists, `cargo
+nightly miri test` for the core/WAV/umbrella subset, and `cargo llvm-cov`.
Missing optional tools are skipped, but installed tools must pass.
`SONARE_REQUIRED_QA_TOOLS` can make selected optional tools mandatory.

Before the first commit exists, the git-dependent Rust package checks cannot
run. Use the artifact-only subset to build and verify the WASM/npm/Python
outputs, including packaged license and notice files, without requiring `HEAD`:

```sh
cargo run -p xtask -- artifact-check
```

It checks package metadata consistency, runs `wasm-pack build --target bundler`,
verifies the generated WASM package entrypoints, runs `npm pack --dry-run
--ignore-scripts` and `npm pack --ignore-scripts`, then verifies the npm
tarball contents. It also runs `python -m maturin build`, then verifies that the
Python wheel contains the type stub, `py.typed`, the license and notice files,
and expected package metadata. The npm tarball check also expands the package
and verifies the generated WASM entrypoints include the production encode API;
the Python wheel check installs the wheel into a temporary target and
smoke-tests `encode_audio_production`.

`package-preflight` adds the git-dependent Rust packaging layer: it runs `cargo
package --list` for every Rust crate in publish order, checks that required
files such as `LICENSE` and `NOTICE` are included, and runs `cargo package
--no-verify` for crates that can be packaged before the first internal
dependency publish.

`publish-preflight` is stricter than `package-preflight`: it always queries
registry names and always blocks on `publish-readiness`. This is the command to
run immediately before the initial registry publish.

After `package-preflight`, inspect the artifact sizes:

```sh
cargo run -p xtask -- size-report
```

The report reads existing `.crate`, npm tarball, WASM bundler output, and Python
wheel files. Missing entries mean the corresponding artifact has not been built
in the current checkout yet.

Print the exact publish order and registry commands from the current workspace
metadata:

```sh
cargo run -p xtask -- publish-plan
```

This command is read-only; it does not publish artifacts. It prints the
mandatory `publish-preflight` with `SONARE_REQUIRED_QA_TOOLS`, the Rust crate
publish order, `cargo package -p` checks to run after each internal dependency
becomes available on crates.io, and the npm/PyPI publish commands.

Rust crates that depend on other local crates cannot be fully packaged before
their dependencies exist on crates.io. The workspace compile/test/clippy checks
in `release-check` cover code verification; after each internal crate is
published, normal `cargo package` verification applies to the next crate in the
publish order.

For local black-box decoder acceptance, optionally point `oracle-smoke` at an
installed FFmpeg binary. This command is intentionally local-only; FFmpeg is not
required by CI or package builds.

```sh
SONARE_FFMPEG=/path/to/ffmpeg cargo run -p xtask -- oracle-smoke
```

The command generates WAV, FLAC, MP3, AAC, and M4A artifacts and asks FFmpeg to
decode them to a null sink. It includes the current non-silent MP3/AAC scaffold
artifacts. Passing this check proves external decoder acceptance of the emitted
bitstream shape; it does not prove production audio quality or completion of
MP3/AAC rate control.

To refresh committed local reference artifacts and their manifest, run:

```sh
SONARE_FFMPEG=/path/to/ffmpeg cargo run -p xtask -- gen-refs
```

This writes `tests/refs/oracle-smoke/`. The artifacts are generated by this
project; FFmpeg is only used as a local black-box acceptance oracle and is not
part of CI or the distributed packages.

`release-check` runs `ref-check`, which regenerates the same artifacts without
FFmpeg and compares them to the committed refs byte-for-byte. If an encoder
change intentionally updates these files, rerun `gen-refs` and review the
manifest diff.

## Name Availability

Registry names can change at any time. Check immediately before first publish:

```sh
cargo run -p xtask -- name-check
```

For each registry, `404` means the queried package name does not currently
exist. `200` means it is already registered. The crates.io check covers every
Rust crate in the publish order, not just the umbrella package.
The same check is also available inside `package-preflight` when
`SONARE_CHECK_REGISTRY_NAMES=1` is set.

## Build Artifacts

```sh
cargo package -p sonare-codec
wasm-pack build bindings/wasm --target bundler
(cd bindings/python && python -m maturin build)
```

`cargo package` requires the repository to have a valid `HEAD` commit; it will
not work in an unborn git repository.
If multiple Python versions are installed locally, set `SONARE_PYTHON` before
running `package-preflight` so maturin builds against the intended interpreter.
If `wasm-pack` is installed outside `PATH`, set `SONARE_WASM_PACK` to its
executable path before running `tool-check` or `package-preflight`.

## Publish Order

1. Publish Rust crates in dependency order once the internal crates are ready
   for public API commitments:
   `sc-core`, `sc-mp4`, `sc-wav`, `sc-flac`, `sc-mp3`, `sc-vorbis`,
   `sc-opus`, `sonare-codec-decode`, `sc-aac`, then `sonare-codec`.
   Run `cargo package -p <crate>` immediately before each crate after `sc-core`;
   dependency verification can succeed only after the previous internal crates
   are available on crates.io.
2. Publish `@libraz/sonare-codec` after `wasm-pack` output is verified.
3. Publish `sonare-codec` wheels after `maturin build` succeeds on supported
   platforms.

Do not publish while MP3/AAC encoder status is ambiguous. Unsupported encoder
paths must remain documented as returning an explicit unsupported error. The
`publish-preflight` command is the blocking local check for this condition.
