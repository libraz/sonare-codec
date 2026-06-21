# sonare-codec

From-scratch Rust audio codec library with a unified API for native Rust, WASM,
Node, and Python.

> ⚠️ **Status: alpha — under active development. Not production-ready.**
> Pre-1.0 software with an unstable API that may change without notice; not yet
> released to crates.io, npm, or PyPI. Decode (via Symphonia) and WAV/FLAC
> encode are functional. MP3 and AAC-LC encode are incomplete production
> candidates, and Vorbis/Opus encode are first-party pure-Rust encoders still
> maturing. See the status table below for what works today.

## Design

Decoding is delegated to [Symphonia](https://github.com/pdeljanov/Symphonia)
(MPL-2.0); the first-party work is the **encoders**, the unified API, and the
WASM/Python packaging. Lossless codecs (WAV, FLAC) are bit-exact; lossy codecs
(MP3, AAC, Vorbis, Opus) target a perceptual tolerance and must never be
compared bit-exactly against a reference decoder.

## Status

| Format | Decode | Encode |
|---|---|---|
| WAV | ✅ PCM u8 / s16 / s24 / s32 / f32 | ✅ lossless — PCM16 default, PCM24/Float32 via `encode_as` |
| FLAC | ✅ fixed/LPC subframes, all stereo modes | ✅ lossless — 16-bit, constant/fixed-predictor + stereo decorrelation |
| MP3 | ✅ (Symphonia) | 🚧 MPEG-1 (32/44.1/48 kHz) + MPEG-2 LSF (16/22.05/24 kHz), mono/stereo, oracle-gated |
| AAC-LC | ✅ (Symphonia) | 🚧 ADTS + M4A, mono/stereo, oracle-gated production candidate |
| Vorbis | ✅ (Symphonia) | ✅ pure-Rust, mono/stereo (block switching + stereo coupling) |
| Opus | ✅ (opus-decoder) | ✅ pure-Rust CELT-only fullband, 48 kHz mono/stereo |

`✅` usable today · `🚧` incomplete production candidate.

The Vorbis and Opus encoders are pure-Rust with no C dependency, so they build
for the wasm target. MP3 and AAC-LC encode emit non-silent production
candidates that are validated by a local FFmpeg-backed decoder oracle, but full
psychoacoustic modelling, standard-codebook coverage, and rate control are still
in progress; their public `encode()` output should be treated as preliminary.

### Per-codec notes

- **WAV** — `sc_wav::encode_as` selects PCM24 or Float32; PCM16 is the default.
- **FLAC** — encoder picks constant subframes for flat channels,
  fixed-predictor (order 1–4) + Rice for smooth channels, stereo decorrelation
  for two-channel input, and verbatim as a fallback. Decode buffers chunked
  input until the whole stream is available (no incremental emission yet).
- **MP3** — Layer III only. Mono uses the true polyphase + hybrid MDCT path;
  stereo currently runs on a compatibility cosine-modulated subband scaffold.
  MPEG-2.5 rates (8/11.025/12 kHz) are clean-room-excluded and unsupported.
- **AAC** — AAC-LC only (no SBR/PS/HE-AAC). Includes ADTS framing and a minimal
  ADTS↔M4A mux/demux.
- **Opus** — CELT-only fullband, Opus mapping family 0. SILK/hybrid modes and
  multistream mappings are not implemented.

The umbrella crate dispatches through `decode(input)` and `encode(format, pcm)`.
`encode_with_mode(format, pcm, EncodeMode::ProductionOnly)` gates only the MP3
and AAC-LC scaffold paths — it rejects non-silent MP3/AAC input at unsupported
sample rates or channel counts, while WAV, FLAC, Vorbis, and Opus always encode.
The incomplete AAC and
MP3 encoders also expose a large set of low-level helpers (section/codebook
planning, scale-factor and entropy packing, step search, reservoir telemetry)
behind the `aac`/`mp3` features for the in-progress encoder work.

## Bindings

WASM and Python expose `detect_format`, `decode_audio(input)`,
`encode_audio(format, sample_rate, channels, samples)`, format-specific decode
helpers, and `StreamDecoder` (buffers chunked input and returns PCM once a
complete stream is accumulated). `format` accepts `wav`, `flac`, `mp3`,
`vorbis`, `opus`, `aac`, `m4a`, or `mp4`; unsupported encoder paths return
`UnsupportedFormat` or `UnsupportedFeature`. Legacy `decode_wav` / `decode_flac`
/ `encode_wav` / `encode_flac` remain available, alongside diagnostic helpers
for the in-progress AAC/MP3 encoders.

## Development Policy & Provenance

This project is Apache-2.0. First-party code is written from scratch.

- Decode integration uses Symphonia's public API through `sc-decode`; Symphonia
  (MPL-2.0) is an unmodified dependency.
- The Vorbis and Opus encoders are pure-Rust and contain portions ported from
  the Xiph.Org BSD-3-Clause sources (libvorbis/aoTuV, libogg, libopus); those
  ported modules are derivative works and the upstream notices are reproduced in
  [`LICENSE-THIRDPARTY`](LICENSE-THIRDPARTY).
- Clean-room restrictions apply to MP3 encode and AAC-LC encode: implement from
  published specifications, not LAME/FAAC/fdk-aac source (ISO/IEC 11172-3 /
  13818-3 for MP3, ISO/IEC 14496-3 for AAC-LC).
- Official conformance vectors should come from standards bodies or upstream
  codec projects directly, not copied from Symphonia test assets.
- GPL/LGPL tools may be used locally as black-box oracles, but their binaries,
  source, and generated CI dependencies must not be committed.
- `cargo-deny` enforces the permissive/MPL dependency policy; GPL/LGPL/AGPL are
  intentionally absent from the allow-list.
- Codec patent status can vary by jurisdiction and format. This project does
  not grant patent licenses beyond the Apache-2.0 license text; downstream users
  are responsible for their own patent due diligence.

## Development

MSRV is Rust `1.86.0`.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

The `xtask` crate drives packaging and the release gates:

| Command | Purpose |
|---|---|
| `release-check` | Local release gate: fmt, metadata, check, tests, clippy, ref-check, fuzz-smoke, WASM check, cargo-deny, npm dry run |
| `artifact-check` | Build/verify package artifacts that don't need a git `HEAD` (wasm-pack, npm pack, Python wheel) |
| `package-preflight` | Stricter publish-artifact gate (requires git `HEAD`, `wasm-pack`, `maturin`); also runs `qa-check` |
| `publish-preflight` | Mandatory first-publish gate: package preflight + registry-name check + `publish-readiness` |
| `publish-readiness` | Final blocker: MP3/AAC-LC production encode decoded by the local oracle (needs `SONARE_FFMPEG`) |
| `qa-check` | Optional QA tools when available (nextest, machete, audit, semver-checks, miri, llvm-cov) |
| `name-check` / `publish-plan` | Registry name availability / printed publish sequence |
| `gen-refs` / `ref-check` | Regenerate and byte-compare committed oracle reference artifacts |
| `oracle-smoke` / `aac-standard-diagnostic` / `mp3-perceptual-diagnostic` | Local-only FFmpeg oracle diagnostics (need `SONARE_FFMPEG`; never run in CI) |
| `tool-check` / `size-report` | Report available publish tools / artifact sizes |

Many commands honor environment overrides (`SONARE_FFMPEG`, `SONARE_WASM_PACK`,
`SONARE_PYTHON`, `SONARE_CARGO_DENY`, `SONARE_REQUIRED_QA_TOOLS`,
`SONARE_CHECK_REGISTRY_NAMES`, …). See `RELEASE.md` for the full package build,
name-availability, and publish-preflight walkthrough.
