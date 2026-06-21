#![allow(unused_imports)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

struct RustPackage {
    package: &'static str,
    manifest: &'static str,
    package_before_first_publish: bool,
}

const RUST_PUBLISH_PACKAGES: &[RustPackage] = &[
    RustPackage {
        package: "sc-core",
        manifest: "crates/sc-core/Cargo.toml",
        package_before_first_publish: true,
    },
    RustPackage {
        package: "sc-mp4",
        manifest: "crates/sc-mp4/Cargo.toml",
        package_before_first_publish: false,
    },
    RustPackage {
        package: "sc-wav",
        manifest: "crates/sc-wav/Cargo.toml",
        package_before_first_publish: false,
    },
    RustPackage {
        package: "sc-flac",
        manifest: "crates/sc-flac/Cargo.toml",
        package_before_first_publish: false,
    },
    RustPackage {
        package: "sc-mp3",
        manifest: "crates/sc-mp3/Cargo.toml",
        package_before_first_publish: false,
    },
    RustPackage {
        package: "sc-vorbis",
        manifest: "crates/sc-vorbis/Cargo.toml",
        package_before_first_publish: false,
    },
    RustPackage {
        package: "sc-opus",
        manifest: "crates/sc-opus/Cargo.toml",
        package_before_first_publish: false,
    },
    RustPackage {
        package: "sonare-codec-decode",
        manifest: "crates/sc-decode/Cargo.toml",
        package_before_first_publish: false,
    },
    RustPackage {
        package: "sc-aac",
        manifest: "crates/sc-aac/Cargo.toml",
        package_before_first_publish: false,
    },
    RustPackage {
        package: "sonare-codec",
        manifest: "crates/sonare-codec/Cargo.toml",
        package_before_first_publish: false,
    },
];

const NPM_PACKAGE_NAME: &str = "@libraz/sonare-codec";
const PYTHON_PACKAGE_NAME: &str = "sonare-codec";
const RELEASE_VERSION: &str = "0.1.0";
const PROJECT_LICENSE: &str = "Apache-2.0";
const PROJECT_REPOSITORY: &str = "https://github.com/libraz/sonare-codec";
const REQUIRED_QA_TOOLS_ENV: &str = "SONARE_REQUIRED_QA_TOOLS";
const AAC_STANDARD_DIAGNOSTIC_GLOBAL_GAIN_CANDIDATES: &[u8] =
    &[104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144];
const AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_GLOBAL_GAIN: u8 = 128;
const AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_MAGNITUDE_BIAS: i16 = 16;
const AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES: &[u8] = &[
    124,
    126,
    AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_GLOBAL_GAIN,
    130,
    132,
];
const AAC_STANDARD_HIGH_LEVEL_FIXED_SURFACE_GLOBAL_GAIN_CANDIDATES: &[u8] =
    AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES;
const AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES: &[i16] = &[
    8,
    12,
    AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_MAGNITUDE_BIAS,
];
const AAC_STANDARD_DIAGNOSTIC_MIN_DECODED_RMS: f64 = 0.10;
const AAC_STANDARD_DIAGNOSTIC_MIN_CORRELATION: f64 = 0.50;
const AAC_STANDARD_ID_MAX_PRODUCTION_CORRELATION_GAP: f64 = 0.25;
const MP3_PERCEPTUAL_DIAGNOSTIC_MIN_DECODED_RMS: f64 = 0.10;
const MP3_PERCEPTUAL_DIAGNOSTIC_MIN_CORRELATION: f64 = 0.30;
const MP3_STEREO_PERCEPTUAL_RESERVOIR_MIN_CORRELATION: f64 = 0.49;
const MP3_PERCEPTUAL_RESERVOIR_MAX_PRODUCTION_CORRELATION_GAP: f64 = 0.10;
const MP3_PRODUCTION_MONO_MIN_CORRELATION: f64 = 0.30;
const MP3_PRODUCTION_STEREO_MIN_CORRELATION: f64 = 0.55;
const AAC_PRODUCTION_MIN_CORRELATION: f64 = 0.70;
const MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES: &[f32] =
    sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES;
const PUBLIC_BINDING_FUNCTIONS: &[&str] = &[
    "detect_format",
    "decode_audio",
    "decode_wav",
    "decode_flac",
    "decode_mp3",
    "decode_vorbis",
    "decode_opus",
    "decode_aac",
    "decode_m4a",
    "encode_audio",
    "encode_audio_production",
    "encode_wav",
    "encode_flac",
    "encode_vorbis",
    "encode_opus",
    "encode_mp3",
    "encode_mp3_with_bitrate",
    "encode_mp3_cbr_with_bitrate",
    "encode_mp3_perceptual_active_cbr_with_bitrate",
    "encode_mp3_perceptual_reservoir_with_bitrate",
    "encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate",
    "encode_mp3_perceptual_scale_factor_band_bias",
    "encode_mp3_perceptual_quantized_band_gain",
    "encode_mp3_perceptual_quantized_band_gain_global_gain_bias",
    "encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate",
    "mp3_reservoir_frame_details_with_bitrate",
    "mp3_perceptual_reservoir_frame_details_with_bitrate",
    "mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate",
    "mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate",
    "mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate",
    "encode_aac",
    "encode_aac_with_bitrate",
    "encode_aac_with_selected_scale_factors_and_bitrate",
    "encode_aac_with_standard_spectral_offsets_and_bitrate",
    "encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate",
    "encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate",
    "encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate",
    "encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate",
    "encode_aac_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate",
    "encode_m4a",
    "encode_m4a_with_bitrate",
    "encode_m4a_with_selected_scale_factors_and_bitrate",
    "encode_m4a_with_standard_spectral_offsets_and_bitrate",
    "encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate",
    "encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate",
    "encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate",
    "encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate",
    "encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate",
    "demux_m4a_as_aac_adts",
    "aac_lc_adts_max_frame_len_for_bitrate",
    "aac_lc_default_production_bitrate_bps",
    "aac_lc_pcm_step_candidates",
    "aac_standard_id_pcm_step_candidates",
    "aac_standard_id_selected_scale_factor_global_gain",
    "aac_standard_id_selected_scale_factor_magnitude_bias",
    "aac_standard_id_selected_scale_factor_balanced_max_quantized_abs",
    "aac_standard_id_selected_scale_factor_balanced_parameters",
    "aac_standard_id_selected_scale_factor_balanced_gain_deltas",
    "aac_standard_id_selected_scale_factor_balanced_magnitude_biases",
    "aac_standard_id_selected_scale_factor_parameters",
    "aac_unsigned_pairs7_unit_magnitude_table",
    "aac_unsigned_pairs7_table",
    "aac_signed_pairs5_table",
    "aac_signed_pairs6_table",
    "aac_signed_quads1_table",
    "aac_signed_quads2_table",
    "aac_unsigned_pairs8_table",
    "aac_unsigned_pairs9_table",
    "aac_unsigned_pairs10_table",
    "aac_unsigned_quads3_table",
    "aac_unsigned_quads4_table",
    "aac_escape_table",
    "aac_scale_factor_delta_table",
    "aac_codebook6_unit_section_plan",
    "aac_quad_unit_section_plan",
    "aac_mixed_unit_section_plan",
    "aac_mixed_unit_payload_bit_lengths",
    "aac_standard_unit_section_plan",
    "aac_standard_offsets_section_plan",
    "aac_standard_escape_payload_bit_lengths",
    "aac_standard_mixed_payload_bit_lengths",
    "aac_standard_mixed_offsets_payload_bit_lengths",
    "encode_aac_standard_mono_offsets_with_step",
    "encode_aac_standard_mono_offsets_with_bitrate",
    "aac_standard_mono_offsets_bitrate_frame_details",
    "encode_aac_standard_stereo_offsets_with_step",
    "encode_aac_standard_stereo_offsets_with_bitrate",
    "aac_standard_stereo_offsets_bitrate_frame_details",
    "aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate",
    "aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate",
    "aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate",
    "aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate",
    "aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate",
    "aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate",
    "aac_recommended_standard_selected_scale_factor_profile_with_bitrate",
    "aac_balanced_standard_selected_scale_factor_profile_with_bitrate",
    "aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate",
    "aac_recommended_standard_id_payload_breakdown_with_bitrate",
    "aac_balanced_standard_id_payload_breakdown_with_bitrate",
    "aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate",
    "aac_balanced_standard_id_quality_control_profile_with_bitrate",
    "aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate",
    "aac_selected_scale_factor_frame_details_with_bitrate",
    "mp3_layer3_main_data_capacity_bytes",
    "mp3_layer3_main_data_capacity_bits",
    "mp3_pcm_step_candidates",
    "mp3_production_pcm_step_candidates",
    "mp3_first_frame_perceptual_candidate_profile_with_bitrate",
    "mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate",
    "mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate",
    "mp3_first_frame_quality_guarded_candidate_profile_with_bitrate",
    "mp3_perceptual_bit_allocation_with_bitrate",
    "mp3_standard_big_value_table_selects",
    "mp3_missing_standard_big_value_table_selects",
    "mp3_standard_count1_table_selects",
];

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("artifact-check") => artifact_check(),
        Some("aac-standard-diagnostic") => aac_standard_diagnostic(),
        Some("gen-refs") => gen_refs(),
        Some("fuzz-smoke") => fuzz_smoke(),
        Some("name-check") => name_check(),
        Some("mp3-perceptual-diagnostic") => mp3_perceptual_diagnostic(),
        Some("oracle-smoke") => oracle_smoke(),
        Some("package-preflight") => package_preflight(),
        Some("publish-preflight") => publish_preflight(),
        Some("publish-readiness") => publish_readiness(),
        Some("publish-plan") => publish_plan(),
        Some("qa-check") => qa_check(),
        Some("ref-check") => ref_check(),
        Some("release-check") => release_check(),
        Some("size-report") => size_report(),
        Some("tool-check") => tool_check(),
        _ => {
            eprintln!(
                "usage: cargo xtask <aac-standard-diagnostic|artifact-check|gen-refs|fuzz-smoke|mp3-perceptual-diagnostic|name-check|oracle-smoke|package-preflight|publish-plan|publish-preflight|publish-readiness|qa-check|ref-check|release-check|size-report|tool-check>"
            );
            ExitCode::from(2)
        }
    }
}

mod commands;
use commands::*;
mod oracle;
use oracle::*;
mod checks;
use checks::*;
mod diagnostics_aac_surface;
use diagnostics_aac_surface::*;
mod diagnostics_aac_balance;
use diagnostics_aac_balance::*;
mod diagnostics_aac_quality;
use diagnostics_aac_quality::*;
mod diagnostics_mp3;
use diagnostics_mp3::*;
mod publish;
use publish::*;
mod qa;
use qa::*;
mod npm_pack;
use npm_pack::*;
mod package_outputs;
use package_outputs::*;
mod util;
use util::*;
mod tests;
