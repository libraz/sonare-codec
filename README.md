# sonare-codec

From-scratch Rust audio codec library with a unified API for native Rust, WASM,
Node, and Python.

The current implementation is an early scaffold:

- `sc-core`: shared PCM types, errors, traits, and format detection
  plus PCM diff/tolerance helpers for round-trip validation
- `sc-decode`: Symphonia-backed decode adapter for the unified API
- `sc-wav`: WAV decode/encode for PCM-oriented bootstrapping
- `sc-flac`: FLAC decode/encode workbench and encoder implementation
- `sc-aac`/`sc-mp4`: AAC ADTS framing, section/spectral payload scaffolding,
  and minimal ADTS-to-M4A mux helpers for the AAC encoder
- `sc-mp3`: MP3 Layer III header/side-info/frame assembly, main-data capacity
  helpers, and limited PCM encoder paths
- `sc-vorbis`: Ogg Vorbis decode plus Ogg Vorbis encode through libvorbis
- `sc-opus`: Ogg Opus decode plus native Ogg Opus encode for mono/stereo
  mapping-family-0 streams, with Rust CELT range/energy/PVQ/theta primitives
  under development
- `sonare-codec`: umbrella crate with `decode(input)` and
  `encode(format, pcm)` dispatch, `encode_with_mode` production-only guardrails,
  plus experimental AAC/MP3 encoder helper re-exports behind their feature flags
- `bindings/wasm`: wasm-bindgen package skeleton with unified encode/decode plus
  WAV/FLAC helpers
- `bindings/python`: PyO3/maturin package skeleton with unified encode/decode
  plus WAV/FLAC helpers

## Features

`sonare-codec` defaults to `decode`, `wav`, `flac`, and `mp3` to match the
planned public surface. Unified `decode(input)` is backed by Symphonia for
supported formats, with a narrow AAC fallback for sonare-generated silent ADTS.
WAV and FLAC currently encode real audio. AAC-LC and MP3 have silent compact
paths plus limited non-silent long-block production candidates that are gated by
local decoder-oracle readiness checks; broader standard-codebook,
psychoacoustic, and rate-control work remains incomplete while implementation
lands phase by phase. Vorbis and Opus encode are available for native
Rust/Python builds behind their feature flags.
`encode_with_mode(format, pcm, EncodeMode::ProductionOnly)` rejects lossy inputs
outside the current mono/stereo MP3 MPEG-1 sample-rate and mono/stereo AAC-LC
ADTS/M4A production candidate paths. The `vorbis` feature provides Ogg Vorbis
decode/encode for native builds, and the `opus` feature provides Ogg Opus
decode for mono/stereo mapping-family-0 streams through the Rust, Python, and
WASM surfaces plus native Rust/Python encode. Shared PCM
channel-block extraction, sine-window, MDCT analysis, and
power-law spectral quantization primitives are in place for the AAC and MP3
encoder work, along with AAC section/codebook planning and MP3 Layer III
big-values/count1/rzero region planning. AAC section metadata can now be packed
into bits, and MP3 region planning can be reflected into Layer III side-info
entries. Shared Huffman codeword packing is available for AAC spectral data and
MP3 main-data work, including exact bit-length reporting for side-info fields
and table-driven symbol-to-codeword packing hooks for the AAC and MP3 entropy
coding stages. Quantized AAC sections and MP3 big-values/count1 regions can now
be converted into pair or quadruple symbols for those table-driven paths. AAC
section metadata and section spectral codewords can be concatenated into one
bit-exact payload with caller-supplied codebook tables. AAC and MP3
magnitude-table codewords can be followed by the required non-zero coefficient
sign bits. Byte-padded packed bit buffers can be concatenated without
preserving padding bits, allowing AAC section payloads and MP3 big-values/count1
entropy regions to form continuous payloads. Minimal experimental unit-magnitude
tables are available to exercise those non-zero payload paths; complete standard
AAC/MP3 Huffman tables remain pending. With the `aac` feature enabled,
raw AAC access units can be wrapped as ADTS via `frame_aac_adts`, and ADTS AAC
frames can be muxed as a minimal M4A container through `mux_aac_adts_as_m4a`.
The locally supported M4A layout can also be demuxed back to ADTS through
`demux_m4a_as_aac_adts`.

## WAV status

WAV currently supports PCM decode for 8-bit unsigned and 16/24/32-bit signed
integer samples, plus 32-bit float samples. Encoding supports PCM16 by default,
with PCM24 and Float32 available through `sc_wav::encode_as`.

## FLAC status

FLAC currently parses `STREAMINFO` metadata and frame headers. Decode supports
early independent-channel frames using constant, verbatim, and fixed-predictor
subframes with Rice residuals, LPC subframes, plus left-side, right-side, and
mid-side stereo decorrelation. Multiple frames are concatenated into one PCM
buffer, including 32-bit independent-channel samples and extended coded frame
or sample numbers. Frame header CRC8, frame footer CRC16, frame block-size
ranges including the final-frame minimum-size exception, frame-size ranges,
declared total sample counts, and non-zero STREAMINFO MD5 checksums are
validated. `FlacDecoder` buffers chunked input until a complete stream is
available; incremental PCM emission is still pending.

FLAC encoding currently writes valid 16-bit frames with STREAMINFO, CRC, and MD5
metadata, choosing constant subframes for flat channels, fixed-predictor order
1-4/Rice subframes for simple smooth channels, stereo decorrelation for
two-channel input, and verbatim subframes as a fallback.

## AAC status

AAC currently supports ADTS framing, minimal ADTS-to-M4A muxing, encoding
silent mono/stereo PCM as AAC-LC ADTS frames, and routing non-silent mono/stereo
PCM through long-block scaffold helpers. The local AAC decoder only
recognizes sonare-generated silent AAC-LC ADTS, including the minimal M4A
container emitted by the local muxer, as a round-trip fallback; general AAC
decode is delegated to Symphonia. Limited non-silent AAC-LC production
candidate streams are available as ADTS and M4A for mono/stereo PCM at the
supported AAC-LC sample rates and are checked by the local FFmpeg-backed
readiness oracle, but
long-block MDCT analysis, scalar quantization, section planning,
section metadata packing, spectral pair extraction, and table-driven section
spectral payload packing are in place. Magnitude-keyed spectral
codewords can also append sign bits for non-zero coefficients before section
payload concatenation; escape-codebook magnitudes can clamp the Huffman lookup
key to 16 and append escape suffix bits for larger coefficients through
sectioned payloads and quantized ADTS helpers. When caller-supplied magnitude
tables are available, experimental helpers can choose AAC section codebooks by
comparing actual packed spectral bit lengths before writing section metadata;
mono/stereo quantized ADTS helpers can opt into that bit-cost section planning
for spectral-only, scale-factor-bearing, or internally selected scale-factor
payloads, and one-block or stream PCM ADTS helpers can use the same bit-cost
section path.
Caller-supplied scale-factor bits can be placed between section metadata and
spectral payload bits. Scale-factor DPCM deltas can be planned for non-zero
sections and packed through caller-supplied Huffman tables and wired into
mono/stereo quantized ADTS helpers and one-block PCM helpers, including PCM ADTS
stream helpers with per-frame or internally selected scale-factor lists.
A basic magnitude-derived per-band scale-factor selector is available as a
deterministic seed for those DPCM paths and can be used directly by
mono/stereo quantized ADTS helpers and PCM helpers.
Sectioned spectral payloads can now be written into a long-block individual
channel stream, single-channel raw data block, and independent-stereo
channel-pair raw data block without inserting byte padding between syntax
fields; pulse, TNS, and gain-control presence flags are emitted after each
long-block payload. Explicit
mono/stereo helpers can frame caller-quantized long-block spectra as AAC-LC
ADTS frames when supplied with matching Huffman tables, and experimental PCM
helpers can now run long-block analysis, quantization, payload packing, and ADTS
framing over one block or a multi-frame ADTS stream through the same path.
Experimental mono/stereo frame-level quantizer step search can now try
candidate steps against the available spectral tables and ADTS frame limits,
allowing non-zero long-block payload scaffolds to be selected without falling
directly back to the all-zero public scaffold. The step search evaluates all
candidates rather than depending on candidate order, and can report the
selected step together with ADTS frame length and frame capacity for the future
rate-control path. A standard AAC scale-factor delta table provider is also
available so non-zero helper paths can pack scale-factor DPCM without callers
building local test tables.
The auto-step non-zero AAC helper path remains experimental; production
candidate output uses bitrate-budgeted stream selection and is still limited
until the remaining standard spectral codebooks replace the local experimental
tables. The standard unsigned-pairs codebook 7/8 tables are exposed as
`aac_unsigned_pairs7_table` and `aac_unsigned_pairs8_table`, with the older
unit-magnitude helper retained for diagnostics, and re-exported by the umbrella
crate so callers can verify the production-shaped spectral packing surface.
The minimal MP4 helper can demux the M4A layout produced by the local muxer back
to ADTS through the public `demux_m4a_as_aac_adts` helper.
Complete standard codebook tables, full standard bit-cost search/rate control,
psychoacoustically correct scale-factor selection, and broader production
`encode()` coverage remain pending.

## MP3 status

MP3 currently supports MPEG audio header parsing, Layer III side-info packing,
frame assembly, Layer III main-data capacity reporting, and encoding PCM at
MPEG-1 sample rates 32/44.1/48 kHz into 128 kbps Layer III frames. The local
MP3 decoder only
recognizes sonare-generated silent Layer III frames as a round-trip fallback;
general MP3 decode is delegated to Symphonia. Limited non-silent mono/stereo MP3
production candidate streams are checked by the local FFmpeg-backed readiness
oracle. Full MP3 psychoacoustic analysis, broader Huffman-table coverage, and
bit reservoir use are still pending; the
long-block MDCT analysis and scalar quantization primitives are present but not
yet connected to Huffman payload coding; interleaved PCM can now be extracted
into zero-padded analysis blocks and classified into Layer III entropy regions
for those stages. Region metadata can be written into side-info, and
preselected main-data codewords can now update `part2_3_length` with the exact
bit count, which is one of the required bridges between payload packing and
side-info.
Big-values pairs can also be routed through caller-supplied Huffman tables as
the table-selection work is filled in, and the planned big-values region can be
materialized from quantized spectra before packing. The Layer III count1 region
is similarly represented as quadruple symbols and can be routed through a
caller-supplied count1 table. Big-values and count1 packing can now use
magnitude-keyed Huffman tables and append MP3 sign bits for non-zero
coefficients; big-values packing also handles escape-table linbits. A
conservative big-values table-class selector can choose representative table
ids and linbits from the maximum magnitude, while a count1 selector can choose
the side-info `count1table_select` flag from quad density. Packed big-values and
count1 regions can now be concatenated and reflected into `part2_3_length`.
Given caller-supplied big-values/count1 Huffman tables, a quantized spectrum can
now be converted into one granule/channel entropy payload with side-info fields
updated; a table-provider wrapper can now select big-values table ids per
region and the count1 table flag by comparing actual packed bit lengths for
available tables, then map those ids to the corresponding table slices before
packing.
The standard table provider now includes the MPEG-1 Layer III big-values table
1 for unit-magnitude pairs and count1 tables 32/33, so the small non-zero
big-values/count1 path no longer depends on experimental codeword tables.
Larger big-values tables, escape tables beyond the existing linbits plumbing,
and bitrate-aware table selection are still being filled in.
Caller-supplied scale-factor bits can also be concatenated before the entropy
regions, including through the quantized-spectrum packing helpers, and
reflected into `part2_3_length`.
MPEG-1 Layer III long-block scale-factor values can be packed with a selected
or automatically selected `scalefac_compress` value and reflected into side-info
metadata, and those generated scale-factor bits can now feed directly into the
quantized-spectrum packing helpers to produce one granule/channel main-data
payload. A deterministic magnitude-derived long-block scale-factor selector is
available as a syntax-valid seed for that path, and zero-padded PCM long-block
analysis can now feed the selected-scale-factor payload helpers directly.
Granule/channel payloads can be concatenated in Layer III main-data order and
assembled into a frame alongside the updated side-info, and a one-frame PCM
long-block scaffold can now drive that sequence for MPEG-1 Layer III headers.
The same scaffold can emit a multi-frame stream and matches the existing silent
encoder output for silent mono/stereo PCM; the production silent `encode()` path
now routes through that scaffold. Public non-silent MP3 `encode()` now uses the
standard table provider with frame-level quantizer step search, so accepted
non-zero long-block payload scaffolds can be emitted instead of falling back to
the all-zero scaffold. The step search now evaluates all candidates rather than
depending on candidate order, and can report the selected step together with
payload bit length and frame capacity for the future rate-control path.
`layer3_header_for_capacity`, `layer3_main_data_capacity_bytes`, and
`layer3_main_data_capacity_bits` expose the per-frame Layer III payload budget
for callers and wrapper crates, and the bitrate-selected stream helper derives
the MPEG header and per-frame capacity from a caller-selected Layer III bitrate.
The umbrella crate re-exports the MP3 scaffold helpers and related
side-info/table-selection types behind the `mp3` feature.
Complete standard big-values table implementation beyond table 1, full standard
bit-cost search/rate control, psychoacoustic scale-factor selection, and full
non-silent encode integration remain pending.

## Bindings status

WASM and Python expose `detect_format`, `decode_audio(input)`, format-specific
decode helpers, and
`encode_audio(format, sample_rate, channels, samples)` as the package-facing
API. `format` accepts `wav`, `flac`, `mp3`, `vorbis`, `opus`, `aac`, `m4a`, or
`mp4`; unsupported encoder paths currently return `UnsupportedFormat` or
`UnsupportedFeature`. Legacy `decode_wav`, `decode_flac`, `encode_wav`, and
`encode_flac` helpers remain available. WASM and Python also expose
`StreamDecoder`, which buffers chunked input and returns PCM once the
accumulated bytes form a complete stream. Small lossy diagnostic helpers are
available on both package surfaces for AAC-LC ADTS bitrate frame budgets, the
AAC unsigned-pairs codebook 7/8 tables, and MP3 Layer III
main-data capacity. WASM and Python also expose caller-selected AAC/M4A and MP3
bitrate encode helpers for the current lossy scaffolds.

## License and clean-room policy

This project is Apache-2.0. Codec implementations are written from scratch.
Decode is delegated to Symphonia, which is MPL-2.0 and used as an unmodified
dependency. GPL/LGPL reference sources must not be used for AAC or MP3 encoding
work; those tools may only be used as black-box local conformance oracles.

## Development Policy & Provenance

- Decode integration uses Symphonia's public API through `sc-decode`.
- Clean-room restrictions apply to MP3 encode and AAC-LC encode: implement from
  published specifications, not LAME/FAAC/fdk-aac source.
- Official conformance vectors should come from standards bodies or upstream
  codec projects directly, not copied from Symphonia test assets.
- Codec patent status can vary by jurisdiction and format. This project does
  not grant patent licenses beyond the Apache-2.0 license text; downstream
  users are responsible for checking whether their use requires additional
  patent rights.
- GPL/LGPL tools may be used locally as black-box oracles, but their binaries,
  source, and generated CI dependencies must not be committed.
- `cargo-deny` enforces the permissive/MPL dependency policy; GPL/LGPL/AGPL are
  intentionally absent from the allow-list.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- artifact-check
cargo run -p xtask -- gen-refs
cargo run -p xtask -- name-check
cargo run -p xtask -- qa-check
cargo run -p xtask -- fuzz-smoke
cargo run -p xtask -- oracle-smoke
cargo run -p xtask -- ref-check
cargo run -p xtask -- release-check
cargo run -p xtask -- package-preflight
cargo run -p xtask -- publish-plan
cargo run -p xtask -- publish-preflight
cargo run -p xtask -- publish-readiness
cargo run -p xtask -- size-report
cargo run -p xtask -- tool-check
cargo check --manifest-path fuzz/Cargo.toml --bin wav_decode
cargo check --manifest-path fuzz/Cargo.toml --bin flac_decode
cargo check --manifest-path fuzz/Cargo.toml --bin aac_decode --bin vorbis_decode --bin opus_decode --bin m4a_demux --bin mp3_header
cargo deny check
```

`release-check` runs the local release gate: fmt, package metadata consistency,
check, tests, clippy, ref-check, fuzz-smoke, fuzz target checks, a WASM target
check when `wasm32-unknown-unknown` is installed, cargo-deny, and an npm package
dry run. Set `SONARE_CARGO_DENY=/path/to/cargo-deny` when `cargo deny` is not
installed as a cargo subcommand.

`artifact-check` builds and verifies the package artifacts that do not require a
git `HEAD`: wasm-pack output, npm pack contents including `LICENSE` and
`NOTICE`, generated WASM production encode entrypoints, and the Python wheel
including its license and notice files plus a temporary install smoke test. It
is a useful preflight subset before the first commit exists.

`package-preflight` is the stricter publish-artifact gate. It expects a valid
git `HEAD` plus `wasm-pack` and `maturin`, then checks Rust package contents
and runs the npm/PyPI artifact builds. It also invokes `qa-check`; local runs
skip missing optional QA tools, while CI installs nextest, audit, machete, and
semver checks so that publish preflight exercises those gates together. Set
`SONARE_WASM_PACK` or `SONARE_PYTHON` when using tools installed outside
`PATH`. Set `SONARE_REQUIRED_QA_TOOLS=nextest,audit,machete,semver-checks`
locally to make those optional QA tools mandatory. Set
`SONARE_CHECK_REGISTRY_NAMES=1` before first publish to make this preflight also
fail if any planned crates.io, npm, or PyPI package name is already registered.

`publish-preflight` is the mandatory first-publish gate. It runs the package
preflight checks with registry name availability required, then runs
`publish-readiness` so the current MP3/AAC-LC production candidate outputs are
decoded and checked before publish.

`size-report` reads existing Rust `.crate`, npm tarball, WASM, and Python wheel
artifacts and reports their sizes. Run it after `package-preflight` when
comparing publish artifacts.

`tool-check` reports whether publish tools are available, including git `HEAD`,
`cargo-deny`, `wasm-pack` (`SONARE_WASM_PACK` is honored), `maturin`, and the
optional WASM target. It also reports optional QA tools such as `cargo-nextest`,
`cargo-audit`, `cargo-semver-checks`, `cargo-machete`, `cargo miri`, and
`cargo-llvm-cov`; set `SONARE_CARGO_NEXTEST`, `SONARE_CARGO_AUDIT`,
`SONARE_CARGO_SEMVER_CHECKS`, `SONARE_CARGO_MACHETE`, or
`SONARE_CARGO_LLVM_COV` when those tools are installed outside `PATH`. Set
`SONARE_REQUIRED_QA_TOOLS` to a comma-separated list such as
`nextest,audit,machete,semver-checks`, or to `all`, when skipped QA tools should
fail the run. `publish-readiness` separately requires
`SONARE_FFMPEG=/path/to/ffmpeg` for the production MP3/AAC oracle.

`qa-check` runs the optional QA tools when they are available: `cargo nextest
run --workspace`, `cargo machete`, `cargo audit`, `cargo semver-checks`
against `HEAD` when a git baseline exists, `cargo +nightly miri test` for the
core/WAV/umbrella subset, and `cargo llvm-cov`. Missing optional tools are
reported as skipped so the command remains usable on a minimal local setup
unless `SONARE_REQUIRED_QA_TOOLS` makes selected tools mandatory.

`name-check` queries crates.io for every Rust package in the publish order, and
npm/PyPI for the planned binding package names. Registry state can change, so
run it immediately before first publish, or set `SONARE_CHECK_REGISTRY_NAMES=1`
when running `package-preflight`.

`publish-plan` prints the current mandatory preflight, staged Rust
`cargo package`/`cargo publish` sequence, and npm/PyPI publish commands derived
from the workspace package list. It does not publish anything.

`publish-readiness` is the final local release blocker. It requires non-silent
MP3 and AAC-LC ADTS/M4A `EncodeMode::ProductionOnly` encode paths to succeed
and requires `SONARE_FFMPEG=/path/to/ffmpeg`; the production MP3/AAC/M4A
outputs are decoded with a local black-box decoder oracle to f32 PCM, and
effectively silent or uncorrelated output is rejected.

`oracle-smoke` is an optional local-only black-box decoder acceptance check. Set
`SONARE_FFMPEG=/path/to/ffmpeg` to run it; without that environment variable it
skips. It is intentionally separate from CI and release packaging so GPL/LGPL
tools are not pulled into the project.

`gen-refs` refreshes `tests/refs/oracle-smoke/` with Sonare-generated reference
artifacts and a manifest. Set `SONARE_FFMPEG=/path/to/ffmpeg` to record local
FFmpeg acceptance in that manifest.

`ref-check` regenerates those artifacts without FFmpeg and compares them to the
committed refs byte-for-byte, then checks that the manifest fingerprints and
decode metadata still describe the files.

When the pinned toolchain is not already installed locally, install Rust
`1.86.0` before running the commands above.

See `RELEASE.md` for package build, name availability, and publish preflight
steps.
