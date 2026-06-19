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
mp3_cbr_128k = sonare_codec.encode_mp3_cbr_with_bitrate(
    44100, 1, [0.0] * (1152 * 3), 128, False
)
mp3_sample_rate, mp3_channels, mp3_samples = sonare_codec.decode_mp3(mp3)
vorbis = sonare_codec.encode_vorbis(48000, 1, [0.0] * 4800)
production_vorbis = sonare_codec.encode_audio_production("vorbis", 48000, 1, [0.0] * 4800)
vorbis_sample_rate, vorbis_channels, vorbis_samples = sonare_codec.decode_vorbis(vorbis)
opus = sonare_codec.encode_opus(48000, 1, [0.0] * 4800)
production_opus = sonare_codec.encode_audio_production("opus", 48000, 1, [0.0] * 4800)
opus_sample_rate, opus_channels, opus_samples = sonare_codec.decode_opus(opus)
aac = sonare_codec.encode_audio("aac", 44100, 1, [0.0] * 1024)
aac_10k = sonare_codec.encode_aac_with_bitrate(44100, 1, [0.0] * 2048, 10000)
aac_selected_10k = sonare_codec.encode_aac_with_selected_scale_factors_and_bitrate(
    44100, 1, [0.0] * 2048, 10000
)
aac_standard_128k = sonare_codec.encode_aac_with_standard_spectral_offsets_and_bitrate(
    44100, 1, [0.0] * 2048, 128000, 128
)
aac_standard_selected_params = sonare_codec.aac_standard_id_selected_scale_factor_parameters(1)
aac_standard_selected_128k = (
    sonare_codec.encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
        44100,
        1,
        [0.0] * 2048,
        128000,
    )
)
aac_standard_selected_details = (
    sonare_codec.aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
        44100,
        1,
        [0.0] * 2048,
        128000,
    )
)
aac_production_details = sonare_codec.aac_selected_scale_factor_frame_details_with_bitrate(
    44100, 1, [0.0] * 2048, 128000
)
m4a = sonare_codec.encode_audio("m4a", 44100, 1, [0.0] * 1024)
m4a_10k = sonare_codec.encode_m4a_with_bitrate(44100, 1, [0.0] * 2048, 10000)
m4a_selected_10k = sonare_codec.encode_m4a_with_selected_scale_factors_and_bitrate(
    44100, 1, [0.0] * 2048, 10000
)
m4a_standard_128k = sonare_codec.encode_m4a_with_standard_spectral_offsets_and_bitrate(
    44100, 1, [0.0] * 2048, 128000, 128
)
m4a_standard_selected_128k = (
    sonare_codec.encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        44100, 1, [0.0] * 2048, 128000, 128, 16
    )
)
kind = sonare_codec.detect_format(m4a)  # "m4a"
adts = sonare_codec.demux_m4a_as_aac_adts(m4a)
m4a_sample_rate, m4a_channels, m4a_samples = sonare_codec.decode_m4a(m4a)
aac_budget = sonare_codec.aac_lc_adts_max_frame_len_for_bitrate(44100, 10000)
aac_production_bitrate = sonare_codec.aac_lc_default_production_bitrate_bps(1)
aac_production_steps = sonare_codec.aac_lc_pcm_step_candidates()
aac_standard_id_steps = sonare_codec.aac_standard_id_pcm_step_candidates()
aac_codebook7 = sonare_codec.aac_unsigned_pairs7_unit_magnitude_table()
mp3_capacity = sonare_codec.mp3_layer3_main_data_capacity_bytes(44100, 1, 128, False, False)
mp3_steps = sonare_codec.mp3_pcm_step_candidates()
mp3_candidate_profile = sonare_codec.mp3_first_frame_perceptual_candidate_profile_with_bitrate(
    44100, 1, [0.0] * (1152 * 3), 128, False
)
mp3_bit_allocation = sonare_codec.mp3_perceptual_bit_allocation_with_bitrate(
    44100, 1, [0.0] * (1152 * 3), 128, False, 0
)
mp3_entropy_targeted_reservoir_details = (
    sonare_codec.mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(
        44100, 1, [0.0] * (1152 * 3), 128, False, 0
    )
)
mp3_entropy_targeted_reservoir = (
    sonare_codec.encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(
        44100, 1, [0.0] * (1152 * 3), 128, False, 0
    )
)
mp3_tables = sonare_codec.mp3_standard_big_value_table_selects()

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
lossy diagnostics for AAC ADTS bitrate budgets, AAC production and standard-id
step candidates, AAC scale-factor/codebook
5/6 direct signed-pair tables, codebook 1/2 direct signed-quad tables,
codebook 3/4 unsigned-quad tables,
codebook 7/8/9/10 unsigned-pair tables, the
escape codebook 11 table, codebook 6 section planning,
quad and mixed standard-id section planning backed by core-owned unit fixtures,
standard table-set section planning that now uses direct signed codebook 5/6
alongside direct signed quad codebook 1/2, unsigned-quad codebook 3/4, and standard unsigned/escape codebooks, MP3 Layer III
step candidates and main-data capacity, standard MP3 Huffman selector lists, AAC default production bitrate lookup, and caller-selected
AAC/MP3 bitrate encoding. The mixed AAC helper also reports
section/spectral/scale-factor split payload bit lengths for the current
caller-table workbench, and the standard escape/mixed helpers report
section/spectral/packed bit lengths for codebook-11 and quad+escape diagnostic
paths. The standard mixed section and payload helpers also include
scale-factor-band-offset variants for the same workbench, and the standard
AAC-LC mono/stereo offsets ADTS helpers expose the same diagnostic stream
framing used by publish-readiness, including bitrate-derived step search
variants and flattened frame-selection telemetry. The MP3 bitrate helpers include
fixed-padding and CBR padding-scheduled variants, the
perceptual active CBR diagnostic encoder, and flattened reservoir frame
telemetry including frame length, padding, `main_data_begin`, and reservoir
state plus perceptual-vs-calibrated granule counts, quality-guard comparison
count, and encoder-side distortion delta for the MP3 reservoir diagnostics. The
perceptual reservoir helper exposes matching telemetry for the mono/stereo
production psychoacoustic scale-factor reservoir path, and the quality-guarded
perceptual reservoir helper remains available as a comparison diagnostic.
The AAC/M4A bitrate helpers include fixed-scale-factor, internally selected
scale-factor, and standard-id selected-scale-factor plus magnitude-bias
variants. The standard-id selected-scale-factor frame-details helper returns
flattened `[frame_index, step, frame_len, frame_capacity_bytes, ...]`
telemetry for the same public AAC/M4A encode path.
