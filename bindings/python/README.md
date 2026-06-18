# sonare-codec

Python bindings for `sonare-codec`.

The wheel includes a `sonare_codec.pyi` stub for editor and type-checker
visibility through maturin's pure Rust project typing support.

```python
import sonare_codec

sample_rate, channels, samples = sonare_codec.decode_audio(data)
wav = sonare_codec.encode_audio("wav", sample_rate, channels, samples)
flac = sonare_codec.encode_audio("flac", sample_rate, channels, samples)
mp3 = sonare_codec.encode_audio("mp3", 44100, 1, [0.0] * 1152)
production_mp3 = sonare_codec.encode_audio_production("mp3", 44100, 1, [0.0] * 1152)
mp3_96k = sonare_codec.encode_mp3_with_bitrate(44100, 1, [0.0] * 1152, 96, False, False)
mp3_sample_rate, mp3_channels, mp3_samples = sonare_codec.decode_mp3(mp3)
vorbis = sonare_codec.encode_vorbis(48000, 1, [0.0] * 4800)
production_vorbis = sonare_codec.encode_audio_production("vorbis", 48000, 1, [0.0] * 4800)
vorbis_sample_rate, vorbis_channels, vorbis_samples = sonare_codec.decode_vorbis(vorbis)
opus = sonare_codec.encode_opus(48000, 1, [0.0] * 4800)
production_opus = sonare_codec.encode_audio_production("opus", 48000, 1, [0.0] * 4800)
opus_sample_rate, opus_channels, opus_samples = sonare_codec.decode_opus(opus)
aac = sonare_codec.encode_audio("aac", 44100, 1, [0.0] * 1024)
aac_10k = sonare_codec.encode_aac_with_bitrate(44100, 1, [0.0] * 2048, 10000)
m4a = sonare_codec.encode_audio("m4a", 44100, 1, [0.0] * 1024)
m4a_10k = sonare_codec.encode_m4a_with_bitrate(44100, 1, [0.0] * 2048, 10000)
kind = sonare_codec.detect_format(m4a)  # "m4a"
adts = sonare_codec.demux_m4a_as_aac_adts(m4a)
m4a_sample_rate, m4a_channels, m4a_samples = sonare_codec.decode_m4a(m4a)
aac_budget = sonare_codec.aac_lc_adts_max_frame_len_for_bitrate(44100, 10000)
aac_codebook7 = sonare_codec.aac_unsigned_pairs7_unit_magnitude_table()
mp3_capacity = sonare_codec.mp3_layer3_main_data_capacity_bytes(44100, 1, 128, False, False)

decoder = sonare_codec.StreamDecoder()
partial = wav[: len(wav) - 2]
assert decoder.decode_stream(partial) is None
streamed = decoder.decode_stream(wav[len(wav) - 2 :])
assert streamed is not None
streamed_sample_rate, streamed_channels, streamed_samples = streamed
```

Current encoder support is WAV, FLAC, Ogg Vorbis, Ogg Opus, MP3 Layer III, and
AAC-LC ADTS/M4A. `encode_audio_production` accepts the current non-silent lossy
production candidates: mono/stereo MP3 at 32/44.1/48 kHz and mono/stereo
AAC-LC ADTS/M4A at 7.35/8/11.025/12/16/22.05/24/32/44.1/48/64/88.2/96 kHz.
Other non-silent MP3/AAC shapes are rejected. The package also exposes small
lossy diagnostics for AAC ADTS bitrate budgets, AAC scale-factor/codebook 7/8
tables, MP3 Layer III main-data capacity, and caller-selected AAC/MP3 bitrate
encoding.
