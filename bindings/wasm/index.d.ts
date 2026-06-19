export type EncodedFormat =
  | "wav"
  | "flac"
  | "mp3"
  | "vorbis"
  | "opus"
  | "aac"
  | "m4a"
  | "mp4";

export class WavPcm {
  readonly sample_rate: number;
  readonly channels: number;
  samples(): Float32Array;
}

export class StreamDecoder {
  constructor();
  decode_stream(input: Uint8Array): WavPcm | undefined;
  reset(): void;
  buffered_len(): number;
}

export function detect_format(input: Uint8Array): string | undefined;

export function decode_audio(input: Uint8Array): WavPcm;

export function decode_wav(input: Uint8Array): WavPcm;

export function decode_flac(input: Uint8Array): WavPcm;

export function decode_mp3(input: Uint8Array): WavPcm;

export function decode_vorbis(input: Uint8Array): WavPcm;

export function decode_opus(input: Uint8Array): WavPcm;

export function decode_aac(input: Uint8Array): WavPcm;

export function decode_m4a(input: Uint8Array): WavPcm;

export function encode_audio(
  format: EncodedFormat,
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_audio_production(
  format: EncodedFormat,
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_wav(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_flac(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_mp3(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_mp3_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  padding: boolean,
  crc_protected: boolean
): Uint8Array;

export function encode_mp3_cbr_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean
): Uint8Array;

export function encode_mp3_perceptual_active_cbr_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean
): Uint8Array;

export function encode_mp3_perceptual_reservoir_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean
): Uint8Array;

export function encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean,
  min_bits_per_granule_channel: number
): Uint8Array;

export function encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean
): Uint8Array;

/**
 * Returns flattened per-frame reservoir telemetry:
 * [frame_index, step, payload_bit_len, frame_len, padding, frame_capacity_bytes, main_data_begin, reservoir_after, perceptual_granules, calibrated_granules, quality_guard_compared_granules, quality_guard_distortion_delta]...
 */
export function mp3_reservoir_frame_details_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean
): Float64Array;

/**
 * Returns flattened per-frame perceptual reservoir telemetry:
 * [frame_index, step, payload_bit_len, frame_len, padding, frame_capacity_bytes, main_data_begin, reservoir_after, perceptual_granules, calibrated_granules, quality_guard_compared_granules, quality_guard_distortion_delta]...
 */
export function mp3_perceptual_reservoir_frame_details_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean
): Float64Array;

/**
 * Returns flattened per-frame entropy-targeted perceptual reservoir telemetry:
 * [frame_index, step, payload_bit_len, frame_len, padding, frame_capacity_bytes, main_data_begin, reservoir_after, perceptual_granules, calibrated_granules, quality_guard_compared_granules, quality_guard_distortion_delta, entropy_target_bits, used_entropy_target_budget]...
 */
export function mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean,
  min_bits_per_granule_channel: number
): Float64Array;

/**
 * Returns flattened per-frame quality-guarded perceptual reservoir telemetry:
 * [frame_index, step, payload_bit_len, frame_len, padding, frame_capacity_bytes, main_data_begin, reservoir_after, perceptual_granules, calibrated_granules, quality_guard_compared_granules, quality_guard_distortion_delta]...
 */
export function mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean
): Float64Array;

export function encode_aac(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_aac_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number
): Uint8Array;

export function encode_aac_with_selected_scale_factors_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number
): Uint8Array;

export function encode_aac_with_standard_spectral_offsets_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number
): Uint8Array;

export function encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number,
  scale_factor_magnitude_bias: number
): Uint8Array;

export function encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number
): Uint8Array;

export function aac_standard_id_selected_scale_factor_global_gain(
  channels: number
): number;

export function aac_standard_id_selected_scale_factor_magnitude_bias(): number;

export function aac_standard_id_selected_scale_factor_parameters(
  channels: number
): Float64Array;

export function encode_m4a(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_m4a_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number
): Uint8Array;

export function encode_m4a_with_selected_scale_factors_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number
): Uint8Array;

export function encode_m4a_with_standard_spectral_offsets_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number
): Uint8Array;

export function encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number,
  scale_factor_magnitude_bias: number
): Uint8Array;

export function encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number
): Uint8Array;

export function demux_m4a_as_aac_adts(input: Uint8Array): Uint8Array;

export function aac_lc_adts_max_frame_len_for_bitrate(
  sample_rate: number,
  target_bitrate_bps: number
): number;

export function aac_lc_default_production_bitrate_bps(
  channels: number
): number;

export function aac_lc_pcm_step_candidates(): Float32Array;

export function aac_standard_id_pcm_step_candidates(): Float32Array;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_unsigned_pairs7_unit_magnitude_table(): Uint32Array;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_unsigned_pairs7_table(): Uint32Array;

/**
 * Returns flattened signed entries as [x, y, bits, len, ...].
 */
export function aac_signed_pairs5_table(): Int32Array;

/**
 * Returns flattened signed entries as [x, y, bits, len, ...].
 */
export function aac_signed_pairs6_table(): Int32Array;

/**
 * Returns flattened signed entries as [v, w, x, y, bits, len, ...].
 */
export function aac_signed_quads1_table(): Int32Array;

/**
 * Returns flattened signed entries as [v, w, x, y, bits, len, ...].
 */
export function aac_signed_quads2_table(): Int32Array;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_unsigned_pairs8_table(): Uint32Array;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_unsigned_pairs9_table(): Uint32Array;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_unsigned_pairs10_table(): Uint32Array;

/**
 * Returns flattened entries as [v, w, x, y, bits, len, ...].
 */
export function aac_unsigned_quads3_table(): Uint32Array;

/**
 * Returns flattened entries as [v, w, x, y, bits, len, ...].
 */
export function aac_unsigned_quads4_table(): Uint32Array;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_escape_table(): Uint32Array;

/**
 * Returns flattened entries as [delta, bits, len, ...].
 */
export function aac_scale_factor_delta_table(): Int32Array;

/**
 * Returns flattened sections as [start, end, codebook_id, ...].
 */
export function aac_codebook6_unit_section_plan(
  quantized: Int32Array,
  band_width: number,
): Uint32Array;

/**
 * Returns flattened quad sections as [start, end, codebook_id, ...].
 */
export function aac_quad_unit_section_plan(
  quantized: Int32Array,
  band_width: number,
): Uint32Array;

/**
 * Returns flattened mixed standard codebook-id sections as [start, end, codebook_id, ...].
 */
export function aac_mixed_unit_section_plan(
  quantized: Int32Array,
  band_width: number,
): Uint32Array;

/**
 * Returns mixed payload bit lengths as
 * [section_bits, spectral_bits, packed_bits, section_scale_bits, spectral_bits, packed_scale_bits].
 */
export function aac_mixed_unit_payload_bit_lengths(
  quantized: Int32Array,
  band_width: number,
): Uint32Array;

/**
 * Returns flattened standard table-set sections as [start, end, codebook_id, ...].
 */
export function aac_standard_unit_section_plan(
  quantized: Int32Array,
  band_width: number,
): Uint32Array;

/**
 * Returns flattened standard table-set sections using scale-factor band offsets as [start, end, codebook_id, ...].
 */
export function aac_standard_offsets_section_plan(
  quantized: Int32Array,
  offsets: Uint32Array,
): Uint32Array;

/**
 * Returns [section_bits, spectral_bits, packed_bits] for a standard codebook-11 escape fixture.
 */
export function aac_standard_escape_payload_bit_lengths(): Uint32Array;

/**
 * Returns mixed standard-id payload bit lengths as
 * [section_bits, spectral_bits, packed_bits, section_scale_bits, spectral_bits, packed_scale_bits].
 */
export function aac_standard_mixed_payload_bit_lengths(
  quantized: Int32Array,
  band_width: number,
): Uint32Array;

/**
 * Returns mixed standard-id payload bit lengths using scale-factor band offsets as
 * [section_bits, spectral_bits, packed_bits, section_scale_bits, spectral_bits, packed_scale_bits].
 */
export function aac_standard_mixed_offsets_payload_bit_lengths(
  quantized: Int32Array,
  offsets: Uint32Array,
): Uint32Array;

/**
 * Encodes a mono AAC-LC ADTS diagnostic stream with standard-id spectral sections.
 */
export function encode_aac_standard_mono_offsets_with_step(
  sample_rate: number,
  samples: Float32Array,
  step: number,
  global_gain: number,
): Uint8Array;

/**
 * Encodes a mono AAC-LC ADTS diagnostic stream with standard-id spectral sections and bitrate-derived step search.
 */
export function encode_aac_standard_mono_offsets_with_bitrate(
  sample_rate: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number,
): Uint8Array;

/**
 * Returns flattened mono standard-id offsets bitrate selections as [frame_index, step, frame_len, frame_capacity_bytes, ...].
 */
export function aac_standard_mono_offsets_bitrate_frame_details(
  sample_rate: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number,
): Float64Array;

/**
 * Encodes a stereo AAC-LC ADTS diagnostic stream with standard-id spectral sections.
 */
export function encode_aac_standard_stereo_offsets_with_step(
  sample_rate: number,
  samples: Float32Array,
  step: number,
  global_gain: number,
): Uint8Array;

/**
 * Encodes a stereo AAC-LC ADTS diagnostic stream with standard-id spectral sections and bitrate-derived step search.
 */
export function encode_aac_standard_stereo_offsets_with_bitrate(
  sample_rate: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number,
): Uint8Array;

/**
 * Returns flattened stereo standard-id offsets bitrate selections as [frame_index, step, frame_len, frame_capacity_bytes, ...].
 */
export function aac_standard_stereo_offsets_bitrate_frame_details(
  sample_rate: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number,
): Float64Array;

/**
 * Returns flattened standard-id selected-scale-factor bitrate selections as [frame_index, step, frame_len, frame_capacity_bytes, ...].
 */
export function aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number,
  global_gain: number,
  scale_factor_magnitude_bias: number,
): Float64Array;

/**
 * Returns flattened recommended standard-id selected-scale-factor bitrate selections as [frame_index, step, frame_len, frame_capacity_bytes, ...].
 */
export function aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number,
): Float64Array;

/**
 * Returns flattened production selected-scale-factor bitrate selections as [frame_index, step, frame_len, frame_capacity_bytes, ...].
 */
export function aac_selected_scale_factor_frame_details_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number,
): Float64Array;

export function mp3_layer3_main_data_capacity_bytes(
  sample_rate: number,
  channels: number,
  bitrate_kbps: number,
  padding: boolean,
  crc_protected: boolean
): number;

export function mp3_layer3_main_data_capacity_bits(
  sample_rate: number,
  channels: number,
  bitrate_kbps: number,
  padding: boolean,
  crc_protected: boolean
): number;

export function mp3_pcm_step_candidates(): Float32Array;

export function mp3_first_frame_perceptual_candidate_profile_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean
): Float64Array;

export function mp3_perceptual_bit_allocation_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  crc_protected: boolean,
  min_bits_per_granule_channel: number
): Float64Array;

export function mp3_standard_big_value_table_selects(): Uint8Array;

export function mp3_missing_standard_big_value_table_selects(): Uint8Array;

export function mp3_standard_count1_table_selects(): Uint8Array;
