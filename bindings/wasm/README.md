# @libraz/sonare-codec

Rust/WASM bindings for [`sonare-codec`](https://github.com/libraz/sonare-codec).

> ⚠️ **Alpha — under active development, not production-ready.** Pre-1.0, with an
> unstable API and not yet published to npm. Lossy encoders (MP3, AAC, Vorbis,
> Opus) are still maturing.

## Usage

```ts
import init, {
  detect_format,
  decode_audio,
  encode_audio,
  encode_audio_production,
  encode_vorbis,
  encode_opus,
  decode_vorbis,
  decode_opus,
  StreamDecoder,
} from "@libraz/sonare-codec";

await init();

// Decode any supported container to interleaved f32 PCM.
const pcm = decode_audio(bytes); // { sample_rate, channels, samples() }

// Encode interleaved f32 PCM. `format`: wav | flac | mp3 | aac | m4a | vorbis | opus
const flac = encode_audio("flac", pcm.sample_rate, pcm.channels, pcm.samples());
const vorbis = encode_vorbis(48000, 1, new Float32Array(4800));
const opus = encode_opus(48000, 1, new Float32Array(4800));

// `encode_audio_production` enforces the production-candidate guardrails for the
// in-progress MP3/AAC-LC paths (WAV/FLAC/Vorbis/Opus always encode).
const mp3 = encode_audio_production("mp3", 44100, 1, new Float32Array(1152));

// Incremental decode: feed chunks until a full stream is buffered.
const decoder = new StreamDecoder();
decoder.decode_stream(flac.slice(0, -2)); // => undefined (incomplete)
const out = decoder.decode_stream(flac.slice(-2)); // => PCM
```

## Encoder support

WAV and FLAC encode real audio losslessly. Vorbis and Opus encode through
first-party pure-Rust encoders (mono/stereo; Opus is 48 kHz CELT-only). MP3
(MPEG-1 32/44.1/48 kHz, MPEG-2 LSF 16/22.05/24 kHz) and AAC-LC ADTS/M4A emit
non-silent production candidates that are still maturing; `encode_audio_production`
rejects MP3/AAC input at unsupported sample rates or channel counts.

The package additionally exposes a large set of low-level **diagnostic helpers**
for the in-progress AAC and MP3 encoders (bitrate frame budgets, step
candidates, codebook/section planning tables, scale-factor and reservoir
telemetry, and `encode_*_with_bitrate` variants). These mirror the Rust
workbench API and are not needed for normal encode/decode use — see the
`.d.ts` type definitions for the full list.
