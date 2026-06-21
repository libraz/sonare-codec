# sonare-codec

Python bindings for [`sonare-codec`](https://github.com/libraz/sonare-codec).

> ⚠️ **Alpha — under active development, not production-ready.** Pre-1.0, with an
> unstable API and not yet published to PyPI. Lossy encoders (MP3, AAC, Vorbis,
> Opus) are still maturing.

The wheel ships a `sonare_codec.pyi` stub for editor and type-checker support
through maturin's pure-Rust project typing.

## Usage

```python
import sonare_codec

# Decode any supported container to interleaved f32 PCM.
sample_rate, channels, samples = sonare_codec.decode_audio(data)

# Encode interleaved f32 PCM.
# format: "wav" | "flac" | "mp3" | "aac" | "m4a" | "vorbis" | "opus"
wav = sonare_codec.encode_audio("wav", sample_rate, channels, samples)
flac = sonare_codec.encode_audio("flac", sample_rate, channels, samples)
vorbis = sonare_codec.encode_vorbis(48000, 1, [0.0] * 4800)
opus = sonare_codec.encode_opus(48000, 1, [0.0] * 4800)

# encode_audio_production enforces the production-candidate guardrails for the
# in-progress MP3/AAC-LC paths (WAV/FLAC/Vorbis/Opus always encode).
mp3 = sonare_codec.encode_audio_production("mp3", 44100, 1, [0.0] * 1152)

kind = sonare_codec.detect_format(opus)  # "opus"

# Incremental decode: feed chunks until a full stream is buffered.
decoder = sonare_codec.StreamDecoder()
assert decoder.decode_stream(wav[:-2]) is None          # incomplete
sample_rate, channels, samples = decoder.decode_stream(wav[-2:])
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
`sonare_codec.pyi` stub for the full list.
