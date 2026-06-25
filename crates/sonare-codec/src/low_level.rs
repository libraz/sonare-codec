//! Low-level MP3 and AAC building blocks.
//!
//! These are the granular encoder primitives (quantizers, Huffman packers,
//! section planners, step selectors, table accessors, and the associated
//! config/result types) used to assemble the high-level [`crate::encode`] /
//! `encode_*` paths. Typical callers do not need anything here — reach for the
//! crate-root `encode`/`decode` functions instead. This module exists so the
//! large unstable surface stays discoverable without burying the high-level
//! entry points in the crate root.

// The umbrella's own AAC profile/breakdown diagnostics belong to the low-level
// surface as well.
#[cfg(feature = "aac")]
pub use crate::aac_breakdown::*;
#[cfg(feature = "aac")]
pub use crate::aac_profiles::*;

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

// AAC diagnostic profile / breakdown types (consumed by the aac_profiles and

// aac_breakdown low-level modules).
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
pub(crate) const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_GAIN_DELTAS: &[u8] =
    &[0, 2, 4, 6, 8];
#[cfg(feature = "aac")]
pub(crate) const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_GAIN_DELTAS: &[u8] =
    &[8, 12, 16];
#[cfg(feature = "aac")]
pub(crate) const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_MONO_BALANCED_MAGNITUDE_BIASES: &[i16] =
    &[8, 12, 16, 20];
#[cfg(feature = "aac")]
pub(crate) const AAC_STANDARD_ID_SELECTED_SCALE_FACTOR_STEREO_BALANCED_MAGNITUDE_BIASES: &[i16] =
    &[4, 8, 12];

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
