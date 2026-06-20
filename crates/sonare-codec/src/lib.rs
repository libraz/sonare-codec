#![deny(unsafe_code)]
#![warn(clippy::all)]

pub use sc_core::{
    compare_pcm, compare_pcm_with_tolerance, detect, AudioBuffer, Decoder, Encoder, Error, Format,
    HuffmanCode, HuffmanEntry, PackedBits, PcmDiff, PcmTolerance,
};

#[cfg(feature = "mp3")]
pub use sc_mp3::{
    apply_big_value_region_tables_to_granule, assemble_layer3_frame_from_payloads,
    assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider,
    assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors,
    assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_quantized_band_gain_and_global_gain_bias_reservoir_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_auto_step_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_max_payload_bits_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scale_factors_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors,
    encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_allowed_noise_scale_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_auto_step_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_cbr_bitrate_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_max_payload_bits_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_global_gain_bias_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_quantized_band_gain_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factor_band_bias_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_perceptual_scalefac_scale_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_quality_guarded_perceptual_reservoir_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider,
    encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors,
    encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider,
    experimental_unit_magnitude_table_provider, layer3_header_for_capacity,
    layer3_main_data_capacity_bits, layer3_main_data_capacity_bytes, mdct_long_block,
    mpeg1_layer3_entropy_target_utilization_profile, mpeg1_layer3_global_gain_for_step,
    mpeg1_layer3_production_pcm_step_candidates, mpeg1_layer3_standard_big_value_table_provider,
    mpeg1_layer3_standard_table_provider, pack_big_value_pairs_with_region_tables_and_provider,
    pack_layer3_main_data_payloads, pack_mpeg1_layer3_long_quantized_spectrum_for_granule,
    pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider,
    pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule,
    pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider,
    pack_mpeg1_layer3_long_scale_factors, pack_mpeg1_layer3_long_scale_factors_for_granule,
    pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider,
    pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule,
    select_big_value_region_tables, select_big_value_region_tables_by_bit_cost,
    select_big_value_table_by_bit_cost, select_count1_table_by_bit_cost,
    select_mpeg1_layer3_entropy_target_utilization_profile_with_table_provider,
    select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider,
    select_mpeg1_layer3_first_frame_band_spectral_shape_candidate_profile_with_table_provider,
    select_mpeg1_layer3_first_frame_low_band_spectral_shape_candidate_profile_with_table_provider,
    select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider,
    select_mpeg1_layer3_first_frame_quality_guarded_candidate_profile_with_table_provider,
    select_mpeg1_layer3_long_scale_factor_compress,
    select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum,
    select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_active_step_with_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_max_payload_bits_and_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_step_with_max_payload_bits_and_table_provider,
    select_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider,
    select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider,
    select_mpeg1_layer3_pcm_frame_step_details_with_table_provider,
    select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider,
    select_mpeg1_layer3_pcm_frame_step_with_table_provider,
    select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate,
    select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider,
    select_mpeg1_layer3_psychoacoustic_long_scale_factors,
    select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider,
    select_mpeg1_layer3_reservoir_frame_details_with_table_provider, ChannelMode, FrameHeader,
    Layer, Layer3BandSpectralShapeCandidateProfile, Layer3BigValueMagnitude, Layer3BigValuePair,
    Layer3BigValueRegionTableSelection, Layer3BigValueTableSelection, Layer3Count1MagnitudeQuad,
    Layer3Count1Quad, Layer3Count1TableSelection, Layer3EntropyTableProvider, Layer3EntropyTables,
    Layer3EntropyTargetUtilizationProfile, Layer3EntropyTargetedReservoirFrameSelection,
    Layer3GranuleChannelInfo, Layer3LowBandSpectralShapeCandidateProfile,
    Layer3PcmFrameStepSelection, Layer3PerceptualBitAllocation, Layer3PerceptualCandidateProfile,
    Layer3QualityGuardedCandidateProfile, Layer3QuantizedBandGain, Layer3ReservoirFrameSelection,
    Layer3ScaleFactorBandBias, Layer3ScaleFactorCompress, Layer3SideInfo, Layer3SpectralRegions,
    MpegVersion, MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT,
    MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS,
    MPEG1_LAYER3_MONO_PRODUCTION_PCM_STEP_CANDIDATES, MPEG1_LAYER3_PCM_STEP_CANDIDATES,
    MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS, MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS,
};

#[cfg(feature = "aac")]
pub use sc_aac::{
    aac_escape_table, aac_lc_adts_max_frame_len_for_bitrate, aac_lc_default_production_bitrate_bps,
    aac_lc_long_window_scale_factor_band_offsets, aac_lc_standard_signed_pair_tables,
    aac_lc_standard_signed_quad_tables, aac_lc_standard_spectral_tables,
    aac_lc_standard_unsigned_quad_tables, aac_scale_factor_delta_table,
    aac_scale_factor_delta_zero_table, aac_signed_pairs5_table, aac_signed_pairs6_table,
    aac_signed_quads1_table, aac_signed_quads2_table, aac_unit_codebook6_spectral_tables,
    aac_unit_quad_spectral_tables, aac_unsigned_pairs10_table, aac_unsigned_pairs7_table,
    aac_unsigned_pairs7_unit_magnitude_spectral_tables, aac_unsigned_pairs7_unit_magnitude_table,
    aac_unsigned_pairs8_table, aac_unsigned_pairs9_table, aac_unsigned_quads3_table,
    aac_unsigned_quads4_table, encode_pcm_mono_long_block_adts_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_auto_step_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_auto_step_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_scale_factors,
    encode_pcm_mono_long_block_adts_stream_with_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors,
    encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost,
    encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost,
    encode_pcm_mono_long_block_adts_with_scale_factors,
    encode_pcm_mono_long_block_adts_with_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_with_selected_scale_factors,
    encode_pcm_mono_long_block_adts_with_selected_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_mono_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost,
    encode_pcm_stereo_long_block_adts_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_auto_step_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_scale_factors,
    encode_pcm_stereo_long_block_adts_stream_with_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors,
    encode_pcm_stereo_long_block_adts_stream_with_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost,
    encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_scale_factors,
    encode_pcm_stereo_long_block_adts_with_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_selected_scale_factors,
    encode_pcm_stereo_long_block_adts_with_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost,
    encode_pcm_stereo_long_block_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost,
    encode_quantized_mono_adts, encode_quantized_mono_adts_by_bit_cost,
    encode_quantized_mono_adts_with_scale_factors,
    encode_quantized_mono_adts_with_scale_factors_by_bit_cost,
    encode_quantized_mono_adts_with_selected_scale_factors,
    encode_quantized_mono_adts_with_selected_scale_factors_by_bit_cost,
    encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
    encode_quantized_mono_adts_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost,
    encode_quantized_mono_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost,
    encode_quantized_stereo_adts, encode_quantized_stereo_adts_by_bit_cost,
    encode_quantized_stereo_adts_with_scale_factors,
    encode_quantized_stereo_adts_with_scale_factors_by_bit_cost,
    encode_quantized_stereo_adts_with_selected_scale_factors,
    encode_quantized_stereo_adts_with_selected_scale_factors_by_bit_cost,
    encode_quantized_stereo_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost,
    encode_quantized_stereo_adts_with_standard_spectral_offsets_and_selected_scale_factors_by_bit_cost,
    encode_quantized_stereo_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_by_bit_cost,
    experimental_aac_scale_factor_delta_table, experimental_unit_magnitude_spectral_tables,
    pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost,
    pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost,
    pack_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
    pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost,
    pack_quad_section_data_with_len, pack_scale_factor_deltas_with_table,
    pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits,
    pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits,
    pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost,
    pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost,
    pack_sectioned_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
    pack_sectioned_spectral_payload_with_sign_bits_by_bit_cost,
    pack_sectioned_spectral_quad_payload_with_sign_bits,
    pack_sectioned_spectral_quad_payload_with_sign_bits_by_bit_cost,
    pack_spectral_pairs_with_sign_bits, pack_spectral_pairs_with_table,
    pack_spectral_quad_sections_with_sign_bits, pack_spectral_quads_with_sign_bits,
    pack_spectral_quads_with_table, pack_spectral_section_data_with_len,
    pack_spectral_section_data_with_offsets, pack_spectral_sections_by_codebook_id_with_sign_bits,
    plan_aac_lc_standard_spectral_sections_by_bit_cost,
    plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost, plan_quad_sections_by_bit_cost,
    plan_sections_by_bit_cost, plan_sections_by_offsets,
    plan_spectral_scale_factor_deltas_by_offsets, plan_spectral_sections_by_bit_cost,
    plan_spectral_sections_by_offsets_by_bit_cost, quantize_pcm_long_block,
    select_aac_lc_mono_pcm_frame_step_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_offsets_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost,
    select_aac_lc_mono_pcm_frame_step_with_offsets_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost,
    select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_offsets_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_offsets_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_offsets_and_scale_factors_by_bit_cost,
    select_aac_lc_stereo_pcm_frame_step_with_offsets_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost,
    select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_max_frame_len_by_bit_cost,
    select_codebook_by_bit_cost, select_quad_codebook_by_bit_cost,
    select_scale_factors_for_quantized_bands, select_scale_factors_for_quantized_bands_by_offsets,
    select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias,
    select_spectral_codebook_id_by_bit_cost,
    split_aac_lc_standard_sectioned_spectral_payload_with_offsets_and_sign_bits,
    split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost,
    split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost,
    split_aac_lc_standard_spectral_payload_with_sign_bits_and_scale_factor_bits_by_bit_cost,
    split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost,
    split_sectioned_spectral_payload_by_codebook_id_with_sign_bits,
    split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits,
    split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost,
    split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost, AacCodebook,
    AacLongBlockConfig, AacPcmFrameStepSelection, AacPcmLongBlockConfig, AacPcmStepSearchConfig,
    AacProfile, AacQuadSection, AacQuantizedChannel, AacQuantizedSpectrum, AacScaleFactorChannel,
    AacScaleFactorDelta, AacScaleFactorSequence, AacSection, AacSpectralMagnitudePair,
    AacSpectralMagnitudeQuad, AacSpectralMagnitudeQuadTables, AacSpectralMagnitudeTables,
    AacSpectralPair, AacSpectralQuad, AacSpectralSection, AacSpectralTables, AdtsConfig,
    AAC_LC_48K_LONG_WINDOW_SCALE_FACTOR_BAND_OFFSETS, AAC_LC_PCM_STEP_CANDIDATES,
    AAC_SCALE_FACTOR_DELTA_ZERO_TABLE, AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
};

#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN: u8 = 128;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_GLOBAL_GAIN: u8 = 126;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS: i16 = 16;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN: u8 = 136;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN: u8 = 138;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_BALANCED_MAGNITUDE_BIAS: i16 = 8;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIAS: i16 = 8;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS: i16 = 4;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS: u32 = 2047;
#[cfg(feature = "aac")]
pub const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS: u32 = 1535;

#[cfg(feature = "aac")]
const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GAIN_DELTAS: &[u8] = &[0, 2, 4, 6, 8];
#[cfg(feature = "aac")]
const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GAIN_DELTAS: &[u8] = &[8, 12, 16];
#[cfg(feature = "aac")]
const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIASES: &[i16] =
    &[8, 12, 16, 20];
#[cfg(feature = "aac")]
const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIASES: &[i16] = &[4, 8, 12];

#[cfg(feature = "aac")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AacStandardIdSelectedScaleFactorBalanceProfile {
    pub recommended_global_gain: u8,
    pub global_gain_deltas: &'static [u8],
    pub magnitude_biases: &'static [i16],
    pub selected_global_gain: u8,
    pub selected_magnitude_bias: i16,
    pub max_quantized_abs: u32,
}

#[cfg(feature = "aac")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AacSelectedScaleFactorProfile {
    pub frames: usize,
    pub channels: usize,
    pub bands: usize,
    pub raised_bands: usize,
    pub max_delta: i16,
    pub mean_delta: f64,
}

#[cfg(feature = "aac")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AacStandardIdSpectralSectionBreakdown {
    pub frame_index: usize,
    pub channel: usize,
    pub start_band: usize,
    pub end_band: usize,
    pub start: usize,
    pub end: usize,
    pub codebook_id: u8,
    pub max_abs: i32,
    pub spectral_bits: usize,
    pub best_alternative_codebook_id: Option<u8>,
    pub best_alternative_spectral_bits: Option<usize>,
}

#[cfg(feature = "aac")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AacStandardIdPayloadBreakdown {
    pub frames: usize,
    pub channels: usize,
    pub sections: usize,
    pub escape_sections: usize,
    pub max_abs: i32,
    pub section_bits: usize,
    pub scale_factor_bits: usize,
    pub spectral_bits: usize,
    pub escape_spectral_bits: usize,
    pub dominant_spectral_section: Option<AacStandardIdSpectralSectionBreakdown>,
    pub dominant_escape_section: Option<AacStandardIdSpectralSectionBreakdown>,
}

#[cfg(feature = "aac")]
impl AacStandardIdPayloadBreakdown {
    #[must_use]
    pub fn total_bits(self) -> usize {
        self.section_bits + self.scale_factor_bits + self.spectral_bits
    }
}

#[cfg(feature = "aac")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AacStandardIdQualityControlProfile {
    pub frames: usize,
    pub channels: usize,
    pub max_frame_len: usize,
    pub min_frame_budget_slack: isize,
    pub max_quantized_abs_limit: u32,
    pub max_abs: i32,
    pub sections: usize,
    pub escape_sections: usize,
    pub total_bits: usize,
    pub spectral_bits: usize,
    pub escape_spectral_bits: usize,
    pub scale_factor_bits: usize,
    pub scale_factor_bands: usize,
    pub raised_scale_factor_bands: usize,
    pub max_scale_factor_delta: i16,
    pub mean_scale_factor_delta: f64,
}

#[cfg(feature = "aac")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AacStandardIdQualityControlCandidate {
    pub global_gain: u8,
    pub scale_factor_magnitude_bias: i16,
    pub max_quantized_abs: u32,
    pub profile: AacStandardIdQualityControlProfile,
}

/// Decodes supported audio bytes into interleaved PCM.
pub fn decode(input: &[u8]) -> Result<AudioBuffer, Error> {
    decode_impl(input)
}

/// Controls whether `encode_with_mode` may return experimental codec output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeMode {
    /// Preserve the regular `encode` behavior, including documented experimental scaffolds.
    Compatibility,
    /// Reject outputs that are not yet production-grade for non-silent lossy encoders.
    ProductionOnly,
}

/// Stateful decoder that buffers chunks until a complete audio stream decodes.
#[derive(Default)]
pub struct StreamDecoder {
    pending: Vec<u8>,
}

impl StreamDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a chunk and returns PCM once the buffered input forms a complete stream.
    pub fn decode_stream(&mut self, chunk: &[u8]) -> Result<Option<AudioBuffer>, Error> {
        if chunk.is_empty() && self.pending.is_empty() {
            return Ok(None);
        }
        self.pending.extend_from_slice(chunk);
        match decode(&self.pending) {
            Ok(pcm) => {
                self.pending.clear();
                Ok(Some(pcm))
            }
            Err(err) if is_incomplete_stream_error(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Drops any buffered partial input.
    pub fn reset(&mut self) {
        self.pending.clear();
    }

    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.pending.len()
    }
}

/// Encodes interleaved PCM in the requested format.
pub fn encode(format: Format, pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_with_mode(format, pcm, EncodeMode::Compatibility)
}

/// Encodes interleaved PCM while applying a caller-selected stability policy.
///
/// `ProductionOnly` accepts the currently production-grade paths and rejects
/// non-silent lossy output that still relies on unsupported scaffold logic.
pub fn encode_with_mode(
    format: Format,
    pcm: &AudioBuffer,
    mode: EncodeMode,
) -> Result<Vec<u8>, Error> {
    if mode == EncodeMode::ProductionOnly {
        if let Some(reason) = production_encode_rejection_reason(format, pcm) {
            return Err(Error::UnsupportedFeature(reason));
        }
    }

    match format {
        Format::Wav => encode_wav_impl(pcm),
        Format::Flac => encode_flac_impl(pcm),
        Format::Mp3 => encode_mp3_impl(pcm),
        Format::Vorbis => encode_vorbis_impl(pcm),
        Format::Opus => encode_opus_impl(pcm),
        Format::Aac => encode_aac_impl(pcm),
    }
}

fn production_encode_rejection_reason(format: Format, pcm: &AudioBuffer) -> Option<&'static str> {
    if is_silent_pcm(pcm) {
        return None;
    }

    match format {
        Format::Mp3 if !is_mp3_non_silent_production_candidate(pcm) => {
            Some("production MP3 encode currently supports mono/stereo MPEG-1 sample rates only")
        }
        Format::Aac if !is_aac_non_silent_production_candidate(pcm) => Some(
            "production AAC-LC encode currently supports mono/stereo 7.35/8/11.025/12/16/22.05/24/32/44.1/48/64/88.2/96kHz only",
        ),
        _ => None,
    }
}

fn is_mp3_non_silent_production_candidate(pcm: &AudioBuffer) -> bool {
    matches!(pcm.channels, 1 | 2) && matches!(pcm.sample_rate, 32_000 | 44_100 | 48_000)
}

#[cfg(feature = "aac")]
fn is_aac_non_silent_production_candidate(pcm: &AudioBuffer) -> bool {
    matches!(pcm.channels, 1 | 2)
        && sc_aac::aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate).is_some()
}

#[cfg(not(feature = "aac"))]
fn is_aac_non_silent_production_candidate(_pcm: &AudioBuffer) -> bool {
    false
}

fn is_silent_pcm(pcm: &AudioBuffer) -> bool {
    pcm.samples.iter().all(|sample| *sample == 0.0)
}

fn is_incomplete_stream_error(err: &Error) -> bool {
    matches!(err, Error::InvalidInput(reason) if reason.contains("truncated"))
}

#[cfg(all(feature = "decode", not(feature = "aac")))]
fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
    match sc_decode::decode(input) {
        Err(Error::UnsupportedFormat) => decode_mp3_fallback(input)
            .or_else(|| decode_opus_fallback(input))
            .unwrap_or(Err(Error::UnsupportedFormat)),
        result => result,
    }
}

#[cfg(all(feature = "decode", feature = "aac"))]
fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) == Some(Format::Aac) || is_m4a_container(input) {
        if let Ok(decoded) = sc_aac::decode(input) {
            return Ok(decoded);
        }
    }

    match sc_decode::decode(input) {
        Err(err) => decode_mp3_fallback(input)
            .or_else(|| decode_opus_fallback(input))
            .unwrap_or_else(|| {
                if detect(input) == Some(Format::Aac) {
                    sc_aac::decode(input)
                } else {
                    Err(err)
                }
            }),
        result => result,
    }
}

#[cfg(all(feature = "decode", feature = "aac"))]
fn is_m4a_container(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}

#[cfg(all(feature = "decode", feature = "mp3"))]
fn decode_mp3_fallback(input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    sc_mp3::FrameHeader::parse(input)
        .is_ok()
        .then(|| sc_mp3::decode(input))
}

#[cfg(all(feature = "decode", not(feature = "mp3")))]
fn decode_mp3_fallback(_input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    None
}

#[cfg(all(feature = "decode", feature = "opus"))]
fn decode_opus_fallback(input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    (detect(input) == Some(Format::Opus)).then(|| sc_opus::decode(input))
}

#[cfg(all(feature = "decode", not(feature = "opus")))]
fn decode_opus_fallback(_input: &[u8]) -> Option<Result<AudioBuffer, Error>> {
    None
}

#[cfg(not(feature = "decode"))]
fn decode_impl(input: &[u8]) -> Result<AudioBuffer, Error> {
    match detect(input) {
        Some(Format::Wav) => decode_wav(input),
        Some(Format::Flac) => decode_flac(input),
        Some(Format::Mp3) => decode_mp3(input),
        Some(Format::Vorbis) => decode_vorbis(input),
        Some(Format::Opus) => decode_opus(input),
        Some(Format::Aac) => decode_aac(input),
        None => Err(Error::UnsupportedFormat),
    }
}

#[cfg(feature = "wav")]
/// Decodes WAV bytes into interleaved PCM.
pub fn decode_wav(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_wav::decode(input)
}

#[cfg(not(feature = "wav"))]
/// Decodes WAV bytes into interleaved PCM.
pub fn decode_wav(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "flac")]
/// Decodes FLAC bytes into interleaved PCM.
pub fn decode_flac(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_flac::decode(input)
}

#[cfg(not(feature = "flac"))]
/// Decodes FLAC bytes into interleaved PCM.
pub fn decode_flac(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "decode")]
/// Decodes MP3 bytes into interleaved PCM.
pub fn decode_mp3(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Mp3) {
        return Err(Error::UnsupportedFormat);
    }
    decode_impl(input)
}

#[cfg(all(feature = "mp3", not(feature = "decode")))]
/// Decodes MP3 bytes into interleaved PCM.
pub fn decode_mp3(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_mp3::decode(input)
}

#[cfg(all(not(feature = "mp3"), not(feature = "decode")))]
/// Decodes MP3 bytes into interleaved PCM.
pub fn decode_mp3(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(all(feature = "decode", not(feature = "vorbis")))]
/// Decodes Vorbis bytes into interleaved PCM.
pub fn decode_vorbis(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Vorbis) {
        return Err(Error::UnsupportedFormat);
    }
    sc_decode::decode(input)
}

#[cfg(feature = "vorbis")]
/// Decodes Vorbis bytes into interleaved PCM.
pub fn decode_vorbis(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_vorbis::decode(input)
}

#[cfg(all(not(feature = "vorbis"), not(feature = "decode")))]
/// Decodes Vorbis bytes into interleaved PCM.
pub fn decode_vorbis(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(all(feature = "decode", not(feature = "opus")))]
/// Decodes Opus bytes into interleaved PCM when the decode backend supports it.
pub fn decode_opus(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Opus) {
        return Err(Error::UnsupportedFormat);
    }
    sc_decode::decode(input)
}

#[cfg(feature = "opus")]
/// Decodes Opus bytes into interleaved PCM.
pub fn decode_opus(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Opus) {
        return Err(Error::UnsupportedFormat);
    }
    sc_opus::decode(input)
}

#[cfg(all(not(feature = "opus"), not(feature = "decode")))]
/// Decodes Opus bytes into interleaved PCM.
pub fn decode_opus(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "decode")]
/// Decodes AAC ADTS or M4A bytes into interleaved PCM.
pub fn decode_aac(input: &[u8]) -> Result<AudioBuffer, Error> {
    if detect(input) != Some(Format::Aac) && !is_m4a_container_for_decode(input) {
        return Err(Error::UnsupportedFormat);
    }
    decode_impl(input)
}

#[cfg(all(feature = "aac", not(feature = "decode")))]
/// Decodes AAC ADTS bytes into interleaved PCM.
pub fn decode_aac(input: &[u8]) -> Result<AudioBuffer, Error> {
    sc_aac::decode(input)
}

#[cfg(all(not(feature = "aac"), not(feature = "decode")))]
/// Decodes AAC ADTS bytes into interleaved PCM.
pub fn decode_aac(_input: &[u8]) -> Result<AudioBuffer, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(all(feature = "decode", feature = "aac"))]
fn is_m4a_container_for_decode(input: &[u8]) -> bool {
    is_m4a_container(input)
}

#[cfg(all(feature = "decode", not(feature = "aac")))]
fn is_m4a_container_for_decode(input: &[u8]) -> bool {
    input.len() >= 12
        && input.get(4..8) == Some(b"ftyp")
        && matches!(
            input.get(8..12),
            Some(b"M4A ") | Some(b"mp42") | Some(b"isom") | Some(b"iso2")
        )
}

/// Encodes interleaved PCM as WAV.
#[cfg(feature = "wav")]
pub fn encode_wav(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_wav_impl(pcm)
}

#[cfg(feature = "flac")]
pub fn encode_flac(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_flac_impl(pcm)
}

#[cfg(feature = "mp3")]
pub fn encode_mp3(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_mp3_impl(pcm)
}

#[cfg(feature = "vorbis")]
pub fn encode_vorbis(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_vorbis_impl(pcm)
}

#[cfg(feature = "opus")]
pub fn encode_opus(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_opus_impl(pcm)
}

#[cfg(feature = "aac")]
pub fn encode_aac(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    encode_aac_impl(pcm)
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
    let scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
    let channel = AacScaleFactorChannel::new(channel_config, &scale_factors);
    let scale_factor_table = aac_scale_factor_delta_table();
    let spectral_tables = aac_unsigned_pairs7_unit_magnitude_spectral_tables();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel,
            channel,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        _ => Err(Error::InvalidInput(
            "AAC bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        180,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();
    let spectral_tables = aac_unsigned_pairs7_unit_magnitude_spectral_tables();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        _ => Err(Error::InvalidInput(
            "AAC selected-scale-factor bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_selected_scale_factor_frame_details_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        180,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();
    let spectral_tables = aac_unsigned_pairs7_unit_magnitude_spectral_tables();

    match pcm.channels {
        1 => select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        2 => select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            offsets,
            AAC_LC_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
            spectral_tables,
        ),
        _ => Err(Error::InvalidInput(
            "AAC selected-scale-factor bitrate frame details require mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factors_by_frame = constant_aac_scale_factors_by_frame(
        pcm,
        i16::from(channel_config.global_gain),
        offsets.len() - 1,
    );
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let scale_factor_table = aac_scale_factor_delta_table();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            adts,
            AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            pcm,
            0,
            offsets,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            adts,
            AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            pcm,
            0,
            offsets,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard spectral-offset bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard spectral-offset selected-scale-factor bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_global_gain(channels: u16) -> Result<u8, Error> {
    match channels {
        1 => Ok(AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN),
        2 => Ok(AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_GLOBAL_GAIN),
        _ => Err(Error::InvalidInput(
            "AAC standard-id selected-scale-factor global gain requires mono or stereo",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_magnitude_bias() -> i16 {
    AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(
    channels: u16,
) -> Result<u32, Error> {
    Ok(aac_standard_id_selected_scale_factor_balance_profile(channels)?.max_quantized_abs)
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_balance_profile(
    channels: u16,
) -> Result<AacStandardIdSelectedScaleFactorBalanceProfile, Error> {
    match channels {
        1 => Ok(AacStandardIdSelectedScaleFactorBalanceProfile {
            recommended_global_gain: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN,
            global_gain_deltas: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GAIN_DELTAS,
            magnitude_biases: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIASES,
            selected_global_gain: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN,
            selected_magnitude_bias:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIAS,
            max_quantized_abs:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS,
        }),
        2 => Ok(AacStandardIdSelectedScaleFactorBalanceProfile {
            recommended_global_gain: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_GLOBAL_GAIN,
            global_gain_deltas: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GAIN_DELTAS,
            magnitude_biases:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIASES,
            selected_global_gain: AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN,
            selected_magnitude_bias:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS,
            max_quantized_abs:
                AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS,
        }),
        _ => Err(Error::InvalidInput(
            "AAC standard-id selected-scale-factor balanced profile requires mono or stereo",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_balanced_parameters(
    channels: u16,
) -> Result<(u8, i16, u32), Error> {
    let profile = aac_standard_id_selected_scale_factor_balance_profile(channels)?;
    Ok((
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
        profile.max_quantized_abs,
    ))
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_selected_scale_factor_parameters(channels: u16) -> Result<(u8, i16), Error> {
    Ok((
        aac_standard_id_selected_scale_factor_global_gain(channels)?,
        aac_standard_id_selected_scale_factor_magnitude_bias(),
    ))
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();

    match pcm.channels {
        1 => encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            max_quantized_abs,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            max_quantized_abs,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard spectral-offset selected-scale-factor max-quantized-abs bitrate encode requires mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    let (global_gain, scale_factor_magnitude_bias, max_quantized_abs) =
        aac_standard_id_selected_scale_factor_balanced_parameters(pcm.channels)?;
    encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC channel count is unsupported"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let channel_config = AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();

    match pcm.channels {
        1 => select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard selected-scale-factor frame details require mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| Error::InvalidInput("AAC standard frame details require u8 channels"))?;
    let adts = AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate).ok_or(
        Error::UnsupportedFeature("AAC-LC scale-factor offsets for sample rate"),
    )?;
    let channel_config = AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| Error::InvalidInput("AAC scale-factor band count exceeds u8"))?,
    );
    let scale_factor_table = aac_scale_factor_delta_table();
    match pcm.channels {
        1 => select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            max_quantized_abs,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        2 => select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            max_quantized_abs,
            target_bitrate_bps,
            &scale_factor_table,
        ),
        _ => Err(Error::InvalidInput(
            "AAC standard selected-scale-factor frame details require mono or stereo PCM",
        )),
    }
}

#[cfg(feature = "aac")]
pub fn aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    let (global_gain, scale_factor_magnitude_bias, max_quantized_abs) =
        aac_standard_id_selected_scale_factor_balanced_parameters(pcm.channels)?;
    aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        scale_factor_magnitude_bias,
        max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<AacSelectedScaleFactorProfile, Error> {
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let mut bands = 0usize;
    let mut raised_bands = 0usize;
    let mut max_delta = 0i16;
    let mut delta_sum = 0i64;

    for (frame_index, detail) in details.iter().enumerate() {
        let start_frame = frame_index
            .checked_mul(1024)
            .ok_or(Error::InvalidInput("AAC frame index overflows"))?;
        for channel in 0..usize::from(pcm.channels) {
            let quantized = quantize_pcm_long_block(pcm, channel, start_frame, detail.step)?;
            let scale_factors =
                select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
                    &quantized,
                    offsets,
                    i16::from(global_gain),
                    scale_factor_magnitude_bias,
                )?;
            for scale_factor in scale_factors {
                let delta = scale_factor - i16::from(global_gain);
                bands += 1;
                raised_bands += usize::from(delta > 0);
                max_delta = max_delta.max(delta);
                delta_sum += i64::from(delta);
            }
        }
    }

    if bands == 0 {
        return Err(Error::InvalidInput(
            "AAC scale-factor profile requires at least one band",
        ));
    }

    Ok(AacSelectedScaleFactorProfile {
        frames: details.len(),
        channels: usize::from(pcm.channels),
        bands,
        raised_bands,
        max_delta,
        mean_delta: delta_sum as f64 / bands as f64,
    })
}

#[cfg(feature = "aac")]
pub fn aac_recommended_standard_selected_scale_factor_profile_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacSelectedScaleFactorProfile, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        global_gain,
        scale_factor_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_selected_scale_factor_profile_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacSelectedScaleFactorProfile, Error> {
    let profile = aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)?;
    aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
fn aac_scale_factor_band_index(offsets: &[usize], offset: usize) -> Result<usize, Error> {
    offsets
        .iter()
        .position(|band_offset| *band_offset == offset)
        .ok_or(Error::InvalidInput(
            "AAC scale-factor band offset not found",
        ))
}

#[cfg(feature = "aac")]
fn aac_spectral_pairs_for_i32_slice(quantized: &[i32]) -> Result<Vec<AacSpectralPair>, Error> {
    if quantized.len() % 2 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral pair slice length must be even",
        ));
    }
    quantized
        .chunks_exact(2)
        .map(|pair| {
            Ok(AacSpectralPair::new(
                i16::try_from(pair[0])
                    .map_err(|_| Error::InvalidInput("AAC spectral pair x exceeds i16"))?,
                i16::try_from(pair[1])
                    .map_err(|_| Error::InvalidInput("AAC spectral pair y exceeds i16"))?,
            ))
        })
        .collect()
}

#[cfg(feature = "aac")]
fn aac_spectral_quads_for_i32_slice(quantized: &[i32]) -> Result<Vec<AacSpectralQuad>, Error> {
    if quantized.len() % 4 != 0 {
        return Err(Error::InvalidInput(
            "AAC spectral quad slice length must be divisible by four",
        ));
    }
    quantized
        .chunks_exact(4)
        .map(|quad| {
            Ok(AacSpectralQuad::new(
                i16::try_from(quad[0])
                    .map_err(|_| Error::InvalidInput("AAC spectral quad v exceeds i16"))?,
                i16::try_from(quad[1])
                    .map_err(|_| Error::InvalidInput("AAC spectral quad w exceeds i16"))?,
                i16::try_from(quad[2])
                    .map_err(|_| Error::InvalidInput("AAC spectral quad x exceeds i16"))?,
                i16::try_from(quad[3])
                    .map_err(|_| Error::InvalidInput("AAC spectral quad y exceeds i16"))?,
            ))
        })
        .collect()
}

#[cfg(feature = "aac")]
fn aac_standard_id_section_codebook_costs(quantized: &[i32]) -> Result<Vec<(u8, usize)>, Error> {
    if quantized.iter().all(|coeff| *coeff == 0) {
        return Ok(vec![(0, 0)]);
    }

    let mut costs = Vec::new();
    if quantized.len() % 4 == 0 {
        let quads = aac_spectral_quads_for_i32_slice(quantized)?;
        for (codebook_id, table) in [
            (1, aac_signed_quads1_table()),
            (2, aac_signed_quads2_table()),
        ] {
            if let Ok(packed) = pack_spectral_quads_with_table(&quads, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
        for (codebook_id, table) in [
            (3, aac_unsigned_quads3_table()),
            (4, aac_unsigned_quads4_table()),
        ] {
            if let Ok(packed) = pack_spectral_quads_with_sign_bits(&quads, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
    }

    if quantized.len() % 2 == 0 {
        let pairs = aac_spectral_pairs_for_i32_slice(quantized)?;
        for (codebook_id, table) in [
            (5, aac_signed_pairs5_table()),
            (6, aac_signed_pairs6_table()),
        ] {
            if let Ok(packed) = pack_spectral_pairs_with_table(&pairs, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
        for (codebook_id, table) in [
            (7, aac_unsigned_pairs7_table()),
            (8, aac_unsigned_pairs8_table()),
            (9, aac_unsigned_pairs9_table()),
            (10, aac_unsigned_pairs10_table()),
            (11, aac_escape_table()),
        ] {
            if let Ok(packed) = pack_spectral_pairs_with_sign_bits(&pairs, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
    }

    if costs.is_empty() {
        return Err(Error::UnsupportedFeature(
            "AAC section has no packable standard-id codebook candidates",
        ));
    }
    costs.sort_by_key(|(codebook_id, bit_len)| (*bit_len, *codebook_id));
    costs.dedup_by_key(|(codebook_id, _)| *codebook_id);
    Ok(costs)
}

#[cfg(feature = "aac")]
fn max_abs_i32(values: &[i32]) -> Result<i32, Error> {
    values
        .iter()
        .map(|value| {
            value
                .checked_abs()
                .ok_or(Error::InvalidInput("AAC spectral coefficient overflows"))
        })
        .try_fold(0, |acc, value| value.map(|value| acc.max(value)))
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<AacStandardIdPayloadBreakdown, Error> {
    let offsets = aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or(Error::UnsupportedFeature("AAC-LC sample rate"))?;
    let scale_factor_table = aac_scale_factor_delta_table();

    let mut sections = 0usize;
    let mut escape_sections = 0usize;
    let mut max_abs = 0i32;
    let mut section_bits = 0usize;
    let mut scale_factor_bits = 0usize;
    let mut spectral_bits = 0usize;
    let mut escape_spectral_bits = 0usize;
    let mut dominant_spectral_section = None;
    let mut dominant_escape_section = None;

    for (frame_index, detail) in details.iter().enumerate() {
        let start_frame = frame_index
            .checked_mul(1024)
            .ok_or(Error::InvalidInput("AAC frame index overflows"))?;
        for channel in 0..usize::from(pcm.channels) {
            let quantized = quantize_pcm_long_block(pcm, channel, start_frame, detail.step)?;
            let planned_sections =
                plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(&quantized, offsets)?;
            let scale_factors =
                select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
                    &quantized,
                    offsets,
                    i16::from(global_gain),
                    scale_factor_magnitude_bias,
                )?;
            let scale_factor_deltas = plan_spectral_scale_factor_deltas_by_offsets(
                &planned_sections,
                offsets,
                &scale_factors,
                i16::from(global_gain),
            )?;
            let packed_scale_factors =
                pack_scale_factor_deltas_with_table(&scale_factor_deltas, &scale_factor_table)?;
            let split_without_scale_factors =
                split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
                    &quantized, offsets,
                )?;
            let split_with_scale_factors =
                split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
                    &quantized,
                    offsets,
                    packed_scale_factors,
                )?;

            if split_without_scale_factors.spectral_bits.bit_len
                != split_with_scale_factors.spectral_bits.bit_len
            {
                return Err(Error::InvalidInput(
                    "AAC standard-id payload split changed spectral bits when adding scale factors",
                ));
            }

            sections += planned_sections.len();
            escape_sections += planned_sections
                .iter()
                .filter(|section| section.codebook_id == 11)
                .count();
            for section in &planned_sections {
                let section_payload =
                    split_aac_lc_standard_sectioned_spectral_payload_with_offsets_and_sign_bits(
                        std::slice::from_ref(section),
                        &quantized,
                        offsets,
                    )?;
                let section_spectral_bits = section_payload.spectral_bits.bit_len;
                let section_max_abs = max_abs_i32(&quantized[section.start..section.end])?;
                let section_codebook_costs =
                    aac_standard_id_section_codebook_costs(&quantized[section.start..section.end])?;
                let best_alternative = section_codebook_costs
                    .iter()
                    .copied()
                    .find(|(codebook_id, _)| *codebook_id != section.codebook_id);
                let section_breakdown = AacStandardIdSpectralSectionBreakdown {
                    frame_index,
                    channel,
                    start_band: aac_scale_factor_band_index(offsets, section.start)?,
                    end_band: aac_scale_factor_band_index(offsets, section.end)?,
                    start: section.start,
                    end: section.end,
                    codebook_id: section.codebook_id,
                    max_abs: section_max_abs,
                    spectral_bits: section_spectral_bits,
                    best_alternative_codebook_id: best_alternative
                        .map(|(codebook_id, _)| codebook_id),
                    best_alternative_spectral_bits: best_alternative.map(|(_, bit_len)| bit_len),
                };
                if section.codebook_id == 11 {
                    escape_spectral_bits += section_spectral_bits;
                    if dominant_escape_section.is_none_or(
                        |dominant: AacStandardIdSpectralSectionBreakdown| {
                            section_breakdown.spectral_bits > dominant.spectral_bits
                        },
                    ) {
                        dominant_escape_section = Some(section_breakdown);
                    }
                }
                if dominant_spectral_section.is_none_or(
                    |dominant: AacStandardIdSpectralSectionBreakdown| {
                        section_breakdown.spectral_bits > dominant.spectral_bits
                    },
                ) {
                    dominant_spectral_section = Some(section_breakdown);
                }
            }

            max_abs = max_abs.max(max_abs_i32(&quantized)?);
            section_bits += split_without_scale_factors
                .section_and_scale_factor_bits
                .bit_len;
            scale_factor_bits += split_with_scale_factors
                .section_and_scale_factor_bits
                .bit_len
                .checked_sub(
                    split_without_scale_factors
                        .section_and_scale_factor_bits
                        .bit_len,
                )
                .ok_or(Error::InvalidInput(
                    "AAC scale-factor bit count underflowed",
                ))?;
            spectral_bits += split_with_scale_factors.spectral_bits.bit_len;
        }
    }

    Ok(AacStandardIdPayloadBreakdown {
        frames: details.len(),
        channels: usize::from(pcm.channels),
        sections,
        escape_sections,
        max_abs,
        section_bits,
        scale_factor_bits,
        spectral_bits,
        escape_spectral_bits,
        dominant_spectral_section,
        dominant_escape_section,
    })
}

#[cfg(feature = "aac")]
pub fn aac_recommended_standard_id_payload_breakdown_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacStandardIdPayloadBreakdown, Error> {
    let (global_gain, scale_factor_magnitude_bias) =
        aac_standard_id_selected_scale_factor_parameters(pcm.channels)?;
    aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        global_gain,
        scale_factor_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_id_payload_breakdown_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacStandardIdPayloadBreakdown, Error> {
    let profile = aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)?;
    aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
    )
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_quality_control_profile_for_frame_details_with_magnitude_bias_max_quantized_abs(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<AacStandardIdQualityControlProfile, Error> {
    let breakdown = aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
        pcm,
        details,
        global_gain,
        scale_factor_magnitude_bias,
    )?;
    let scale_factor_profile =
        aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
            pcm,
            details,
            global_gain,
            scale_factor_magnitude_bias,
        )?;
    let max_frame_len = details
        .iter()
        .map(|detail| detail.frame_len)
        .max()
        .unwrap_or(0);
    let min_frame_budget_slack = details
        .iter()
        .map(|detail| detail.frame_capacity_bytes as isize - detail.frame_len as isize)
        .min()
        .unwrap_or(0);

    Ok(AacStandardIdQualityControlProfile {
        frames: details.len(),
        channels: usize::from(pcm.channels),
        max_frame_len,
        min_frame_budget_slack,
        max_quantized_abs_limit: max_quantized_abs,
        max_abs: breakdown.max_abs,
        sections: breakdown.sections,
        escape_sections: breakdown.escape_sections,
        total_bits: breakdown.total_bits(),
        spectral_bits: breakdown.spectral_bits,
        escape_spectral_bits: breakdown.escape_spectral_bits,
        scale_factor_bits: breakdown.scale_factor_bits,
        scale_factor_bands: scale_factor_profile.bands,
        raised_scale_factor_bands: scale_factor_profile.raised_bands,
        max_scale_factor_delta: scale_factor_profile.max_delta,
        mean_scale_factor_delta: scale_factor_profile.mean_delta,
    })
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_id_quality_control_profile_for_frame_details(
    pcm: &AudioBuffer,
    details: &[AacPcmFrameStepSelection],
) -> Result<AacStandardIdQualityControlProfile, Error> {
    let profile = aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)?;
    aac_standard_id_quality_control_profile_for_frame_details_with_magnitude_bias_max_quantized_abs(
        pcm,
        details,
        profile.selected_global_gain,
        profile.selected_magnitude_bias,
        profile.max_quantized_abs,
    )
}

#[cfg(feature = "aac")]
pub fn aac_balanced_standard_id_quality_control_profile_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<AacStandardIdQualityControlProfile, Error> {
    let details = aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
        pcm,
        target_bitrate_bps,
    )?;
    aac_balanced_standard_id_quality_control_profile_for_frame_details(pcm, &details)
}

#[cfg(feature = "aac")]
pub fn aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<AacStandardIdQualityControlCandidate>, Error> {
    let balance_profile = aac_standard_id_selected_scale_factor_balance_profile(pcm.channels)?;
    let mut candidates = Vec::new();

    for &global_gain_delta in balance_profile.global_gain_deltas {
        let global_gain = balance_profile
            .recommended_global_gain
            .saturating_add(global_gain_delta);
        for &scale_factor_magnitude_bias in balance_profile.magnitude_biases {
            let details =
                aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
                    pcm,
                    target_bitrate_bps,
                    global_gain,
                    scale_factor_magnitude_bias,
                    balance_profile.max_quantized_abs,
                )?;
            let profile =
                aac_standard_id_quality_control_profile_for_frame_details_with_magnitude_bias_max_quantized_abs(
                    pcm,
                    &details,
                    global_gain,
                    scale_factor_magnitude_bias,
                    balance_profile.max_quantized_abs,
                )?;

            if profile.min_frame_budget_slack >= 0
                && profile.max_abs
                    <= i32::try_from(balance_profile.max_quantized_abs).unwrap_or(i32::MAX)
            {
                candidates.push(AacStandardIdQualityControlCandidate {
                    global_gain,
                    scale_factor_magnitude_bias,
                    max_quantized_abs: balance_profile.max_quantized_abs,
                    profile,
                });
            }
        }
    }

    if candidates.is_empty() {
        return Err(Error::InvalidInput(
            "AAC standard-id balanced quality-control profile found no constrained candidates",
        ));
    }

    Ok(candidates)
}

#[cfg(feature = "aac")]
pub fn aac_standard_selected_scale_factor_frame_details_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<AacPcmFrameStepSelection>, Error> {
    aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        0,
    )
}

#[cfg(feature = "aac")]
pub fn encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, Error> {
    encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        0,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(&encode_aac_adts_with_bitrate(pcm, target_bitrate_bps)?)
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(&encode_aac_adts_with_selected_scale_factors_and_bitrate(
        pcm,
        target_bitrate_bps,
    )?)
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_standard_spectral_offsets_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(&encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
    )?)
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
            pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
            pcm,
            target_bitrate_bps,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
    scale_factor_magnitude_bias: i16,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
            pcm,
            target_bitrate_bps,
            global_gain,
            scale_factor_magnitude_bias,
            max_quantized_abs,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    max_quantized_abs: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
            pcm,
            target_bitrate_bps,
            max_quantized_abs,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
) -> Result<Vec<u8>, Error> {
    mux_aac_adts_as_m4a(
        &encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
            pcm,
            target_bitrate_bps,
        )?,
    )
}

#[cfg(feature = "aac")]
pub fn encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
    pcm: &AudioBuffer,
    target_bitrate_bps: u32,
    global_gain: u8,
) -> Result<Vec<u8>, Error> {
    encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
        pcm,
        target_bitrate_bps,
        global_gain,
        0,
    )
}

#[cfg(feature = "aac")]
pub fn frame_aac_adts(config: AdtsConfig, access_unit: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::frame_adts(config, access_unit)
}

#[cfg(feature = "aac")]
pub fn frame_aac_adts_stream<'a>(
    config: AdtsConfig,
    access_units: impl IntoIterator<Item = &'a [u8]>,
) -> Result<Vec<u8>, Error> {
    sc_aac::frame_adts_stream(config, access_units)
}

#[cfg(feature = "aac")]
pub fn mux_aac_adts_as_m4a(adts: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::mux_adts_as_m4a(adts)
}

#[cfg(feature = "aac")]
pub fn demux_m4a_as_aac_adts(input: &[u8]) -> Result<Vec<u8>, Error> {
    sc_aac::demux_m4a_as_adts(input)
}

#[cfg(feature = "aac")]
fn constant_aac_scale_factors_by_frame(
    pcm: &AudioBuffer,
    scale_factor: i16,
    band_count: usize,
) -> Vec<Vec<i16>> {
    let frame_count = pcm.samples.len().div_ceil(usize::from(pcm.channels) * 1024);
    (0..frame_count)
        .map(|_| vec![scale_factor; band_count])
        .collect()
}

#[cfg(feature = "wav")]
fn encode_wav_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_wav::encode(pcm)
}

#[cfg(not(feature = "wav"))]
fn encode_wav_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "flac")]
fn encode_flac_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_flac::encode(pcm)
}

#[cfg(not(feature = "flac"))]
fn encode_flac_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "mp3")]
fn encode_mp3_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_mp3::encode(pcm)
}

#[cfg(not(feature = "mp3"))]
fn encode_mp3_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "vorbis")]
fn encode_vorbis_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_vorbis::encode(pcm)
}

#[cfg(not(feature = "vorbis"))]
fn encode_vorbis_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "opus")]
fn encode_opus_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_opus::encode(pcm)
}

#[cfg(not(feature = "opus"))]
fn encode_opus_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(feature = "aac")]
fn encode_aac_impl(pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    sc_aac::encode(pcm)
}

#[cfg(not(feature = "aac"))]
fn encode_aac_impl(_pcm: &AudioBuffer) -> Result<Vec<u8>, Error> {
    Err(Error::UnsupportedFormat)
}

#[cfg(test)]
mod tests {
    use super::{
        decode, encode, encode_wav, encode_with_mode, AudioBuffer, EncodeMode, Error, Format,
        StreamDecoder,
    };
    use sc_core::BitReader;

    #[cfg(feature = "opus")]
    use super::encode_opus;

    #[test]
    fn dispatches_wav_roundtrip() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let wav = encode(Format::Wav, &pcm).unwrap();
        let decoded = decode(&wav).unwrap();

        assert_eq!(
            encode_with_mode(Format::Wav, &pcm, EncodeMode::ProductionOnly).unwrap(),
            wav
        );
        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(encode_wav(&pcm).unwrap(), wav);
        assert!(matches!(
            super::decode_mp3(&wav),
            Err(Error::UnsupportedFormat)
        ));
        assert!(matches!(
            super::decode_vorbis(&wav),
            Err(Error::UnsupportedFormat)
        ));
        assert!(matches!(
            super::decode_opus(&wav),
            Err(Error::UnsupportedFormat)
        ));
    }

    #[test]
    fn stream_decoder_buffers_until_complete_input() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0, 0.25, -0.25]).unwrap();
        let wav = encode(Format::Wav, &pcm).unwrap();
        let split = wav.len() - 2;
        let mut decoder = StreamDecoder::new();

        assert!(decoder.decode_stream(&wav[..split]).unwrap().is_none());
        assert!(decoder.buffered_len() > 0);
        let decoded = decoder
            .decode_stream(&wav[split..])
            .unwrap()
            .expect("complete stream should decode");

        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    #[cfg(feature = "flac")]
    fn dispatches_flac_roundtrip() {
        let samples = (0..128)
            .map(|sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();
        let flac = encode(Format::Flac, &pcm).unwrap();
        let decoded = decode(&flac).unwrap();

        assert_eq!(
            encode_with_mode(Format::Flac, &pcm, EncodeMode::ProductionOnly).unwrap(),
            flac
        );
        assert_eq!(decoded.sample_rate, pcm.sample_rate);
        assert_eq!(decoded.channels, pcm.channels);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
    }

    #[test]
    fn dispatches_known_unimplemented_formats_as_unsupported() {
        let err = decode(b"ID3\x04\0\0\0\0\0\0").unwrap_err();
        assert!(matches!(err, Error::UnsupportedFormat));
    }

    #[test]
    #[cfg(feature = "opus")]
    fn dispatches_opus_encode_to_ogg_stream() {
        let pcm = AudioBuffer::new(48_000, 1, vec![0.0; 4800]).unwrap();
        let encoded = encode(Format::Opus, &pcm).expect("opus encode");

        assert_eq!(&encoded[..4], b"OggS");
        assert_eq!(super::detect(&encoded), Some(Format::Opus));
        assert_eq!(encode_opus(&pcm).expect("encode_opus"), encoded);
        let production = encode_with_mode(Format::Opus, &pcm, EncodeMode::ProductionOnly)
            .expect("production opus");
        assert_eq!(&production[..4], b"OggS");
        assert_eq!(super::detect(&production), Some(Format::Opus));
    }

    #[test]
    #[cfg(feature = "vorbis")]
    fn dispatches_vorbis_encode_to_ogg_stream() {
        let pcm = AudioBuffer::new(48_000, 1, vec![0.0; 4800]).unwrap();
        let encoded = encode(Format::Vorbis, &pcm).expect("vorbis encode");
        assert_eq!(&encoded[..4], b"OggS");
        assert_eq!(super::detect(&encoded), Some(Format::Vorbis));
        let production = encode_with_mode(Format::Vorbis, &pcm, EncodeMode::ProductionOnly)
            .expect("production vorbis");
        assert_eq!(&production[..4], b"OggS");
        assert_eq!(super::detect(&production), Some(Format::Vorbis));
    }

    #[test]
    #[cfg(feature = "opus")]
    fn dispatches_ffmpeg_generated_ogg_opus_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-umbrella-opus-smoke-{}.opus",
            std::process::id()
        ));

        let status = std::process::Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.05:sample_rate=48000",
                "-ac",
                "1",
                "-c:a",
                "libopus",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("run ffmpeg");
        assert!(status.success(), "ffmpeg failed with {status}");

        let bytes = std::fs::read(&path).expect("read opus");
        let _ = std::fs::remove_file(&path);
        let decoded = decode(&bytes).expect("decode opus");
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));
    }

    #[test]
    #[cfg(feature = "vorbis")]
    fn dispatches_ffmpeg_generated_ogg_vorbis_when_available() {
        let Ok(ffmpeg) = std::env::var("SONARE_FFMPEG") else {
            return;
        };
        let path = std::env::temp_dir().join(format!(
            "sonare-codec-umbrella-vorbis-smoke-{}.ogg",
            std::process::id()
        ));

        let status = std::process::Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=0.05:sample_rate=48000",
                "-ac",
                "1",
                "-c:a",
                "libvorbis",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("run ffmpeg");
        assert!(status.success(), "ffmpeg failed with {status}");

        let bytes = std::fs::read(&path).expect("read vorbis");
        let _ = std::fs::remove_file(&path);
        let decoded = super::decode_vorbis(&bytes).expect("decode vorbis");
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 1);
        assert!(!decoded.samples.is_empty());
        assert!(decoded.samples.iter().any(|sample| sample.abs() > 0.0001));
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn dispatches_silent_mp3_encode() {
        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1152 * 2]).unwrap();

        let mp3 = encode(Format::Mp3, &pcm).unwrap();
        let decoded = decode(&mp3).unwrap();

        assert_eq!(
            encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap(),
            mp3
        );
        assert_eq!(&mp3[..2], &[0xff, 0xfb]);
        assert_eq!(super::detect(&mp3), Some(Format::Mp3));
        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), pcm.samples.len());
        assert_eq!(
            super::decode_mp3(&mp3).unwrap().samples.len(),
            pcm.samples.len()
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn dispatches_non_silent_mp3_encode_as_layer3_scaffold() {
        for (sample_rate, channels) in [
            (32_000, 1),
            (44_100, 1),
            (48_000, 1),
            (32_000, 2),
            (44_100, 2),
            (48_000, 2),
        ] {
            let mut samples = Vec::new();
            for frame in 0..2048 {
                for channel in 0..channels {
                    let phase = if channel == 0 { 0.01 } else { 0.013 };
                    samples.push(((frame as f32) * phase).sin() * 0.25);
                }
            }
            let pcm = AudioBuffer::new(sample_rate, channels, samples).unwrap();

            let mp3 = encode(Format::Mp3, &pcm).unwrap();
            let production =
                encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap();
            let zero_payload = super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                    &pcm,
                    f32::MAX,
                    super::Layer3EntropyTableProvider::default(),
                )
                .unwrap();
            let decoded = decode(&mp3).unwrap();

            assert_eq!(&mp3[..2], &[0xff, 0xfb]);
            assert_eq!(
                production, mp3,
                "sample_rate={sample_rate} channels={channels}"
            );
            assert_eq!(super::detect(&mp3), Some(Format::Mp3));
            assert!(mp3.len() > 4);
            assert_ne!(mp3, zero_payload);
            assert_eq!(decoded.sample_rate, sample_rate);
            assert_eq!(decoded.channels, channels);
            assert_eq!(decoded.samples.len(), 2304 * usize::from(channels));
        }
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn production_mono_mp3_uses_low_band_gain_entropy_reservoir_path() {
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let samples = (0..(frames * samples_per_frame))
            .map(|sample| {
                let t = sample as f32;
                0.24 * ((t * 0.043).sin() + 0.5 * (t * 0.131).sin())
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 1, samples).unwrap();

        let production = encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap();
        let production_candidates =
            super::mpeg1_layer3_production_pcm_step_candidates(pcm.channels).unwrap();
        let perceptual_cbr =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let entropy_targeted_reservoir = super::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &pcm,
            production_candidates,
            128,
            false,
            0,
            super::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        let low_band_gain_reservoir = super::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_quantized_band_gain_and_global_gain_bias_reservoir_and_table_provider(
            &pcm,
            &[2.0],
            128,
            false,
            0,
            super::Layer3QuantizedBandGain {
                band_start: 0,
                band_end: 7,
                gain: 1.5,
            },
            -4,
            super::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();

        let mut offset = 0_usize;
        let mut frame_index = 0_usize;
        let mut max_main_data_begin = 0_u32;
        while offset < production.len() {
            let header = super::FrameHeader::parse(&production[offset..offset + 4]).unwrap();
            let mut reader = BitReader::new(&production[offset + 4..]);
            let main_data_begin = reader.read_bits(9).unwrap();
            max_main_data_begin = max_main_data_begin.max(main_data_begin);
            offset += header.frame_len();
            frame_index += 1;
        }

        assert_eq!(offset, production.len());
        assert_eq!(frame_index, frames);
        assert!(
            max_main_data_begin > 0,
            "production MP3 stopped using the bit reservoir"
        );
        assert_eq!(
            production, low_band_gain_reservoir,
            "mono production MP3 should use the low-band gain/global-gain-bias entropy reservoir path"
        );
        assert_ne!(
            production, entropy_targeted_reservoir,
            "mono production MP3 should no longer use the older entropy-targeted perceptual reservoir payload"
        );
        assert_ne!(
            production, perceptual_cbr,
            "mono production MP3 should keep the reservoir layout, not the self-contained perceptual CBR layout"
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn production_stereo_mp3_uses_entropy_targeted_perceptual_reservoir_path() {
        let frames = 8_usize;
        let samples_per_frame = 1152_usize;
        let samples = (0..(frames * samples_per_frame * 2))
            .map(|index| {
                let frame = index / (samples_per_frame * 2);
                let t = ((index / 2) % samples_per_frame) as f32;
                let right = index % 2 == 1;
                if frame % 2 == 0 {
                    if right {
                        0.24 * ((t * 0.053).sin() + (t * 0.173).sin() + (t * 0.337).sin())
                    } else {
                        0.28 * ((t * 0.037).sin() + (t * 0.149).sin() + (t * 0.419).sin())
                    }
                } else if right {
                    0.018 * (t * 0.047).sin()
                } else {
                    0.02 * (t * 0.041).sin()
                }
            })
            .collect();
        let pcm = AudioBuffer::new(44_100, 2, samples).unwrap();

        let production = encode_with_mode(Format::Mp3, &pcm, EncodeMode::ProductionOnly).unwrap();
        let production_candidates =
            super::mpeg1_layer3_production_pcm_step_candidates(pcm.channels).unwrap();
        let entropy_targeted_details =
            super::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                production_candidates,
                128,
                false,
                0,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let perceptual_details =
            super::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let perceptual_reservoir =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let entropy_targeted_reservoir = super::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &pcm,
            production_candidates,
            128,
            false,
            0,
            super::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();

        assert!(entropy_targeted_details
            .iter()
            .all(|detail| { detail.perceptual_granules == 4 && detail.calibrated_granules == 0 }));
        assert!(entropy_targeted_details.iter().all(|detail| {
            detail.quality_guard_compared_granules == 0
                && detail.quality_guard_distortion_delta == 0.0
        }));
        assert!(entropy_targeted_details
            .iter()
            .any(|detail| detail.used_entropy_target_budget));
        assert_eq!(
            entropy_targeted_details
                .iter()
                .map(|detail| detail.entropy_target_bits)
                .sum::<usize>(),
            entropy_targeted_details
                .iter()
                .map(|detail| detail.frame_capacity_bytes * 8)
                .sum::<usize>()
        );

        let mut offset = 0_usize;
        let mut frame_index = 0_usize;
        let mut max_main_data_begin = 0_u32;
        while offset < production.len() {
            let header = super::FrameHeader::parse(&production[offset..offset + 4]).unwrap();
            let mut reader = BitReader::new(&production[offset + 4..]);
            let main_data_begin = reader.read_bits(9).unwrap();
            assert_eq!(
                entropy_targeted_details[frame_index].main_data_begin as u32,
                main_data_begin
            );
            max_main_data_begin = max_main_data_begin.max(main_data_begin);
            offset += header.frame_len();
            frame_index += 1;
        }

        assert_eq!(offset, production.len());
        assert_eq!(frame_index, entropy_targeted_details.len());
        assert!(
            max_main_data_begin > 0,
            "production stereo MP3 stopped using the bit reservoir"
        );
        assert_eq!(
            production, entropy_targeted_reservoir,
            "stereo production MP3 should use the entropy-targeted perceptual reservoir path"
        );
        assert_ne!(
            production, perceptual_reservoir,
            "stereo production MP3 should no longer use the raw perceptual reservoir path"
        );
        assert_eq!(perceptual_details.len(), entropy_targeted_details.len());
        assert_eq!(super::detect(&production), Some(Format::Mp3));
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_frame_scaffold_helper() {
        assert_eq!(
            super::MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS,
            &[
                1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
                26, 27, 28, 29, 30, 31
            ]
        );
        assert_eq!(
            super::MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS,
            &[]
        );
        assert_eq!(
            super::MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS,
            &[false, true]
        );

        let pcm = AudioBuffer::new(44_100, 2, vec![0.0; 1153 * 2]).unwrap();

        let scaffold =
            super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm,
                1.0,
                super::Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(scaffold, encode(Format::Mp3, &pcm).unwrap());
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_payload_budget_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let header = super::FrameHeader {
            version: super::MpegVersion::Mpeg1,
            layer: super::Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: super::ChannelMode::SingleChannel,
        };
        let provider = super::mpeg1_layer3_standard_table_provider();
        let unconstrained = super::select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            0,
            super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();

        let step =
            super::select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let details =
            super::select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let budgeted =
            super::encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                unconstrained.payload_bit_len,
                provider,
            )
            .unwrap();
        let selected =
            super::encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider(
                &pcm, step, provider,
            )
            .unwrap();

        assert_eq!(step, unconstrained.step);
        assert_eq!(details.step, step);
        assert_eq!(details.frame_capacity_bits, unconstrained.payload_bit_len);
        assert!(details.payload_bit_len <= unconstrained.payload_bit_len);
        assert_eq!(super::layer3_main_data_capacity_bits(header).unwrap(), 3168);
        assert_eq!(super::layer3_main_data_capacity_bytes(header).unwrap(), 396);
        assert_eq!(
            super::layer3_main_data_capacity_bytes(
                super::layer3_header_for_capacity(44_100, 2, 128, false, false).unwrap()
            )
            .unwrap(),
            381
        );
        assert_eq!(budgeted, selected);
        assert!(
            super::select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider(
                header,
                &pcm,
                0,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                0,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_psychoacoustic_scalefactor_helper() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2304]).unwrap();
        let scale_factors = super::select_mpeg1_layer3_psychoacoustic_long_scale_factors(
            &pcm, 0, 576, 0.05, false, 1024,
        )
        .unwrap();

        assert_eq!(
            scale_factors,
            [0_u8; super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT]
        );
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_perceptual_scalefactor_stream_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.02).sin() * 0.2)
                .collect(),
        )
        .unwrap();
        let header = super::layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let candidates = [0.05_f32, 0.1, 0.2];
        let selected =
            super::select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let active_selected = super::select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider(
                header,
                &pcm,
                0,
                &candidates,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let profile_candidates = [0.05_f32, 0.1, 0.2, 1.0];
        let candidate_profile =
            super::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
                &pcm,
                &profile_candidates,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let low_band_profile =
            super::select_mpeg1_layer3_first_frame_low_band_spectral_shape_candidate_profile_with_table_provider(
                &pcm,
                &profile_candidates,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let band_shape_profile =
            super::select_mpeg1_layer3_first_frame_band_spectral_shape_candidate_profile_with_table_provider(
                &pcm,
                &profile_candidates,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let bit_allocation =
            super::select_mpeg1_layer3_perceptual_bit_allocation_with_bitrate(&pcm, 128, false, 0)
                .unwrap();
        let encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider(
                &pcm,
                0.1,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let scalefac_scale_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_scalefac_scale_and_table_provider(
                &pcm,
                0.1,
                true,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let allowed_noise_scaled =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_allowed_noise_scale_and_table_provider(
                &pcm,
                0.1,
                0.5,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let budgeted =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_max_payload_bits_and_table_provider(
                &pcm,
                &candidates,
                selected.payload_bit_len,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let bitrate_header =
            super::layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();
        let bitrate_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let cbr_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_cbr_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let active_cbr_encoded =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                &candidates,
                96,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();

        assert!(active_selected.payload_bit_len <= active_selected.frame_capacity_bits);
        assert_eq!(candidate_profile.len(), profile_candidates.len());
        assert!(candidate_profile.iter().all(|profile| {
            profile.scale_factor_bands == super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT * 2
        }));
        assert!(candidate_profile
            .iter()
            .any(|profile| profile.nonzero_scale_factors > 0));
        assert_eq!(low_band_profile.len(), profile_candidates.len());
        assert!(low_band_profile.iter().all(|profile| {
            profile.low_band_abs_sum <= profile.total_abs_sum
                && profile.low_band_nonzero_lines <= profile.total_nonzero_lines
        }));
        assert!(low_band_profile
            .iter()
            .any(|profile| profile.low_band_nonzero_lines > 0));
        assert_eq!(
            band_shape_profile.len(),
            profile_candidates.len() * super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
        );
        assert!(band_shape_profile.iter().all(|profile| {
            profile.band < super::MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT
                && profile.band_start <= profile.band_end
                && profile.band_abs_sum <= profile.total_abs_sum
                && profile.band_nonzero_lines <= profile.total_nonzero_lines
        }));
        assert!(band_shape_profile
            .iter()
            .any(|profile| profile.band_nonzero_lines > 0));
        assert_eq!(bit_allocation.len(), 2);
        assert_eq!(
            bit_allocation
                .iter()
                .map(|allocation| allocation.target_bits)
                .sum::<usize>(),
            super::layer3_main_data_capacity_bits(header).unwrap()
        );
        assert!(bit_allocation
            .iter()
            .all(|allocation| allocation.perceptual_entropy.is_finite()));
        assert_eq!(encoded.len(), header.frame_len());
        assert_eq!(budgeted.len(), header.frame_len());
        assert_eq!(bitrate_encoded.len(), bitrate_header.frame_len());
        assert_eq!(cbr_encoded.len(), bitrate_header.frame_len());
        assert_eq!(active_cbr_encoded.len(), bitrate_header.frame_len());
        assert_eq!(super::detect(&encoded), Some(Format::Mp3));
        assert_eq!(scalefac_scale_encoded.len(), header.frame_len());
        assert_eq!(super::detect(&scalefac_scale_encoded), Some(Format::Mp3));
        assert_eq!(allowed_noise_scaled.len(), header.frame_len());
        assert_eq!(super::detect(&allowed_noise_scaled), Some(Format::Mp3));
        assert_eq!(super::detect(&budgeted), Some(Format::Mp3));
        assert_eq!(
            super::FrameHeader::parse(&bitrate_encoded[..4]).unwrap(),
            bitrate_header
        );

        let cbr_pcm = AudioBuffer::new(
            44_100,
            1,
            (0..(1152 * 3))
                .map(|sample| ((sample as f32) * 0.013).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let active_cbr_128 =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &cbr_pcm,
                &candidates,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let first_header = super::FrameHeader::parse(&active_cbr_128[..4]).unwrap();
        let second_offset = first_header.frame_len();
        let second_header =
            super::FrameHeader::parse(&active_cbr_128[second_offset..second_offset + 4]).unwrap();
        let third_offset = second_offset + second_header.frame_len();
        let third_header =
            super::FrameHeader::parse(&active_cbr_128[third_offset..third_offset + 4]).unwrap();
        assert_eq!(first_header.frame_len(), 417);
        assert_eq!(second_header.frame_len(), 418);
        assert_eq!(third_header.frame_len(), 418);
        assert_eq!(active_cbr_128.len(), 1253);

        let reservoir_pcm = AudioBuffer::new(
            44_100,
            1,
            (0..(1152 * 8))
                .map(|sample| {
                    let t = sample as f32;
                    if sample / 1152 % 2 == 0 {
                        0.24 * ((t * 0.043).sin()
                            + 0.7 * (t * 0.131).sin()
                            + 0.4 * (t * 0.277).sin())
                    } else {
                        0.02 * (t * 0.05).sin()
                    }
                })
                .collect(),
        )
        .unwrap();
        let perceptual_reservoir_details =
            super::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
                &reservoir_pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let entropy_targeted_details =
            super::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
                &reservoir_pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                0,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let perceptual_reservoir =
            super::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
                &reservoir_pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                super::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        assert_eq!(perceptual_reservoir_details.len(), 8);
        assert_eq!(
            entropy_targeted_details.len(),
            perceptual_reservoir_details.len()
        );
        assert_eq!(
            entropy_targeted_details
                .iter()
                .map(|detail| detail.entropy_target_bits)
                .sum::<usize>(),
            perceptual_reservoir_details
                .iter()
                .map(|detail| detail.frame_capacity_bytes * 8)
                .sum::<usize>()
        );
        assert!(entropy_targeted_details
            .iter()
            .any(|detail| detail.used_entropy_target_budget));
        assert!(perceptual_reservoir_details
            .iter()
            .any(|detail| detail.main_data_begin > 0));
        assert_eq!(super::detect(&perceptual_reservoir), Some(Format::Mp3));
    }

    #[test]
    #[cfg(feature = "mp3")]
    fn exposes_mp3_pcm_bitrate_helper() {
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..1152)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let provider = super::mpeg1_layer3_standard_table_provider();
        let header = super::layer3_header_for_capacity(44_100, 1, 96, false, false).unwrap();

        let encoded = super::encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
            &pcm,
            super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            96,
            false,
            false,
            provider,
        )
        .unwrap();
        let parsed = super::FrameHeader::parse(&encoded[..4]).unwrap();

        assert_eq!(parsed, header);
        assert_eq!(parsed.bitrate_kbps, 96);
        assert_eq!(encoded.len(), header.frame_len());
        assert!(
            super::encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider(
                &pcm,
                super::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                123,
                false,
                false,
                provider,
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_adts_to_m4a_mux() {
        let adts =
            super::frame_aac_adts(super::AdtsConfig::aac_lc(44_100, 2), &[0x11, 0x22]).unwrap();
        let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();
        let demuxed = super::demux_m4a_as_aac_adts(&m4a).unwrap();

        assert_eq!(&m4a[4..8], b"ftyp");
        assert!(m4a.windows(4).any(|window| window == b"mdat"));
        assert_eq!(demuxed, adts);
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_pcm_scale_factor_stream_helper() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 2048]).unwrap();
        let scale_factors_by_frame: [&[i16]; 2] = [&[0], &[0]];
        let selected =
            super::select_scale_factors_for_quantized_bands(&[0, 0, 1, -1], 2, 100).unwrap();
        let offsets = [0, 2, 4];
        let selected_by_offsets = super::select_scale_factors_for_quantized_bands_by_offsets(
            &[0, 0, 1, -1],
            &offsets,
            100,
        )
        .unwrap();
        let biased_by_offsets =
            super::select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
                &[0, 0, 1, -1],
                &offsets,
                100,
                2,
            )
            .unwrap();
        let quantized_adts = super::encode_quantized_mono_adts_with_selected_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacLongBlockConfig::new(0, 1),
            &[0, 0],
            2,
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected_pcm_adts = super::encode_pcm_mono_long_block_adts_with_selected_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacLongBlockConfig::new(0, 1),
            &pcm,
            super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();
        let selected_stream_adts =
            super::encode_pcm_mono_long_block_adts_stream_with_selected_scale_factors(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacLongBlockConfig::new(0, 1),
                &pcm,
                super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
                &[],
                super::AacSpectralMagnitudeTables::default(),
            )
            .unwrap();

        let adts = super::encode_pcm_mono_long_block_adts_stream_with_scale_factors(
            super::AdtsConfig::aac_lc(44_100, 1),
            super::AacScaleFactorSequence::new(
                super::AacLongBlockConfig::new(0, 1),
                &scale_factors_by_frame,
            ),
            &pcm,
            super::AacPcmLongBlockConfig::new(0, 1.0, 1024),
            &[],
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(&quantized_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&selected_pcm_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&selected_stream_adts[..2], &[0xff, 0xf1]);
        assert_eq!(adts.len(), 26);
        assert_eq!(selected_stream_adts.len(), 26);
        assert_eq!(selected, vec![100, 101]);
        assert_eq!(selected_by_offsets, vec![100, 101]);
        assert_eq!(biased_by_offsets, vec![100, 100]);
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_pcm_bitrate_budget_stream_helper() {
        fn max_adts_frame_len(stream: &[u8]) -> usize {
            let mut max_len = 0;
            let mut offset = 0;
            while offset + 7 <= stream.len() {
                let frame_len = (((stream[offset + 3] & 0x03) as usize) << 11)
                    | ((stream[offset + 4] as usize) << 3)
                    | ((stream[offset + 5] as usize) >> 5);
                assert!(frame_len >= 7);
                assert!(offset + frame_len <= stream.len());
                max_len = max_len.max(frame_len);
                offset += frame_len;
            }
            assert_eq!(offset, stream.len());
            max_len
        }

        let mono = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
                .collect(),
        )
        .unwrap();
        let mut stereo_samples = Vec::new();
        for sample in 0..2048 {
            stereo_samples.push(((sample as f32) * 0.01).sin() * 0.25);
            stereo_samples.push(((sample as f32) * 0.013).cos() * 0.20);
        }
        let stereo = AudioBuffer::new(44_100, 2, stereo_samples).unwrap();
        let offsets = super::aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let channel_config = super::AacLongBlockConfig::new(180, (offsets.len() - 1) as u8);
        let scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
        let channel = super::AacScaleFactorChannel::new(channel_config, &scale_factors);
        let scale_factor_table = super::aac_scale_factor_delta_zero_table();
        let spectral_tables = super::aac_unsigned_pairs7_unit_magnitude_spectral_tables();

        let mono_target_bitrate = 10_000;
        let mono_budget =
            super::aac_lc_adts_max_frame_len_for_bitrate(44_100, mono_target_bitrate).unwrap();
        let mono_adts =
            super::encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                channel,
                &mono,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                mono_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        let stereo_target_bitrate = 14_000;
        let stereo_budget =
            super::aac_lc_adts_max_frame_len_for_bitrate(44_100, stereo_target_bitrate).unwrap();
        let stereo_adts =
            super::encode_pcm_stereo_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                channel,
                channel,
                &stereo,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                stereo_target_bitrate,
                scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let mono_adts_high_level =
            super::encode_aac_adts_with_bitrate(&mono, mono_target_bitrate).unwrap();
        let stereo_adts_high_level =
            super::encode_aac_adts_with_bitrate(&stereo, stereo_target_bitrate).unwrap();
        let selected_scale_factor_table = super::aac_scale_factor_delta_table();
        let selected_mono_target_bitrate = 128_000;
        let selected_stereo_target_bitrate = 256_000;
        let selected_mono_adts = super::encode_aac_adts_with_selected_scale_factors_and_bitrate(
            &mono,
            selected_mono_target_bitrate,
        )
        .unwrap();
        let selected_stereo_adts = super::encode_aac_adts_with_selected_scale_factors_and_bitrate(
            &stereo,
            selected_stereo_target_bitrate,
        )
        .unwrap();
        let selected_mono_details = super::aac_selected_scale_factor_frame_details_with_bitrate(
            &mono,
            selected_mono_target_bitrate,
        )
        .unwrap();
        let selected_stereo_details = super::aac_selected_scale_factor_frame_details_with_bitrate(
            &stereo,
            selected_stereo_target_bitrate,
        )
        .unwrap();
        let selected_mono_core_details =
            super::select_aac_lc_mono_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                channel_config,
                &mono,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                selected_mono_target_bitrate,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();
        let selected_stereo_core_details =
            super::select_aac_lc_stereo_pcm_stream_frame_details_with_offsets_and_selected_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                channel_config,
                channel_config,
                &stereo,
                offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                selected_stereo_target_bitrate,
                &selected_scale_factor_table,
                spectral_tables,
            )
            .unwrap();

        assert!(super::AAC_LC_PCM_STEP_CANDIDATES.contains(&0.2));
        assert!(!super::AAC_LC_PCM_STEP_CANDIDATES.contains(&0.15));
        assert!(super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES.contains(&0.15));
        assert!(super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES.contains(&0.075));
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_global_gain(1).unwrap(),
            128
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_global_gain(2).unwrap(),
            126
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_magnitude_bias(),
            16
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_parameters(1).unwrap(),
            (128, 16)
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_parameters(2).unwrap(),
            (126, 16)
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_balanced_parameters(1).unwrap(),
            (136, 8, 2047)
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_balanced_parameters(2).unwrap(),
            (138, 4, 1535)
        );
        let mono_balance_profile =
            super::aac_standard_id_selected_scale_factor_balance_profile(1).unwrap();
        let stereo_balance_profile =
            super::aac_standard_id_selected_scale_factor_balance_profile(2).unwrap();
        assert_eq!(mono_balance_profile.recommended_global_gain, 128);
        assert_eq!(mono_balance_profile.global_gain_deltas, &[0, 2, 4, 6, 8]);
        assert_eq!(mono_balance_profile.magnitude_biases, &[8, 12, 16, 20]);
        assert_eq!(mono_balance_profile.selected_global_gain, 136);
        assert_eq!(mono_balance_profile.selected_magnitude_bias, 8);
        assert_eq!(mono_balance_profile.max_quantized_abs, 2047);
        assert_eq!(stereo_balance_profile.recommended_global_gain, 126);
        assert_eq!(stereo_balance_profile.global_gain_deltas, &[8, 12, 16]);
        assert_eq!(stereo_balance_profile.magnitude_biases, &[4, 8, 12]);
        assert_eq!(stereo_balance_profile.selected_global_gain, 138);
        assert_eq!(stereo_balance_profile.selected_magnitude_bias, 4);
        assert_eq!(stereo_balance_profile.max_quantized_abs, 1535);
        let mono_quality_control_candidates =
            super::aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
                &mono,
                selected_mono_target_bitrate,
            )
            .unwrap();
        let stereo_quality_control_candidates =
            super::aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(
                &stereo,
                selected_stereo_target_bitrate,
            )
            .unwrap();
        assert_eq!(
            mono_quality_control_candidates.len(),
            mono_balance_profile.global_gain_deltas.len()
                * mono_balance_profile.magnitude_biases.len()
        );
        assert_eq!(
            stereo_quality_control_candidates.len(),
            stereo_balance_profile.global_gain_deltas.len()
                * stereo_balance_profile.magnitude_biases.len()
        );
        assert!(mono_quality_control_candidates.iter().all(|candidate| {
            candidate.profile.min_frame_budget_slack >= 0
                && candidate.profile.max_abs <= i32::try_from(candidate.max_quantized_abs).unwrap()
        }));
        assert!(stereo_quality_control_candidates.iter().all(|candidate| {
            candidate.profile.min_frame_budget_slack >= 0
                && candidate.profile.max_abs <= i32::try_from(candidate.max_quantized_abs).unwrap()
        }));
        assert!(mono_quality_control_candidates.iter().any(|candidate| {
            candidate.global_gain == mono_balance_profile.selected_global_gain
                && candidate.scale_factor_magnitude_bias
                    == mono_balance_profile.selected_magnitude_bias
                && candidate.max_quantized_abs == mono_balance_profile.max_quantized_abs
        }));
        assert!(stereo_quality_control_candidates.iter().any(|candidate| {
            candidate.global_gain == stereo_balance_profile.selected_global_gain
                && candidate.scale_factor_magnitude_bias
                    == stereo_balance_profile.selected_magnitude_bias
                && candidate.max_quantized_abs == stereo_balance_profile.max_quantized_abs
        }));
        assert!(super::aac_standard_id_selected_scale_factor_global_gain(3).is_err());
        assert!(super::aac_standard_id_selected_scale_factor_balance_profile(3).is_err());
        assert_eq!(&mono_adts[..2], &[0xff, 0xf1]);
        assert_eq!(&stereo_adts[..2], &[0xff, 0xf1]);
        assert_eq!(mono_adts_high_level, mono_adts);
        assert_eq!(stereo_adts_high_level, stereo_adts);
        assert_eq!(selected_mono_details, selected_mono_core_details);
        assert_eq!(selected_stereo_details, selected_stereo_core_details);
        assert_eq!(selected_mono_details.len(), 2);
        assert_eq!(selected_stereo_details.len(), 2);
        assert_eq!(
            selected_mono_details
                .iter()
                .map(|detail| detail.frame_len)
                .max()
                .unwrap(),
            max_adts_frame_len(&selected_mono_adts)
        );
        assert_eq!(
            selected_stereo_details
                .iter()
                .map(|detail| detail.frame_len)
                .max()
                .unwrap(),
            max_adts_frame_len(&selected_stereo_adts)
        );
        assert!(max_adts_frame_len(&mono_adts) <= mono_budget);
        assert!(max_adts_frame_len(&stereo_adts) <= stereo_budget);
        assert!(super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 1).is_err());
        assert!(super::encode_aac_adts_with_bitrate(&mono, 1).is_err());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs7_unit_magnitude_table() {
        let table = super::aac_unsigned_pairs7_unit_magnitude_table();
        assert_eq!(table.len(), 4);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0b0, 1).unwrap());
        assert_eq!(table[1].symbol, super::AacSpectralMagnitudePair::new(0, 1));
        assert_eq!(table[1].code, super::HuffmanCode::new(0b101, 3).unwrap());
        assert_eq!(table[2].symbol, super::AacSpectralMagnitudePair::new(1, 0));
        assert_eq!(table[2].code, super::HuffmanCode::new(0b100, 3).unwrap());
        assert_eq!(table[3].symbol, super::AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[3].code, super::HuffmanCode::new(0b1100, 4).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs7_table() {
        let table = super::aac_unsigned_pairs7_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(table[63].symbol, super::AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, super::HuffmanCode::new(0xfff, 12).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_signed_pairs5_and_6_tables() {
        let pairs5 = super::aac_signed_pairs5_table();
        let pairs6 = super::aac_signed_pairs6_table();

        assert_eq!(pairs5.len(), 81);
        assert_eq!(pairs5[40].symbol, super::AacSpectralPair::new(0, 0));
        assert_eq!(pairs5[40].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(pairs6.len(), 81);
        assert_eq!(pairs6[40].symbol, super::AacSpectralPair::new(0, 0));
        assert_eq!(pairs6[40].code, super::HuffmanCode::new(0, 4).unwrap());

        let tables = super::aac_lc_standard_signed_pair_tables();
        assert_eq!(tables.signed_pairs5.len(), 81);
        assert_eq!(tables.signed_pairs6.len(), 81);
        assert_eq!(
            super::pack_spectral_pairs_with_table(
                &[super::AacSpectralPair::new(1, -1)],
                tables.signed_pairs6,
            )
            .unwrap()
            .bit_len,
            4
        );
        assert_eq!(
            super::plan_aac_lc_standard_spectral_sections_by_bit_cost(&[0, 1], 2).unwrap(),
            vec![super::AacSpectralSection {
                start: 0,
                end: 2,
                codebook_id: 5,
            }]
        );
        assert_eq!(
            super::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(&[1, -1], 2)
                .unwrap()
                .spectral_bits
                .bit_len,
            4
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_quads3_and_4_tables() {
        let quads3 = super::aac_unsigned_quads3_table();
        let quads4 = super::aac_unsigned_quads4_table();

        assert_eq!(quads3.len(), 81);
        assert_eq!(
            quads3[40].symbol,
            super::AacSpectralMagnitudeQuad::new(1, 1, 1, 1)
        );
        assert_eq!(quads3[40].code, super::HuffmanCode::new(0x74, 7).unwrap());
        assert_eq!(quads4.len(), 81);
        assert_eq!(
            quads4[40].symbol,
            super::AacSpectralMagnitudeQuad::new(1, 1, 1, 1)
        );
        assert_eq!(quads4[40].code, super::HuffmanCode::new(0, 4).unwrap());

        let tables = super::aac_lc_standard_unsigned_quad_tables();
        assert_eq!(tables.quads3.len(), 81);
        assert_eq!(tables.quads4.len(), 81);
        assert_eq!(
            super::select_quad_codebook_by_bit_cost(&[1, -1, 1, -1], tables).unwrap(),
            4
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_signed_quads1_and_2_tables() {
        let quads1 = super::aac_signed_quads1_table();
        let quads2 = super::aac_signed_quads2_table();

        assert_eq!(quads1.len(), 81);
        assert_eq!(quads1[40].symbol, super::AacSpectralQuad::new(0, 0, 0, 0));
        assert_eq!(quads1[40].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(quads2.len(), 81);
        assert_eq!(quads2[40].symbol, super::AacSpectralQuad::new(0, 0, 0, 0));
        assert_eq!(quads2[40].code, super::HuffmanCode::new(0, 3).unwrap());

        let tables = super::aac_lc_standard_signed_quad_tables();
        assert_eq!(tables.quads1.len(), 81);
        assert_eq!(tables.quads2.len(), 81);
        assert_eq!(
            super::plan_aac_lc_standard_spectral_sections_by_bit_cost(&[1, -1, 1, -1], 4).unwrap()
                [0]
            .codebook_id,
            2
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_unsigned_pairs8_table() {
        let table = super::aac_unsigned_pairs8_table();

        assert_eq!(table.len(), 64);
        assert_eq!(table[0].symbol, super::AacSpectralMagnitudePair::new(0, 0));
        assert_eq!(table[0].code, super::HuffmanCode::new(0x00e, 5).unwrap());
        assert_eq!(table[9].symbol, super::AacSpectralMagnitudePair::new(1, 1));
        assert_eq!(table[9].code, super::HuffmanCode::new(0, 3).unwrap());
        assert_eq!(table[63].symbol, super::AacSpectralMagnitudePair::new(7, 7));
        assert_eq!(table[63].code, super::HuffmanCode::new(0x3ff, 10).unwrap());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_scale_factor_delta_table() {
        let table = super::aac_scale_factor_delta_table();

        assert_eq!(table.len(), 121);
        assert_eq!(table[0].symbol, super::AacScaleFactorDelta::new(-60));
        assert_eq!(table[60].symbol, super::AacScaleFactorDelta::new(0));
        assert_eq!(table[60].code, super::HuffmanCode::new(0, 1).unwrap());
        assert_eq!(table[120].symbol, super::AacScaleFactorDelta::new(60));
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_spectral_quad_helpers() {
        let quads = [super::AacSpectralQuad::new(1, -1, 0, 1)];
        let sections = [super::AacQuadSection {
            start: 0,
            end: 4,
            codebook_id: 2,
        }];
        let quantized = [1, -1, 0, 1];
        let signed_table = [super::HuffmanEntry {
            symbol: quads[0],
            code: super::HuffmanCode::new(0b11, 2).unwrap(),
        }];
        let magnitude_table = [super::HuffmanEntry {
            symbol: super::AacSpectralMagnitudeQuad::new(1, 1, 0, 1),
            code: super::HuffmanCode::new(0b10, 2).unwrap(),
        }];

        assert_eq!(
            super::pack_spectral_quads_with_table(&quads, &signed_table)
                .unwrap()
                .bit_len,
            2
        );
        assert_eq!(
            super::pack_spectral_quads_with_sign_bits(&quads, &magnitude_table)
                .unwrap()
                .bit_len,
            5
        );
        let tables = super::AacSpectralMagnitudeQuadTables {
            quads2: &magnitude_table,
            ..Default::default()
        };
        let unit_pair_tables = super::aac_unit_codebook6_spectral_tables();
        let unit_quad_tables = super::aac_unit_quad_spectral_tables();
        assert_eq!(
            super::select_quad_codebook_by_bit_cost(&quantized, tables).unwrap(),
            2
        );
        assert_eq!(
            super::plan_quad_sections_by_bit_cost(&quantized, 4, tables).unwrap(),
            sections
        );
        assert_eq!(
            super::pack_quad_section_data_with_len(&sections, 4)
                .unwrap()
                .bit_len,
            9
        );
        assert_eq!(
            super::pack_spectral_quad_sections_with_sign_bits(&sections, &quantized, tables)
                .unwrap()
                .bit_len,
            5
        );
        assert_eq!(
            super::pack_sectioned_spectral_quad_payload_with_sign_bits(
                &sections, &quantized, 4, tables,
            )
            .unwrap()
            .bit_len,
            14
        );
        assert_eq!(
            super::pack_sectioned_spectral_quad_payload_with_sign_bits_by_bit_cost(
                &quantized, 4, tables,
            )
            .unwrap()
            .bit_len,
            14
        );
        assert_eq!(unit_pair_tables.pairs6.len(), 1);
        assert_eq!(unit_quad_tables.quads1.len(), 2);
        assert_eq!(unit_quad_tables.quads3.len(), 2);
        assert_eq!(
            super::plan_spectral_sections_by_bit_cost(
                &[1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0],
                4,
                unit_pair_tables,
                unit_quad_tables,
            )
            .unwrap(),
            vec![
                super::AacSpectralSection {
                    start: 0,
                    end: 4,
                    codebook_id: 3,
                },
                super::AacSpectralSection {
                    start: 4,
                    end: 8,
                    codebook_id: 6,
                },
                super::AacSpectralSection {
                    start: 8,
                    end: 12,
                    codebook_id: 0,
                },
            ]
        );
        assert_eq!(
            super::plan_aac_lc_standard_spectral_sections_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                4
            )
            .unwrap(),
            vec![
                super::AacSpectralSection {
                    start: 0,
                    end: 4,
                    codebook_id: 4,
                },
                super::AacSpectralSection {
                    start: 4,
                    end: 8,
                    codebook_id: 11,
                },
            ]
        );
        assert_eq!(
            super::pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                4
            )
            .unwrap()
            .bit_len,
            44
        );
        let standard_split =
            super::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                4,
            )
            .unwrap();
        assert_eq!(standard_split.section_and_scale_factor_bits.bit_len, 18);
        assert_eq!(standard_split.spectral_bits.bit_len, 26);
        assert_eq!(
            super::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8]
            )
            .unwrap(),
            vec![
                super::AacSpectralSection {
                    start: 0,
                    end: 4,
                    codebook_id: 4,
                },
                super::AacSpectralSection {
                    start: 4,
                    end: 8,
                    codebook_id: 11,
                },
            ]
        );
        assert_eq!(
            super::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8]
            )
            .unwrap()
            .bit_len,
            44
        );
        let standard_offsets_split =
            super::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
            )
            .unwrap();
        assert_eq!(
            standard_offsets_split.section_and_scale_factor_bits.bit_len,
            18
        );
        assert_eq!(standard_offsets_split.spectral_bits.bit_len, 26);
        let standard_offsets_sections =
            super::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
            )
            .unwrap();
        assert_eq!(
            super::plan_spectral_scale_factor_deltas_by_offsets(
                &standard_offsets_sections,
                &[0, 4, 8],
                &[100, 100],
                100
            )
            .unwrap(),
            vec![
                super::AacScaleFactorDelta::new(0),
                super::AacScaleFactorDelta::new(0)
            ]
        );
        let standard_adts =
            super::encode_quantized_mono_adts_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacLongBlockConfig::new(100, 2),
                &[1, -1, 0, 1, 17, 0, 0, 0],
                &[0, 4, 8],
                &[100, 100],
                super::aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert_eq!(&standard_adts[..2], &[0xff, 0xf1]);
        let pcm = AudioBuffer::new(
            44_100,
            1,
            (0..2048)
                .map(|sample| ((sample as f32) * 0.02).sin() * 0.2)
                .collect(),
        )
        .unwrap();
        let long_offsets = super::aac_lc_long_window_scale_factor_band_offsets(44_100).unwrap();
        let max_sfb = long_offsets.len() - 1;
        let scale_frame0 = vec![128_i16; max_sfb];
        let scale_frame1 = vec![128_i16; max_sfb];
        let scale_frames: [&[i16]; 2] = [&scale_frame0, &scale_frame1];
        let max_adts_frame_len = |stream: &[u8]| -> usize {
            let mut offset = 0usize;
            let mut max_frame_len = 0usize;
            while offset < stream.len() {
                let frame_len = (usize::from(stream[offset + 3] & 0x03) << 11)
                    | (usize::from(stream[offset + 4]) << 3)
                    | usize::from(stream[offset + 5] >> 5);
                max_frame_len = max_frame_len.max(frame_len);
                offset += frame_len;
            }
            assert_eq!(offset, stream.len());
            max_frame_len
        };
        let standard_stream =
            super::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                &pcm,
                0,
                0.005,
                long_offsets,
                super::aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        let mut offset = 0usize;
        let mut frame_count = 0usize;
        while offset < standard_stream.len() {
            assert_eq!(standard_stream[offset], 0xff);
            assert_eq!(standard_stream[offset + 1] & 0xf0, 0xf0);
            let frame_len = (usize::from(standard_stream[offset + 3] & 0x03) << 11)
                | (usize::from(standard_stream[offset + 4]) << 3)
                | usize::from(standard_stream[offset + 5] >> 5);
            offset += frame_len;
            frame_count += 1;
        }
        assert_eq!(frame_count, 2);
        assert_eq!(offset, standard_stream.len());
        let standard_bitrate_stream =
            super::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                &pcm,
                0,
                long_offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                128_000,
                super::aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert!(
            max_adts_frame_len(&standard_bitrate_stream)
                <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap()
        );
        let high_level_standard_bitrate_stream =
            super::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(&pcm, 128_000, 128)
                .unwrap();
        assert_eq!(&high_level_standard_bitrate_stream[..2], &[0xff, 0xf1]);
        assert!(
            max_adts_frame_len(&high_level_standard_bitrate_stream)
                <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap()
        );
        let high_level_standard_m4a =
            super::encode_m4a_with_standard_spectral_offsets_and_bitrate(&pcm, 128_000, 128)
                .unwrap();
        assert_eq!(&high_level_standard_m4a[4..8], b"ftyp");
        let high_level_selected_standard_bitrate_stream =
            super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                &pcm, 128_000, 128, 16,
            )
            .unwrap();
        let core_selected_standard_bitrate_stream =
            super::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacLongBlockConfig::new(128, max_sfb as u8),
                &pcm,
                0,
                long_offsets,
                16,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                128_000,
                &super::aac_scale_factor_delta_table(),
            )
            .unwrap();
        assert_eq!(
            high_level_selected_standard_bitrate_stream,
            core_selected_standard_bitrate_stream
        );
        let high_level_selected_standard_details =
            super::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                &pcm, 128_000, 128, 16,
            )
            .unwrap();
        let core_selected_standard_details =
            super::select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacLongBlockConfig::new(128, max_sfb as u8),
                &pcm,
                0,
                long_offsets,
                16,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                128_000,
                &super::aac_scale_factor_delta_table(),
            )
            .unwrap();
        assert_eq!(
            high_level_selected_standard_details,
            core_selected_standard_details
        );
        assert_eq!(
            high_level_selected_standard_details
                .iter()
                .map(|selection| selection.frame_len)
                .max(),
            Some(max_adts_frame_len(
                &high_level_selected_standard_bitrate_stream
            ))
        );
        assert!(
            max_adts_frame_len(&high_level_selected_standard_bitrate_stream)
                <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap()
        );
        let high_level_selected_standard_m4a =
            super::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                &pcm, 128_000, 128, 16,
            )
            .unwrap();
        assert_eq!(&high_level_selected_standard_m4a[4..8], b"ftyp");
        let recommended_selected_standard_bitrate_stream =
            super::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                &pcm, 128_000,
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_bitrate_stream,
            high_level_selected_standard_bitrate_stream
        );
        let recommended_selected_standard_details =
            super::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
                &pcm, 128_000,
            )
            .unwrap();
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(1).unwrap(),
            super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS
        );
        assert_eq!(
            super::aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(2).unwrap(),
            super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS
        );
        assert_eq!(
            recommended_selected_standard_details,
            high_level_selected_standard_details
        );
        let recommended_selected_profile =
            super::aac_recommended_standard_selected_scale_factor_profile_for_frame_details(
                &pcm,
                &recommended_selected_standard_details,
            )
            .unwrap();
        let expected_recommended_selected_profile =
            super::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
                &pcm,
                &recommended_selected_standard_details,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS,
            )
            .unwrap();
        assert_eq!(
            recommended_selected_profile,
            expected_recommended_selected_profile
        );
        assert_eq!(recommended_selected_profile.frames, 2);
        assert_eq!(recommended_selected_profile.channels, 1);
        assert_eq!(recommended_selected_profile.bands, 2 * max_sfb);
        assert!(recommended_selected_profile.mean_delta.is_finite());
        let recommended_payload_breakdown =
            super::aac_recommended_standard_id_payload_breakdown_for_frame_details(
                &pcm,
                &recommended_selected_standard_details,
            )
            .unwrap();
        let expected_recommended_payload_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
                &pcm,
                &recommended_selected_standard_details,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS,
            )
            .unwrap();
        assert_eq!(
            recommended_payload_breakdown,
            expected_recommended_payload_breakdown
        );
        assert_eq!(recommended_payload_breakdown.frames, 2);
        assert_eq!(recommended_payload_breakdown.channels, 1);
        assert!(recommended_payload_breakdown.sections > 0);
        assert!(recommended_payload_breakdown.spectral_bits > 0);
        assert!(
            recommended_payload_breakdown.total_bits()
                >= recommended_payload_breakdown.spectral_bits
        );
        let balanced_selected_standard_stream =
            super::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                &pcm, 128_000,
            )
            .unwrap();
        let expected_balanced_selected_standard_stream =
            super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
                &pcm,
                128_000,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_BALANCED_MAGNITUDE_BIAS,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS,
            )
            .unwrap();
        assert_eq!(
            balanced_selected_standard_stream,
            expected_balanced_selected_standard_stream
        );
        let balanced_selected_standard_details =
            super::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
                &pcm, 128_000,
            )
            .unwrap();
        let expected_balanced_selected_standard_details =
            super::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
                &pcm,
                128_000,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_BALANCED_MAGNITUDE_BIAS,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAX_QUANTIZED_ABS,
            )
            .unwrap();
        assert_eq!(
            balanced_selected_standard_details,
            expected_balanced_selected_standard_details
        );
        let balanced_selected_profile =
            super::aac_balanced_standard_selected_scale_factor_profile_for_frame_details(
                &pcm,
                &balanced_selected_standard_details,
            )
            .unwrap();
        let expected_balanced_selected_profile =
            super::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
                &pcm,
                &balanced_selected_standard_details,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIAS,
            )
            .unwrap();
        assert_eq!(
            balanced_selected_profile,
            expected_balanced_selected_profile
        );
        assert_eq!(balanced_selected_profile.frames, 2);
        assert_eq!(balanced_selected_profile.channels, 1);
        assert_eq!(balanced_selected_profile.bands, 2 * max_sfb);
        assert!(balanced_selected_profile.mean_delta.is_finite());
        let balanced_payload_breakdown =
            super::aac_balanced_standard_id_payload_breakdown_for_frame_details(
                &pcm,
                &balanced_selected_standard_details,
            )
            .unwrap();
        let expected_balanced_payload_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_details_with_magnitude_bias(
                &pcm,
                &balanced_selected_standard_details,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIAS,
            )
            .unwrap();
        assert_eq!(
            balanced_payload_breakdown,
            expected_balanced_payload_breakdown
        );
        assert_eq!(balanced_payload_breakdown.frames, 2);
        assert_eq!(balanced_payload_breakdown.channels, 1);
        assert!(balanced_payload_breakdown.sections > 0);
        assert!(balanced_payload_breakdown.spectral_bits > 0);
        let balanced_selected_standard_m4a =
            super::encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                &pcm, 128_000,
            )
            .unwrap();
        assert_eq!(
            super::demux_m4a_as_aac_adts(&balanced_selected_standard_m4a).unwrap(),
            balanced_selected_standard_stream
        );
        let high_level_selected_standard_limited_stream =
            super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
                &pcm, 128_000, 128, 16, 12,
            )
            .unwrap();
        let core_selected_standard_limited_stream =
            super::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacLongBlockConfig::new(128, max_sfb as u8),
                &pcm,
                0,
                long_offsets,
                16,
                super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
                12,
                128_000,
                &super::aac_scale_factor_delta_table(),
            )
            .unwrap();
        assert_eq!(
            high_level_selected_standard_limited_stream,
            core_selected_standard_limited_stream
        );
        let recommended_selected_standard_limited_stream =
            super::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
                &pcm, 128_000, 12,
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_limited_stream,
            high_level_selected_standard_limited_stream
        );
        let recommended_selected_standard_limited_details =
            super::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
                &pcm, 128_000, 12,
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_limited_details
                .iter()
                .map(|selection| selection.frame_len)
                .max(),
            Some(max_adts_frame_len(
                &recommended_selected_standard_limited_stream
            ))
        );
        let recommended_selected_standard_m4a =
            super::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                &pcm, 128_000,
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_m4a,
            high_level_selected_standard_m4a
        );
        let recommended_selected_standard_limited_m4a =
            super::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
                &pcm, 128_000, 12,
            )
            .unwrap();
        assert_eq!(&recommended_selected_standard_limited_m4a[4..8], b"ftyp");
        let standard_bitrate_details =
            super::select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 1),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                &pcm,
                0,
                long_offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                128_000,
                super::aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert_eq!(standard_bitrate_details.len(), 2);
        assert!(standard_bitrate_details.iter().all(|detail| {
            detail.frame_len
                <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 128_000).unwrap()
        }));
        let stereo_pcm = AudioBuffer::new(
            44_100,
            2,
            (0..2048)
                .flat_map(|sample| {
                    [
                        ((sample as f32) * 0.02).sin() * 0.2,
                        ((sample as f32) * 0.017).cos() * 0.18,
                    ]
                })
                .collect(),
        )
        .unwrap();
        let standard_stereo_stream =
            super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                &stereo_pcm,
                0,
                0.005,
                long_offsets,
                super::aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        let mut offset = 0usize;
        let mut stereo_frame_count = 0usize;
        while offset < standard_stereo_stream.len() {
            assert_eq!(standard_stereo_stream[offset], 0xff);
            assert_eq!(standard_stereo_stream[offset + 1] & 0xf0, 0xf0);
            let channels = ((standard_stereo_stream[offset + 2] & 0x01) << 2)
                | ((standard_stereo_stream[offset + 3] >> 6) & 0x03);
            assert_eq!(channels, 2);
            let frame_len = (usize::from(standard_stereo_stream[offset + 3] & 0x03) << 11)
                | (usize::from(standard_stereo_stream[offset + 4]) << 3)
                | usize::from(standard_stereo_stream[offset + 5] >> 5);
            offset += frame_len;
            stereo_frame_count += 1;
        }
        assert_eq!(stereo_frame_count, 2);
        assert_eq!(offset, standard_stereo_stream.len());
        let standard_stereo_bitrate_stream =
            super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                &stereo_pcm,
                0,
                long_offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                256_000,
                super::aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert!(
            max_adts_frame_len(&standard_stereo_bitrate_stream)
                <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap()
        );
        let standard_stereo_bitrate_details =
            super::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_scale_factors_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                super::AacScaleFactorSequence::new(
                    super::AacLongBlockConfig::new(128, max_sfb as u8),
                    &scale_frames,
                ),
                &stereo_pcm,
                0,
                long_offsets,
                super::AAC_LC_PCM_STEP_CANDIDATES,
                256_000,
                super::aac_scale_factor_delta_zero_table(),
            )
            .unwrap();
        assert_eq!(standard_stereo_bitrate_details.len(), 2);
        assert!(standard_stereo_bitrate_details.iter().all(|detail| {
            detail.frame_len
                <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap()
        }));
        let high_level_selected_standard_stereo_bitrate_stream =
            super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                &stereo_pcm, 256_000, 128, 16,
            )
            .unwrap();
        let core_selected_standard_stereo_bitrate_stream =
            super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                super::AacLongBlockConfig::new(128, max_sfb as u8),
                super::AacLongBlockConfig::new(128, max_sfb as u8),
                &stereo_pcm,
                0,
                long_offsets,
                16,
                super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
                256_000,
                &super::aac_scale_factor_delta_table(),
            )
            .unwrap();
        assert_eq!(
            high_level_selected_standard_stereo_bitrate_stream,
            core_selected_standard_stereo_bitrate_stream
        );
        let high_level_selected_standard_stereo_details =
            super::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                &stereo_pcm, 256_000, 128, 16,
            )
            .unwrap();
        let core_selected_standard_stereo_details =
            super::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                super::AacLongBlockConfig::new(128, max_sfb as u8),
                super::AacLongBlockConfig::new(128, max_sfb as u8),
                &stereo_pcm,
                0,
                long_offsets,
                16,
                super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
                256_000,
                &super::aac_scale_factor_delta_table(),
            )
            .unwrap();
        assert_eq!(
            high_level_selected_standard_stereo_details,
            core_selected_standard_stereo_details
        );
        let recommended_selected_standard_stereo_bitrate_stream =
            super::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                &stereo_pcm, 256_000,
            )
            .unwrap();
        let core_recommended_selected_standard_stereo_bitrate_stream =
            super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                super::AacLongBlockConfig::new(126, max_sfb as u8),
                super::AacLongBlockConfig::new(126, max_sfb as u8),
                &stereo_pcm,
                0,
                long_offsets,
                16,
                super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
                256_000,
                &super::aac_scale_factor_delta_table(),
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_stereo_bitrate_stream,
            core_recommended_selected_standard_stereo_bitrate_stream
        );
        let recommended_selected_standard_stereo_details =
            super::aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(
                &stereo_pcm,
                256_000,
            )
            .unwrap();
        let core_recommended_selected_standard_stereo_details =
            super::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                super::AacLongBlockConfig::new(126, max_sfb as u8),
                super::AacLongBlockConfig::new(126, max_sfb as u8),
                &stereo_pcm,
                0,
                long_offsets,
                16,
                super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
                256_000,
                &super::aac_scale_factor_delta_table(),
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_stereo_details,
            core_recommended_selected_standard_stereo_details
        );
        let recommended_selected_standard_stereo_profile =
            super::aac_recommended_standard_selected_scale_factor_profile_for_frame_details(
                &stereo_pcm,
                &recommended_selected_standard_stereo_details,
            )
            .unwrap();
        let expected_recommended_selected_standard_stereo_profile =
            super::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
                &stereo_pcm,
                &recommended_selected_standard_stereo_details,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MAGNITUDE_BIAS,
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_stereo_profile,
            expected_recommended_selected_standard_stereo_profile
        );
        assert_eq!(recommended_selected_standard_stereo_profile.frames, 2);
        assert_eq!(recommended_selected_standard_stereo_profile.channels, 2);
        assert_eq!(
            recommended_selected_standard_stereo_profile.bands,
            4 * max_sfb
        );
        assert!(recommended_selected_standard_stereo_profile
            .mean_delta
            .is_finite());
        let balanced_selected_standard_stereo_stream =
            super::encode_aac_adts_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(
                &stereo_pcm,
                256_000,
            )
            .unwrap();
        let expected_balanced_selected_standard_stereo_stream =
            super::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_max_quantized_abs_and_bitrate(
                &stereo_pcm,
                256_000,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS,
            )
            .unwrap();
        assert_eq!(
            balanced_selected_standard_stereo_stream,
            expected_balanced_selected_standard_stereo_stream
        );
        let balanced_selected_standard_stereo_details =
            super::aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(
                &stereo_pcm,
                256_000,
            )
            .unwrap();
        let expected_balanced_selected_standard_stereo_details =
            super::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(
                &stereo_pcm,
                256_000,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAX_QUANTIZED_ABS,
            )
            .unwrap();
        assert_eq!(
            balanced_selected_standard_stereo_details,
            expected_balanced_selected_standard_stereo_details
        );
        let balanced_selected_standard_stereo_profile =
            super::aac_balanced_standard_selected_scale_factor_profile_for_frame_details(
                &stereo_pcm,
                &balanced_selected_standard_stereo_details,
            )
            .unwrap();
        let expected_balanced_selected_standard_stereo_profile =
            super::aac_standard_selected_scale_factor_profile_for_frame_details_with_magnitude_bias(
                &stereo_pcm,
                &balanced_selected_standard_stereo_details,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GLOBAL_GAIN,
                super::AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIAS,
            )
            .unwrap();
        assert_eq!(
            balanced_selected_standard_stereo_profile,
            expected_balanced_selected_standard_stereo_profile
        );
        assert_eq!(balanced_selected_standard_stereo_profile.frames, 2);
        assert_eq!(balanced_selected_standard_stereo_profile.channels, 2);
        assert_eq!(balanced_selected_standard_stereo_profile.bands, 4 * max_sfb);
        assert!(balanced_selected_standard_stereo_profile
            .mean_delta
            .is_finite());
        let recommended_selected_standard_stereo_limited_stream =
            super::encode_aac_adts_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
                &stereo_pcm,
                256_000,
                12,
            )
            .unwrap();
        let core_recommended_selected_standard_stereo_limited_stream =
            super::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_quantized_abs_and_bitrate_by_bit_cost(
                super::AdtsConfig::aac_lc(44_100, 2),
                super::AacLongBlockConfig::new(126, max_sfb as u8),
                super::AacLongBlockConfig::new(126, max_sfb as u8),
                &stereo_pcm,
                0,
                long_offsets,
                16,
                super::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
                12,
                256_000,
                &super::aac_scale_factor_delta_table(),
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_stereo_limited_stream,
            core_recommended_selected_standard_stereo_limited_stream
        );
        let recommended_selected_standard_stereo_limited_details =
            super::aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(
                &stereo_pcm,
                256_000,
                12,
            )
            .unwrap();
        assert_eq!(
            recommended_selected_standard_stereo_limited_details
                .iter()
                .map(|selection| selection.frame_len)
                .max(),
            Some(max_adts_frame_len(
                &recommended_selected_standard_stereo_limited_stream
            ))
        );
        let recommended_selected_standard_stereo_limited_m4a =
            super::encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(
                &stereo_pcm,
                256_000,
                12,
            )
            .unwrap();
        assert_eq!(
            &recommended_selected_standard_stereo_limited_m4a[4..8],
            b"ftyp"
        );
        assert_eq!(
            high_level_selected_standard_stereo_details
                .iter()
                .map(|selection| selection.frame_len)
                .max(),
            Some(max_adts_frame_len(
                &high_level_selected_standard_stereo_bitrate_stream
            ))
        );
        assert!(
            max_adts_frame_len(&high_level_selected_standard_stereo_bitrate_stream)
                <= super::aac_lc_adts_max_frame_len_for_bitrate(44_100, 256_000).unwrap()
        );

        let mixed_quantized = [1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0];
        let pairs6 = [super::HuffmanEntry {
            symbol: super::AacSpectralMagnitudePair::new(1, 1),
            code: super::HuffmanCode::new(0b1, 1).unwrap(),
        }];
        let pair_tables = super::AacSpectralMagnitudeTables {
            pairs6: &pairs6,
            ..Default::default()
        };
        let quad_tables = super::AacSpectralMagnitudeQuadTables {
            quads3: &magnitude_table,
            ..Default::default()
        };
        let mixed_sections = vec![
            super::AacSpectralSection {
                start: 0,
                end: 4,
                codebook_id: 3,
            },
            super::AacSpectralSection {
                start: 4,
                end: 8,
                codebook_id: 6,
            },
            super::AacSpectralSection {
                start: 8,
                end: 12,
                codebook_id: 0,
            },
        ];
        assert_eq!(
            super::select_spectral_codebook_id_by_bit_cost(
                &mixed_quantized[..4],
                pair_tables,
                quad_tables,
            )
            .unwrap(),
            3
        );
        assert_eq!(
            super::plan_spectral_sections_by_bit_cost(
                &mixed_quantized,
                4,
                pair_tables,
                quad_tables,
            )
            .unwrap(),
            mixed_sections
        );
        assert_eq!(
            super::pack_spectral_section_data_with_len(&mixed_sections, 4)
                .unwrap()
                .bit_len,
            27
        );
        assert_eq!(
            super::pack_spectral_sections_by_codebook_id_with_sign_bits(
                &mixed_sections,
                &mixed_quantized,
                pair_tables,
                quad_tables,
            )
            .unwrap()
            .bit_len,
            11
        );
        assert_eq!(
            super::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost(
                &mixed_quantized,
                4,
                pair_tables,
                quad_tables,
            )
            .unwrap()
            .bit_len,
            38
        );
        assert_eq!(
            super::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits(
                &mixed_sections,
                &mixed_quantized,
                4,
                pair_tables,
                quad_tables,
            )
            .unwrap()
            .spectral_bits
            .bit_len,
            11
        );
        assert_eq!(
            super::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
                &mixed_sections,
                &mixed_quantized,
                4,
                super::PackedBits {
                    bytes: vec![0b1100_0000],
                    bit_len: 2,
                },
                pair_tables,
                quad_tables,
            )
            .unwrap()
            .bit_len,
            40
        );
        assert_eq!(
            super::pack_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost(
                &mixed_quantized,
                4,
                super::PackedBits {
                    bytes: vec![0b1100_0000],
                    bit_len: 2,
                },
                pair_tables,
                quad_tables,
            )
            .unwrap()
            .bit_len,
            40
        );
        assert_eq!(
            super::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits(
                &mixed_sections,
                &mixed_quantized,
                4,
                super::PackedBits {
                    bytes: vec![0b1100_0000],
                    bit_len: 2,
                },
                pair_tables,
                quad_tables,
            )
            .unwrap()
            .section_and_scale_factor_bits
            .bit_len,
            29
        );
        assert_eq!(
            super::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_by_bit_cost(
                &mixed_quantized,
                4,
                pair_tables,
                quad_tables,
            )
            .unwrap()
            .spectral_bits
            .bit_len,
            11
        );
        assert_eq!(
            super::split_sectioned_spectral_payload_by_codebook_id_with_sign_bits_and_scale_factor_bits_by_bit_cost(
                &mixed_quantized,
                4,
                super::PackedBits {
                    bytes: vec![0b1100_0000],
                    bit_len: 2,
                },
                pair_tables,
                quad_tables,
            )
            .unwrap()
            .section_and_scale_factor_bits
            .bit_len,
            29
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn exposes_aac_codebook7_section_planning() {
        let sections = super::plan_sections_by_bit_cost(
            &[1, -1, 0, 0],
            2,
            super::AacSpectralMagnitudeTables::default(),
        )
        .unwrap();

        assert_eq!(
            sections,
            vec![
                super::AacSection {
                    start: 0,
                    end: 2,
                    codebook: super::AacCodebook::UnsignedPairs8,
                },
                super::AacSection {
                    start: 2,
                    end: 4,
                    codebook: super::AacCodebook::Zero,
                },
            ]
        );
    }

    #[test]
    #[cfg(feature = "aac")]
    fn dispatches_silent_aac_encode_as_adts() {
        let pcm = AudioBuffer::new(44_100, 1, vec![0.0; 1024]).unwrap();

        let adts = encode(Format::Aac, &pcm).unwrap();
        let decoded = decode(&adts).unwrap();
        let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();
        let decoded_m4a = decode(&m4a).unwrap();
        let decoded_aac_helper = super::decode_aac(&adts).unwrap();
        let decoded_m4a_helper = super::decode_aac(&m4a).unwrap();

        assert_eq!(
            encode_with_mode(Format::Aac, &pcm, EncodeMode::ProductionOnly).unwrap(),
            adts
        );
        assert_eq!(&adts[..2], &[0xff, 0xf1]);
        assert_eq!(super::detect(&adts), Some(Format::Aac));
        assert_eq!(decoded.sample_rate, 44_100);
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded_m4a.sample_rate, 44_100);
        assert_eq!(decoded_m4a.channels, 1);
        assert_eq!(decoded_m4a.samples.len(), pcm.samples.len());
        assert_eq!(decoded_aac_helper.samples.len(), pcm.samples.len());
        assert_eq!(decoded_m4a_helper.samples.len(), pcm.samples.len());
    }

    #[test]
    #[cfg(feature = "aac")]
    fn dispatches_non_silent_aac_encode_as_adts_scaffold() {
        for (sample_rate, channels) in [
            (7_350, 1),
            (8_000, 1),
            (11_025, 1),
            (12_000, 1),
            (16_000, 1),
            (22_050, 1),
            (24_000, 1),
            (32_000, 1),
            (44_100, 1),
            (48_000, 1),
            (64_000, 1),
            (88_200, 1),
            (96_000, 1),
            (7_350, 2),
            (8_000, 2),
            (11_025, 2),
            (12_000, 2),
            (16_000, 2),
            (22_050, 2),
            (24_000, 2),
            (32_000, 2),
            (44_100, 2),
            (48_000, 2),
            (64_000, 2),
            (88_200, 2),
            (96_000, 2),
        ] {
            let mut samples = Vec::new();
            for frame in 0..2048 {
                for channel in 0..channels {
                    let phase = if channel == 0 { 0.01 } else { 0.013 };
                    samples.push(((frame as f32) * phase).sin() * 0.25);
                }
            }
            let pcm = AudioBuffer::new(sample_rate, channels, samples).unwrap();

            let adts = encode(Format::Aac, &pcm).unwrap();
            let production =
                encode_with_mode(Format::Aac, &pcm, EncodeMode::ProductionOnly).unwrap();
            let m4a = super::mux_aac_adts_as_m4a(&adts).unwrap();

            assert_eq!(&adts[..2], &[0xff, 0xf1]);
            assert_eq!(&production[..2], &[0xff, 0xf1]);
            assert_eq!(production, adts);
            assert_eq!(super::detect(&adts), Some(Format::Aac));
            assert!(adts.len() > 7);
            assert!(m4a.len() > adts.len());
        }
    }
}
