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
tables are available to exercise those non-zero payload paths; standard
MP3 big-values/count1 Huffman selector coverage is now exposed, while AAC production
quality control and MP3/AAC full rate control remain pending. With the `aac` feature enabled,
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
mono/stereo quantized ADTS helpers and PCM helpers. Offset-based mono/stereo
selected-scale-factor stream helpers can also run under caller-provided
max-frame or bitrate budgets, and the umbrella crate exposes a
production-shaped ADTS/M4A bitrate helper pair for that internally selected
scale-factor path. The public non-silent AAC-LC production candidate `encode()`
path now uses the same offset-based selected-scale-factor bitrate-budget stream
helpers with a conservative default production budget, so scale-factor
selection, bit-cost section planning, and ADTS frame budgeting stay on one path
for Rust callers.
The standard-id AAC spectral workbench can also plan and pack direct signed
codebook 5/6, quad, unsigned-pair, and escape diagnostic payloads by
scale-factor band offsets, giving the future full codebook production path the
same section metadata shape used by production AAC-LC frames.
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
tables. The standard signed-quad codebook 1/2 tables, signed-pairs codebook
5/6 tables, unsigned-quad codebook 3/4 tables, unsigned-pairs codebook 7/8/9/10 tables,
and escape codebook 11 table are exposed as
`aac_signed_quads1_table`, `aac_signed_quads2_table`,
`aac_signed_pairs5_table`, `aac_signed_pairs6_table`,
`aac_unsigned_quads3_table`, `aac_unsigned_quads4_table`,
`aac_unsigned_pairs7_table`, `aac_unsigned_pairs8_table`,
`aac_unsigned_pairs9_table`, `aac_unsigned_pairs10_table`, and `aac_escape_table`, with the older
unit-magnitude helper retained for diagnostics, and re-exported by the umbrella
crate so callers can verify the production-shaped spectral packing surface. The
AAC quad section workbench is also exposed through a core-owned unit-table
section planner for binding/package diagnostics. A low-level standard-id
spectral section planner can combine standard direct signed-quad codebook 1/2,
unsigned-quad codebook 3/4, standard direct signed-pair codebook 5/6,
unsigned-pair codebooks 7/8/9/10, and escape codebook 11, or caller-supplied
quad codebooks 1-4 with pair/escape codebooks 5-11, without reusing the older
compatibility codebook-1 pair path, and exposes split helpers that keep section
plus scale-factor bits separate from spectral bits for the long-block
individual-channel payload packer, including a dedicated standard
AAC-LC workbench helper and bit-cost planning entry points that feed those
split/scale-factor payload helpers directly. The
umbrella crate also exposes `encode_aac_adts_with_standard_spectral_offsets_and_bitrate`
and `encode_m4a_with_standard_spectral_offsets_and_bitrate`, plus
`encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate`
and `encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate`,
giving Rust, WASM, and Python callers high-level mono/stereo
bitrate-budgeted ADTS/M4A surfaces that use the standard-id offsets,
scale-factor DPCM, and full standard spectral table-set workbench without
making that path the default production encoder yet. The
default magnitude-classified section planner now uses the available standard
unsigned-pairs codebook 7 table for magnitudes up to 7 and codebook 9 for
magnitudes up to 12 before falling back to escape-class sections. A standard
AAC-LC spectral table-set helper that includes escape codebook 11 is also
available for diagnostics and future rate-control work, but production encode
keeps the current oracle-passing table set until escape-coded output passes the
FFmpeg readiness gate.
Low-level spectral quadruple symbols, caller-supplied quad-table section
metadata, and sign-bit section payload helpers are available as the workbench
for standard AAC codebooks 1-4.
The minimal MP4 helper can demux the M4A layout produced by the local muxer back
to ADTS through the public `demux_m4a_as_aac_adts` helper.
Complete standard signed/quad codebook tables, full standard bit-cost
search/rate control, psychoacoustically correct scale-factor selection, and
broader production `encode()` coverage remain pending.

## MP3 status

MP3 currently supports MPEG audio header parsing, Layer III side-info packing,
frame assembly, Layer III main-data capacity reporting, and encoding PCM at
MPEG-1 sample rates 32/44.1/48 kHz into 128 kbps Layer III frames. The local
MP3 decoder only
recognizes sonare-generated silent Layer III frames as a round-trip fallback;
general MP3 decode is delegated to Symphonia. Limited non-silent mono/stereo MP3
production candidate streams are checked by the local FFmpeg-backed readiness
oracle. The production-facing quantizer uses the true polyphase + hybrid MDCT
workbench for mono and keeps stereo on a compatibility cosine-modulated
subband scaffold while the stereo true-polyphase path is brought up to that
oracle. The default non-silent mono/stereo production path now uses the CBR
bit-reservoir packer and is checked against the same selector telemetry in the
publish-readiness gate. MP3 also exposes clean-room psychoacoustic long-block scale-factor helpers that wire zero-padded PCM analysis, the sign-inverted hybrid MDCT spectrum, masking thresholds, and per-band allocation into the scale-factor quantizer workbench, plus frame/stream helpers with payload-budget step search, bitrate-derived frame capacity, and an allocation-active CBR selector that prefers fitting candidates with non-zero scale factors. Non-silent mono/stereo production encode now uses the entropy-targeted perceptual scale-factor reservoir path; the raw and quality-guarded perceptual reservoir helpers remain available as comparison diagnostics. Standard Huffman selector coverage is exposed, while full rate
control is still pending; interleaved PCM can now be extracted into
zero-padded analysis blocks and classified into Layer III entropy regions for
those stages. Region metadata can be written into side-info, and
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
The standard table provider now includes MPEG-1 Layer III big-values tables
1/2/5/7/10/13, the table-16 codeword tree used by escape-class tables 16..=23,
and count1 tables 32/33, so the small non-zero big-values/count1 path no
longer depends on experimental codeword tables. The remaining standard
big-values tables and bitrate-aware table selection are still being filled in.
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
for callers and wrapper crates, the bitrate-selected stream helper derives the
MPEG header and per-frame capacity from a caller-selected Layer III bitrate,
and the reservoir detail helper reports per-frame CBR capacity, selected step,
payload bits, frame length, padding, `main_data_begin`, and post-frame reservoir
state from the same selection pass used by the production reservoir encoder,
including per-frame perceptual-vs-calibrated granule counts for the guarded
psychoacoustic bridge plus quality-guard comparison count and encoder-side
distortion delta telemetry.
The umbrella crate re-exports the MP3 scaffold helpers, standard Huffman selector lists, psychoacoustic
long-block scale-factor selector, perceptual scale-factor frame/stream helpers
with payload-budget and bitrate-derived capacity search, the perceptual active
CBR diagnostic stream helper, the perceptual active reservoir candidate helper,
reservoir detail helpers, and related side-info/table-selection types behind
the `mp3` feature.
Complete standard big-values table implementation beyond the currently wired
tables, full standard bit-cost search/rate control, production psychoacoustic
bit-allocation integration, and full non-silent encode integration remain
pending.

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
AAC default production bitrate budget, AAC production and standard-id step
candidates, AAC signed-pairs codebook 5/6 tables,
unsigned-pairs codebook 7/8/9/10 tables, escape codebook 11 table, codebook 6 section planning, mixed
standard-id payload bit lengths, standard table-set section planning, standard
escape and mixed standard-id payload bit lengths, standard-id AAC-LC mono/stereo
offsets ADTS diagnostic stream helpers with fixed-step and bitrate-derived step
search modes plus frame-selection telemetry, and MP3 Layer III step candidates,
main-data capacity/reservoir
telemetry for the mono/stereo production entropy-targeted perceptual reservoir
path and the raw/quality-guarded comparison helpers. WASM and Python also expose
caller-selected AAC/M4A bitrate encode helpers for fixed and internally
selected scale-factor paths, plus MP3 fixed-padding and CBR padding-scheduled
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
cargo run -p xtask -- aac-standard-diagnostic
cargo run -p xtask -- artifact-check
cargo run -p xtask -- gen-refs
cargo run -p xtask -- name-check
cargo run -p xtask -- mp3-perceptual-diagnostic
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
and requires `SONARE_FFMPEG=/path/to/ffmpeg`; MP3 outputs are checked for the
default 128kbps production frame budget, AAC-LC ADTS/M4A outputs are checked
against the default production bitrate frame budget, then the production
MP3/AAC/M4A outputs are decoded with a local black-box decoder oracle to f32
PCM, and effectively silent or uncorrelated output is rejected.
`aac-standard-diagnostic` uses the same local FFmpeg oracle against the AAC-LC
standard spectral table-set helper output without promoting that path to
production. It searches a small diagnostic global-gain set, reports each
candidate's selected quantizer step, frame length, decoded RMS, and correlation,
then keeps the best correlated candidate with an RMS tie-break toward the input
level. The final summary still reports the standard-table section mix and
default bitrate-derived ADTS frame budget so escape-table, scale-factor, and
future full-codebook work can be tracked separately from the production
candidate. The same readiness diagnostic now also runs the public high-level
standard-id AAC/M4A bitrate helpers for mono and stereo, including the selected
scale-factor plus magnitude-bias variants, checking ADTS frame budgets, FFmpeg
decode, decoded RMS, and correlation while leaving the default production AAC
path on the higher-quality selected-scale-factor table set. Python/WASM also
expose recommended standard-id selected-scale-factor gain/bias parameters, a
combined parameter helper, and convenience ADTS/M4A encode helpers plus
flattened frame-selection telemetry for the same selected standard-id path as
`[frame_index, step, frame_len, frame_capacity_bytes, ...]`. The local
oracle also rejects extremely over-amplified output, not just silent or
uncorrelated PCM.
`mp3-perceptual-diagnostic` uses the same local FFmpeg oracle to decode the
perceptual-scale-factor MP3 helper output at 128kbps CBR without promoting that
path to production. It also reports the CBR padding count, selected step range,
payload bit usage, and frame capacity so rate-control work can distinguish
capacity pressure from the current psychoacoustic/scale-factor quality limit.
The diagnostic uses the allocation-active selector over the normal candidate
set so the reported quality exercises non-zero perceptual scale factors instead
of the finest zero-scale-factor fallback. Python/WASM also expose the first-frame
perceptual candidate profile as flattened
`[step, payload_bits, capacity_bits, nonzero_scale_factors, scale_factor_bands,
max_scale_factor, ...]` telemetry so package smoke tests can detect whether
future MP3 rate-control changes alter scale-factor activation. A companion
perceptual bit-allocation helper exposes flattened
`[frame_index, granule, channel, perceptual_entropy, target_bits, ...]`
telemetry using the same CBR main-data capacity and psychoacoustic entropy
distribution that future reservoir rate-control work will consume. An
entropy-targeted perceptual reservoir details helper then applies those frame
targets to diagnostic step selection and reports whether each frame fit the
entropy budget or fell back to the ordinary borrowed-reservoir budget; the
matching diagnostic encode helper assembles the same selected frames into MP3
bytes. The non-silent mono/stereo MP3 production path now uses that
entropy-targeted selector directly, and publish-readiness verifies production
side-info against its telemetry.

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
