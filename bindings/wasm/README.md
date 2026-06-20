# @libraz/sonare-codec

Rust/WASM bindings for `sonare-codec`.

```ts
import init, {
  decode_audio,
  decode_m4a,
  decode_mp3,
  demux_m4a_as_aac_adts,
  detect_format,
  aac_lc_adts_max_frame_len_for_bitrate,
  aac_lc_default_production_bitrate_bps,
  aac_lc_pcm_step_candidates,
  aac_standard_id_pcm_step_candidates,
  aac_standard_id_selected_scale_factor_global_gain,
  aac_standard_id_selected_scale_factor_magnitude_bias,
  aac_standard_id_selected_scale_factor_parameters,
  aac_standard_id_selected_scale_factor_balanced_parameters,
  aac_standard_id_selected_scale_factor_balanced_gain_deltas,
  aac_standard_id_selected_scale_factor_balanced_magnitude_biases,
  aac_unsigned_pairs7_unit_magnitude_table,
  encode_audio,
  encode_audio_production,
  encode_aac_with_bitrate,
  encode_aac_with_selected_scale_factors_and_bitrate,
  encode_aac_with_standard_spectral_offsets_and_bitrate,
  encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate,
  encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
  aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate,
  aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate,
  aac_recommended_standard_selected_scale_factor_profile_with_bitrate,
  aac_balanced_standard_selected_scale_factor_profile_with_bitrate,
  aac_recommended_standard_id_payload_breakdown_with_bitrate,
  aac_balanced_standard_id_payload_breakdown_with_bitrate,
  aac_balanced_standard_id_quality_control_profile_with_bitrate,
  aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate,
  aac_selected_scale_factor_frame_details_with_bitrate,
  encode_m4a_with_bitrate,
  encode_m4a_with_selected_scale_factors_and_bitrate,
  encode_m4a_with_standard_spectral_offsets_and_bitrate,
  encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate,
  encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate,
  encode_mp3_with_bitrate,
  encode_mp3_cbr_with_bitrate,
  encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate,
  encode_mp3_perceptual_scale_factor_band_bias,
  encode_mp3_perceptual_quantized_band_gain,
  encode_mp3_perceptual_quantized_band_gain_global_gain_bias,
  mp3_layer3_main_data_capacity_bytes,
  mp3_pcm_step_candidates,
  mp3_production_pcm_step_candidates,
  mp3_first_frame_perceptual_candidate_profile_with_bitrate,
  mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate,
  mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate,
  mp3_first_frame_quality_guarded_candidate_profile_with_bitrate,
  mp3_perceptual_bit_allocation_with_bitrate,
  mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate,
  mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate,
  mp3_standard_big_value_table_selects,
  StreamDecoder,
} from "@libraz/sonare-codec";

await init();

const pcm = decode_audio(bytes);
const flac = encode_audio("flac", pcm.sample_rate, pcm.channels, pcm.samples());
const mp3 = encode_audio("mp3", 44100, 1, new Float32Array(1152));
const productionMp3 = encode_audio_production("mp3", 44100, 1, new Float32Array(1152));
const mp3_96k = encode_mp3_with_bitrate(44100, 1, new Float32Array(1152), 96, false, false);
const mp3Cbr128k = encode_mp3_cbr_with_bitrate(44100, 1, new Float32Array(1152 * 3), 128, false);
const mp3Pcm = decode_mp3(mp3);
const aac = encode_audio("aac", 44100, 1, new Float32Array(1024));
const aac_10k = encode_aac_with_bitrate(44100, 1, new Float32Array(2048), 10000);
const aacSelected10k = encode_aac_with_selected_scale_factors_and_bitrate(
  44100,
  1,
  new Float32Array(2048),
  10000
);
const aacStandard128k = encode_aac_with_standard_spectral_offsets_and_bitrate(
  44100,
  1,
  new Float32Array(2048),
  128000,
  128
);
const aacBalancedGainDeltas = aac_standard_id_selected_scale_factor_balanced_gain_deltas(1);
const aacBalancedBiases = aac_standard_id_selected_scale_factor_balanced_magnitude_biases(1);
const aacStandardSelected128k =
  encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000
  );
const aacStandardSelectedDetails =
  aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000
  );
const aacStandardSelectedProfile =
  aac_recommended_standard_selected_scale_factor_profile_with_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000
  );
const aacBalancedSelectedProfile =
  aac_balanced_standard_selected_scale_factor_profile_with_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000
  );
const aacStandardPayloadBreakdown =
  aac_recommended_standard_id_payload_breakdown_with_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000
  );
const aacBalancedPayloadBreakdown =
  aac_balanced_standard_id_payload_breakdown_with_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000
  );
const aacBalancedQualityProfile =
  aac_balanced_standard_id_quality_control_profile_with_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000
  );
const aacBalancedQualityCandidates =
  aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000
  );
const aacProductionDetails = aac_selected_scale_factor_frame_details_with_bitrate(
  44100,
  1,
  new Float32Array(2048),
  128000
);
const m4a = encode_audio("m4a", 44100, 1, new Float32Array(1024));
const m4a_10k = encode_m4a_with_bitrate(44100, 1, new Float32Array(2048), 10000);
const m4aSelected10k = encode_m4a_with_selected_scale_factors_and_bitrate(
  44100,
  1,
  new Float32Array(2048),
  10000
);
const m4aStandard128k = encode_m4a_with_standard_spectral_offsets_and_bitrate(
  44100,
  1,
  new Float32Array(2048),
  128000,
  128
);
const m4aStandardSelected128k =
  encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    44100,
    1,
    new Float32Array(2048),
    128000,
    128,
    16
  );
const kind = detect_format(m4a); // "m4a"
const adts = demux_m4a_as_aac_adts(m4a);
const m4aPcm = decode_m4a(m4a);
const aacBudget = aac_lc_adts_max_frame_len_for_bitrate(44100, 10000);
const aacProductionBitrate = aac_lc_default_production_bitrate_bps(1);
const aacProductionSteps = Array.from(aac_lc_pcm_step_candidates());
const aacStandardIdSteps = Array.from(aac_standard_id_pcm_step_candidates());
const aacCodebook7 = aac_unsigned_pairs7_unit_magnitude_table();
const mp3Capacity = mp3_layer3_main_data_capacity_bytes(44100, 1, 128, false, false);
const mp3Steps = Array.from(mp3_pcm_step_candidates());
const mp3MonoProductionSteps = Array.from(mp3_production_pcm_step_candidates(1));
const mp3CandidateProfile = Array.from(
  mp3_first_frame_perceptual_candidate_profile_with_bitrate(
    44100,
    1,
    new Float32Array(1152 * 3),
    128,
    false
  )
);
const mp3LowBandShapeProfile = Array.from(
  mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate(
    44100,
    1,
    new Float32Array(1152 * 3),
    128,
    false
  )
);
const mp3BandShapeProfile = Array.from(
  mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate(
    44100,
    1,
    new Float32Array(1152 * 3),
    128,
    false
  )
);
const bandBiasedMp3 = encode_mp3_perceptual_scale_factor_band_bias(
  44100,
  1,
  new Float32Array(1152),
  0.2,
  0,
  7,
  2
);
const bandGainMp3 = encode_mp3_perceptual_quantized_band_gain(
  44100,
  1,
  new Float32Array(1152),
  0.2,
  0,
  7,
  1.5
);
const bandGainMatchedMp3 =
  encode_mp3_perceptual_quantized_band_gain_global_gain_bias(
    44100,
    1,
    new Float32Array(1152),
    2.0,
    0,
    7,
    1.5,
    -4
  );
const mp3GuardedCandidateProfile = Array.from(
  mp3_first_frame_quality_guarded_candidate_profile_with_bitrate(
    44100,
    1,
    new Float32Array(1152 * 3),
    128,
    false
  )
);
const mp3BitAllocation = Array.from(
  mp3_perceptual_bit_allocation_with_bitrate(
    44100,
    1,
    new Float32Array(1152 * 3),
    128,
    false,
    0
  )
);
const mp3EntropyTargetedReservoirDetails = Array.from(
  mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(
    44100,
    1,
    new Float32Array(1152 * 3),
    128,
    false,
    0
  )
);
const mp3EntropyTargetedReservoirUtilization = Array.from(
  mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate(
    44100,
    1,
    new Float32Array(1152 * 3),
    128,
    false,
    0
  )
);
const mp3EntropyTargetedReservoir = encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(
  44100,
  1,
  new Float32Array(1152 * 3),
  128,
  false,
  0
);
const mp3Tables = Array.from(mp3_standard_big_value_table_selects());

const decoder = new StreamDecoder();
const partial = flac.slice(0, flac.length - 2);
console.assert(decoder.decode_stream(partial) === undefined);
const streamed = decoder.decode_stream(flac.slice(flac.length - 2));
```

Current WASM encoder support is WAV, FLAC, MP3 Layer III, and AAC-LC ADTS/M4A.
`encode_audio_production` accepts the current non-silent lossy production
candidates: mono/stereo MP3 at 32/44.1/48 kHz and mono/stereo AAC-LC ADTS/M4A
at 7.35/8/11.025/12/16/22.05/24/32/44.1/48/64/88.2/96 kHz. Other non-silent
MP3/AAC shapes are rejected, and Vorbis/Opus encode is still incomplete on the
WASM surface. The package also exposes small lossy diagnostics for AAC ADTS
bitrate budgets, AAC production and standard-id step candidates, AAC scale-factor/codebook 5/6 direct signed-pair tables,
codebook 1/2 direct signed-quad tables, codebook 3/4 unsigned-quad tables, codebook 7/8/9/10 unsigned-pair tables, the escape codebook 11 table, codebook 6 section planning, quad and mixed standard-id
section planning backed by core-owned unit fixtures, standard table-set
section planning that now uses direct signed quad codebook 1/2, unsigned-quad
codebook 3/4, direct signed pair codebook 5/6, and standard unsigned/escape codebooks, MP3 Layer III step candidates and main-data capacity,
standard MP3 Huffman selector lists, AAC
default production bitrate lookup, and caller-selected AAC/MP3 bitrate encoding. The AAC
standard-id payload breakdown helpers report `[frames, channels, sections,
escape_sections, max_abs, section_bits, scale_factor_bits, spectral_bits,
escape_spectral_bits, dominant_spectral_bits, dominant_escape_spectral_bits]`.
The MP3 bitrate
helpers include fixed-padding and CBR padding-scheduled variants, the
perceptual active CBR diagnostic encoder, and flattened reservoir frame
telemetry including frame length, padding, `main_data_begin`, and reservoir
state plus perceptual-vs-calibrated granule counts, quality-guard comparison
count, and encoder-side distortion delta for the MP3 reservoir diagnostics. The
perceptual reservoir helper exposes matching telemetry for the mono/stereo
production psychoacoustic scale-factor reservoir path; the entropy-targeted
helper also exposes utilization summary `[frames, used_frames, payload_bits,
entropy_budget_bits, utilization, max_slack_bits]`, and the quality-guarded
perceptual reservoir helper remains available as a comparison diagnostic. The mixed AAC helper
also reports section/spectral/scale-factor split payload bit lengths for the
current caller-table workbench, and the standard escape/mixed helpers report
section/spectral/packed bit lengths for codebook-11 and quad+escape diagnostic
paths. The standard mixed section and payload helpers also include
scale-factor-band-offset variants for the same workbench, and the standard
AAC-LC mono/stereo offsets ADTS helpers expose the same diagnostic stream
framing used by publish-readiness, including bitrate-derived step search
variants and flattened frame-selection telemetry.
The AAC/M4A bitrate helpers include fixed-scale-factor, internally selected
scale-factor, and standard-id selected-scale-factor plus magnitude-bias
variants. The standard-id selected-scale-factor frame-details helper returns
flattened `[frame_index, step, frame_len, frame_capacity_bytes, ...]`
telemetry for the same public AAC/M4A encode path.
