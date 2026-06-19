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
    "encode_mp3",
    "encode_mp3_with_bitrate",
    "encode_mp3_cbr_with_bitrate",
    "encode_mp3_perceptual_active_cbr_with_bitrate",
    "encode_mp3_perceptual_reservoir_with_bitrate",
    "encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate",
    "encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate",
    "mp3_reservoir_frame_details_with_bitrate",
    "mp3_perceptual_reservoir_frame_details_with_bitrate",
    "mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate",
    "mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate",
    "encode_aac",
    "encode_aac_with_bitrate",
    "encode_aac_with_selected_scale_factors_and_bitrate",
    "encode_aac_with_standard_spectral_offsets_and_bitrate",
    "encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate",
    "encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate",
    "encode_m4a",
    "encode_m4a_with_bitrate",
    "encode_m4a_with_selected_scale_factors_and_bitrate",
    "encode_m4a_with_standard_spectral_offsets_and_bitrate",
    "encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate",
    "encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate",
    "demux_m4a_as_aac_adts",
    "aac_lc_adts_max_frame_len_for_bitrate",
    "aac_lc_default_production_bitrate_bps",
    "aac_lc_pcm_step_candidates",
    "aac_standard_id_pcm_step_candidates",
    "aac_standard_id_selected_scale_factor_global_gain",
    "aac_standard_id_selected_scale_factor_magnitude_bias",
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
    "aac_selected_scale_factor_frame_details_with_bitrate",
    "mp3_layer3_main_data_capacity_bytes",
    "mp3_layer3_main_data_capacity_bits",
    "mp3_pcm_step_candidates",
    "mp3_first_frame_perceptual_candidate_profile_with_bitrate",
    "mp3_perceptual_bit_allocation_with_bitrate",
    "mp3_standard_big_value_table_selects",
    "mp3_missing_standard_big_value_table_selects",
    "mp3_standard_count1_table_selects",
];
const PYTHON_ONLY_BINDING_FUNCTIONS: &[&str] = &["encode_vorbis", "encode_opus"];

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

fn artifact_check() -> ExitCode {
    let checks = [
        Check::PackageMetadata,
        Check::WasmPackBuild,
        Check::WasmPackOutput,
        Check::NpmPackDryRun,
        Check::NpmPackOutput,
        Check::MaturinBuild,
        Check::PythonWheelOutput,
    ];

    for check in checks {
        if let Err(err) = run_check(check) {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn release_check() -> ExitCode {
    let checks = [
        Check::Cargo(&["fmt", "--check"]),
        Check::PackageMetadata,
        Check::Cargo(&["check", "--workspace", "--all-features"]),
        Check::Cargo(&["test", "--workspace"]),
        Check::Cargo(&[
            "clippy",
            "--workspace",
            "--all-features",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
        Check::Cargo(&["run", "-p", "xtask", "--", "ref-check"]),
        Check::Cargo(&["run", "-p", "xtask", "--", "fuzz-smoke"]),
        Check::Cargo(&[
            "check",
            "--manifest-path",
            "fuzz/Cargo.toml",
            "--bin",
            "wav_decode",
            "--bin",
            "flac_decode",
            "--bin",
            "aac_decode",
            "--bin",
            "vorbis_decode",
            "--bin",
            "opus_decode",
            "--bin",
            "m4a_demux",
            "--bin",
            "mp3_header",
        ]),
        Check::WasmTarget,
        Check::Deny(&["check", "licenses", "bans", "sources"]),
        Check::NpmPackDryRun,
    ];

    for check in checks {
        if let Err(err) = run_check(check) {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn name_check() -> ExitCode {
    match run_registry_name_check() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run_registry_name_check() -> Result<(), String> {
    for package in RUST_PUBLISH_PACKAGES {
        let label = format!("crates.io {}", package.package);
        let url = format!("https://crates.io/api/v1/crates/{}", package.package);
        check_registry_name(&label, &url)?;
    }

    let npm_url = format!(
        "https://registry.npmjs.org/{}",
        NPM_PACKAGE_NAME.replace('@', "%40").replace('/', "%2F")
    );
    check_registry_name(&format!("npm {NPM_PACKAGE_NAME}"), &npm_url)?;

    let pypi_url = format!("https://pypi.org/pypi/{PYTHON_PACKAGE_NAME}/json");
    check_registry_name(&format!("PyPI {PYTHON_PACKAGE_NAME}"), &pypi_url)?;

    Ok(())
}

fn package_preflight() -> ExitCode {
    let checks = [
        Check::ToolReadiness,
        Check::GitHead,
        Check::RegistryNamesIfRequested,
        Check::PackageMetadata,
        Check::Cargo(&["run", "-p", "xtask", "--", "qa-check"]),
        Check::PublishRustPackages,
        Check::WasmPackBuild,
        Check::WasmPackOutput,
        Check::NpmPackDryRun,
        Check::NpmPackOutput,
        Check::MaturinBuild,
        Check::PythonWheelOutput,
    ];

    for check in checks {
        if let Err(err) = run_check(check) {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn publish_preflight() -> ExitCode {
    let checks = [
        Check::ToolReadiness,
        Check::GitHead,
        Check::RegistryNames,
        Check::PackageMetadata,
        Check::Cargo(&["run", "-p", "xtask", "--", "qa-check"]),
        Check::PublishRustPackages,
        Check::WasmPackBuild,
        Check::WasmPackOutput,
        Check::NpmPackDryRun,
        Check::NpmPackOutput,
        Check::MaturinBuild,
        Check::PythonWheelOutput,
        Check::PublishReadiness,
    ];

    for check in checks {
        if let Err(err) = run_check(check) {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn qa_check() -> ExitCode {
    for check in [
        run_optional_nextest,
        run_optional_machete,
        run_optional_audit,
        run_optional_semver_checks,
        run_optional_miri,
        run_optional_coverage,
    ] {
        if let Err(err) = check() {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn publish_readiness() -> ExitCode {
    match run_publish_readiness_check() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run_publish_readiness_check() -> Result<(), String> {
    for check in [
        run_package_metadata_check,
        verify_production_lossy_encode_readiness,
        verify_diagnostic_lossy_encode_readiness,
    ] {
        check()?;
    }
    Ok(())
}

fn aac_standard_diagnostic() -> ExitCode {
    let Some(ffmpeg) = env::var_os("SONARE_FFMPEG") else {
        eprintln!(
            "aac-standard-diagnostic requires SONARE_FFMPEG=/path/to/ffmpeg for local black-box acceptance checks"
        );
        return ExitCode::FAILURE;
    };
    let out_dir = env::temp_dir().join(format!(
        "sonare-codec-aac-standard-diagnostic-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("failed to create {}: {err}", out_dir.display());
        return ExitCode::FAILURE;
    }

    let result = readiness_pcm(44_100, 1)
        .map_err(|err| format!("failed to build AAC standard diagnostic PCM: {err}"))
        .and_then(|pcm| standard_aac_lc_nonzero_encode_diagnostic(&ffmpeg, &pcm, &out_dir));

    match result {
        Ok(quality) => {
            if let Err(err) = fs::remove_dir_all(&out_dir) {
                eprintln!("failed to remove {}: {err}", out_dir.display());
                return ExitCode::FAILURE;
            }
            eprintln!(
                "AAC-LC standard-table diagnostic: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("AAC-LC standard-table diagnostic is not production-ready: {err}");
            eprintln!(
                "AAC standard diagnostic artifact kept at {}",
                out_dir.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn mp3_perceptual_diagnostic() -> ExitCode {
    let Some(ffmpeg) = env::var_os("SONARE_FFMPEG") else {
        eprintln!(
            "mp3-perceptual-diagnostic requires SONARE_FFMPEG=/path/to/ffmpeg for local black-box acceptance checks"
        );
        return ExitCode::FAILURE;
    };
    let out_dir = env::temp_dir().join(format!(
        "sonare-codec-mp3-perceptual-diagnostic-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("failed to create {}: {err}", out_dir.display());
        return ExitCode::FAILURE;
    }

    let result = readiness_pcm(44_100, 1)
        .map_err(|err| format!("failed to build MP3 perceptual diagnostic PCM: {err}"))
        .and_then(|pcm| mp3_perceptual_nonzero_encode_diagnostic(&ffmpeg, &pcm, &out_dir));

    match result {
        Ok(quality) => {
            if let Err(err) = fs::remove_dir_all(&out_dir) {
                eprintln!("failed to remove {}: {err}", out_dir.display());
                return ExitCode::FAILURE;
            }
            eprintln!(
                "MP3 perceptual-scale-factor diagnostic: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            eprintln!(
                "MP3 perceptual diagnostic artifact kept at {}",
                out_dir.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn publish_plan() -> ExitCode {
    eprintln!("publish plan for version {RELEASE_VERSION}");
    eprintln!();
    eprintln!("preflight");
    eprintln!(
        "  {REQUIRED_QA_TOOLS_ENV}=nextest,audit,machete,semver-checks cargo run -p xtask -- publish-preflight"
    );
    eprintln!("  cargo run -p xtask -- artifact-check");
    eprintln!("  cargo run -p xtask -- size-report");
    eprintln!();
    eprintln!("rust crates");
    for (index, package) in RUST_PUBLISH_PACKAGES.iter().enumerate() {
        if !package.package_before_first_publish {
            eprintln!("  {}a. cargo package -p {}", index + 1, package.package);
            eprintln!("  {}b. cargo publish -p {}", index + 1, package.package);
        } else {
            eprintln!("  {}. cargo publish -p {}", index + 1, package.package);
        }
    }
    eprintln!();
    eprintln!("npm");
    eprintln!("  cargo run -p xtask -- artifact-check");
    eprintln!("  cd bindings/wasm");
    eprintln!("  npm publish --access public");
    eprintln!();
    eprintln!("pypi");
    eprintln!("  cargo run -p xtask -- artifact-check");
    eprintln!("  cd bindings/python");
    eprintln!("  python -m maturin publish");
    eprintln!();
    eprintln!("post-publish");
    eprintln!("  cargo run -p xtask -- size-report");
    ExitCode::SUCCESS
}

fn gen_refs() -> ExitCode {
    let out_dir = Path::new("tests/refs/oracle-smoke");
    if let Err(err) = fs::remove_dir_all(out_dir) {
        if err.kind() != std::io::ErrorKind::NotFound {
            eprintln!("failed to remove {}: {err}", out_dir.display());
            return ExitCode::FAILURE;
        }
    }
    if let Err(err) = fs::create_dir_all(out_dir) {
        eprintln!("failed to create {}: {err}", out_dir.display());
        return ExitCode::FAILURE;
    }

    let artifacts = match generate_oracle_smoke_artifacts(out_dir) {
        Ok(artifacts) => artifacts,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    let ffmpeg = env::var_os("SONARE_FFMPEG");
    let manifest = match build_reference_manifest(&artifacts, ffmpeg.as_deref()) {
        Ok(manifest) => manifest,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    let manifest_path = out_dir.join("manifest.json");
    if let Err(err) = fs::write(&manifest_path, manifest) {
        eprintln!("failed to write {}: {err}", manifest_path.display());
        return ExitCode::FAILURE;
    }

    eprintln!("wrote {}", manifest_path.display());
    if ffmpeg.is_none() {
        eprintln!("reference manifest has no FFmpeg acceptance data; set SONARE_FFMPEG=/path/to/ffmpeg to capture local oracle acceptance");
    }
    ExitCode::SUCCESS
}

fn ref_check() -> ExitCode {
    match verify_refs() {
        Ok(()) => {
            eprintln!("reference artifacts match current encoder output");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn size_report() -> ExitCode {
    match collect_size_report() {
        Ok(entries) => {
            eprintln!("artifact size report");
            for entry in entries {
                match entry.bytes {
                    Some(bytes) => eprintln!(
                        "{:<18} {:>12}  {}",
                        entry.kind,
                        human_bytes(bytes),
                        entry.path.display()
                    ),
                    None => eprintln!(
                        "{:<18} {:>12}  {}",
                        entry.kind,
                        "missing",
                        entry.path.display()
                    ),
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn tool_check() -> ExitCode {
    match run_tool_readiness_check() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}

fn run_tool_readiness_check() -> Result<(), ()> {
    eprintln!("tool readiness");
    let checks = [
        ToolCheck::command("git", &["rev-parse", "--verify", "HEAD"], true),
        ToolCheck::command("cargo", &["--version"], true),
        ToolCheck::cargo_subcommand("cargo-deny", "deny", &["--version"], true),
        ToolCheck::command("npm", &["--version"], true),
        ToolCheck::command("node", &["--version"], true),
        ToolCheck::env_command("wasm-pack", "SONARE_WASM_PACK", &["--version"], true),
        ToolCheck::python_module("maturin", true),
        ToolCheck::python_module("build", false),
        ToolCheck::env_command(
            "cargo-nextest",
            "SONARE_CARGO_NEXTEST",
            &["--version"],
            false,
        ),
        ToolCheck::env_command("cargo-audit", "SONARE_CARGO_AUDIT", &["--version"], false),
        ToolCheck::env_command(
            "cargo-semver-checks",
            "SONARE_CARGO_SEMVER_CHECKS",
            &["--version"],
            false,
        ),
        ToolCheck::env_command(
            "cargo-machete",
            "SONARE_CARGO_MACHETE",
            &["--version"],
            false,
        ),
        ToolCheck::cargo_toolchain_subcommand(
            "cargo-miri",
            "+nightly",
            "miri",
            &["--version"],
            false,
        ),
        ToolCheck::cargo_subcommand_with_env(
            "cargo-llvm-cov",
            "SONARE_CARGO_LLVM_COV",
            "llvm-cov",
            &["--version"],
            false,
        ),
    ];

    let mut missing_required = false;
    for check in checks {
        match check.run() {
            ToolStatus::Present(detail) => {
                eprintln!("{:<24} ok       {}", check.label, detail.trim());
            }
            ToolStatus::Missing(detail) => {
                eprintln!("{:<24} missing  {}", check.label, detail);
                missing_required |= check.required;
            }
        }
    }

    match wasm_target_installed() {
        Ok(true) => eprintln!("{:<24} ok       installed", "wasm32 target"),
        Ok(false) => eprintln!(
            "{:<24} missing  rustup target add wasm32-unknown-unknown",
            "wasm32 target"
        ),
        Err(err) => eprintln!("{:<24} missing  {err}", "wasm32 target"),
    }

    if missing_required {
        Err(())
    } else {
        Ok(())
    }
}

fn check_registry_name(label: &str, url: &str) -> Result<(), String> {
    match http_status(url) {
        Ok(404) => {
            eprintln!("{label}: available (404)");
            Ok(())
        }
        Ok(200) => Err(format!("{label}: already exists (200)")),
        Ok(status) => Err(format!("{label}: unexpected HTTP status {status}")),
        Err(err) => Err(format!("{label}: {err}")),
    }
}

fn http_status(url: &str) -> Result<u16, String> {
    let output = Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--location",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code}",
            "--user-agent",
            "sonare-codec release check",
            url,
        ])
        .output()
        .map_err(|err| format!("failed to run curl: {err}"))?;
    if !output.status.success() {
        return Err(format!("curl failed with status {}", output.status));
    }
    let status = String::from_utf8(output.stdout)
        .map_err(|err| format!("curl status output is not UTF-8: {err}"))?;
    status
        .trim()
        .parse::<u16>()
        .map_err(|err| format!("curl status output is not an HTTP status: {err}"))
}

fn fuzz_smoke() -> ExitCode {
    let wav_fixture = decode_hex(include_str!("../../tests/fixtures/wav-pcm16-stereo.hex"));
    let flac_fixture = match sonare_codec::AudioBuffer::new(
        48_000,
        1,
        (0..128)
            .map(|sample| sample as f32 / 32_767.0)
            .collect::<Vec<_>>(),
    )
    .and_then(|pcm| sonare_codec::encode(sonare_codec::Format::Flac, &pcm))
    {
        Ok(flac) => flac,
        Err(err) => {
            eprintln!("fuzz-smoke FLAC fixture generation failed: {err}");
            return ExitCode::FAILURE;
        }
    };
    let legacy_flac_corpus = [
        decode_hex(include_str!(
            "../../fuzz/corpus/flac_decode/minimal-constant-frame.hex"
        )),
        decode_hex(include_str!(
            "../../fuzz/corpus/flac_decode/minimal-fixed-frame.hex"
        )),
        decode_hex(include_str!(
            "../../fuzz/corpus/flac_decode/minimal-left-side-frame.hex"
        )),
        decode_hex(include_str!(
            "../../fuzz/corpus/flac_decode/minimal-lpc-frame.hex"
        )),
    ];
    let silent_aac = match sonare_codec::AudioBuffer::new(44_100, 1, vec![0.0; 1024])
        .and_then(|pcm| sonare_codec::encode(sonare_codec::Format::Aac, &pcm))
    {
        Ok(aac) => aac,
        Err(err) => {
            eprintln!("fuzz-smoke AAC fixture generation failed: {err}");
            return ExitCode::FAILURE;
        }
    };
    let malformed_corpus: &[&[u8]] = &[
        b"",
        b"RIFF",
        b"RIFF\x24\0\0\0WAVEfmt ",
        b"fLaC\0\0\0\0",
        b"ID3\x04\0\0\0\0\0\0",
        b"OggS\0\0\0OpusHead",
        b"OggS\0\0\0\x01vorbis",
        b"\0\0\0\x18ftypM4A ",
        &[0xff, 0xf1, 0x50, 0x80],
        &[0xff; 4096],
    ];

    for input in malformed_corpus {
        let _ = sonare_codec::decode(input);
    }
    for input in legacy_flac_corpus {
        let _ = sonare_codec::decode(&input);
    }
    for input in [&wav_fixture, &flac_fixture] {
        if let Err(err) = sonare_codec::decode(input) {
            eprintln!("fuzz-smoke fixture decode failed: {err}");
            return ExitCode::FAILURE;
        }
    }
    if let Err(err) = sonare_codec::decode(&silent_aac) {
        eprintln!("fuzz-smoke AAC fixture decode failed: {err}");
        return ExitCode::FAILURE;
    }
    let silent_m4a = match sonare_codec::mux_aac_adts_as_m4a(&silent_aac) {
        Ok(m4a) => m4a,
        Err(err) => {
            eprintln!("fuzz-smoke AAC fixture mux failed: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = sonare_codec::decode(&silent_m4a) {
        eprintln!("fuzz-smoke M4A fixture decode failed: {err}");
        return ExitCode::FAILURE;
    }
    match sonare_codec::demux_m4a_as_aac_adts(&silent_m4a) {
        Ok(adts) if adts == silent_aac => {}
        Ok(_) => {
            eprintln!("fuzz-smoke M4A demux did not preserve ADTS bytes");
            return ExitCode::FAILURE;
        }
        Err(err) => {
            eprintln!("fuzz-smoke M4A fixture demux failed: {err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn oracle_smoke() -> ExitCode {
    let Some(ffmpeg) = env::var_os("SONARE_FFMPEG") else {
        eprintln!("skipping oracle-smoke: set SONARE_FFMPEG=/path/to/ffmpeg to run local black-box acceptance checks");
        return ExitCode::SUCCESS;
    };

    let out_dir = env::temp_dir().join(format!(
        "sonare-codec-oracle-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    if let Err(err) = fs::create_dir_all(&out_dir) {
        eprintln!("oracle-smoke failed to create {}: {err}", out_dir.display());
        return ExitCode::FAILURE;
    }

    let generated = match generate_oracle_smoke_artifacts(&out_dir) {
        Ok(generated) => generated,
        Err(err) => {
            eprintln!("{err}");
            let _ = fs::remove_dir_all(&out_dir);
            return ExitCode::FAILURE;
        }
    };

    for artifact in &generated {
        let label = format!(
            "{} -v error -i {} -f null -",
            ffmpeg.to_string_lossy(),
            artifact.display()
        );
        let mut command = Command::new(&ffmpeg);
        command
            .args(["-v", "error", "-i"])
            .arg(artifact)
            .args(["-f", "null", "-"]);
        if let Err(err) = run_prepared_command(&mut command, &label) {
            eprintln!("{err}");
            eprintln!("oracle-smoke artifact kept at {}", artifact.display());
            return ExitCode::FAILURE;
        }
    }

    if let Err(err) = fs::remove_dir_all(&out_dir) {
        eprintln!("oracle-smoke could not remove {}: {err}", out_dir.display());
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn generate_oracle_smoke_artifacts(out_dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let non_silent = sonare_codec::AudioBuffer::new(
        44_100,
        1,
        (0..2048)
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>(),
    )
    .map_err(|err| format!("oracle-smoke PCM generation failed: {err}"))?;
    let silent_1024 = sonare_codec::AudioBuffer::new(44_100, 1, vec![0.0; 1024])
        .map_err(|err| format!("oracle-smoke silent PCM generation failed: {err}"))?;
    let silent_mp3 = sonare_codec::AudioBuffer::new(44_100, 1, vec![0.0; 1152 * 2])
        .map_err(|err| format!("oracle-smoke silent PCM generation failed: {err}"))?;

    let artifacts = [
        (
            "wav-non-silent.wav",
            sonare_codec::encode(sonare_codec::Format::Wav, &non_silent),
        ),
        (
            "flac-non-silent.flac",
            sonare_codec::encode(sonare_codec::Format::Flac, &non_silent),
        ),
        (
            "mp3-silent.mp3",
            sonare_codec::encode(sonare_codec::Format::Mp3, &silent_mp3),
        ),
        (
            "aac-silent.aac",
            sonare_codec::encode(sonare_codec::Format::Aac, &silent_1024),
        ),
    ];

    let mut paths = Vec::new();
    for (name, artifact) in artifacts {
        let bytes =
            artifact.map_err(|err| format!("oracle-smoke {name} generation failed: {err}"))?;
        let path = out_dir.join(name);
        fs::write(&path, bytes)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        paths.push(path);
    }

    let aac = fs::read(out_dir.join("aac-silent.aac"))
        .map_err(|err| format!("failed to read oracle-smoke AAC artifact: {err}"))?;
    let m4a = sonare_codec::mux_aac_adts_as_m4a(&aac)
        .map_err(|err| format!("oracle-smoke M4A generation failed: {err}"))?;
    let m4a_path = out_dir.join("aac-silent.m4a");
    fs::write(&m4a_path, m4a)
        .map_err(|err| format!("failed to write {}: {err}", m4a_path.display()))?;
    paths.push(m4a_path);

    for (name, artifact) in [
        (
            "mp3-non-silent-scaffold.mp3",
            sonare_codec::encode(sonare_codec::Format::Mp3, &non_silent),
        ),
        (
            "mp3-non-silent-standard-scaffold.mp3",
            sonare_codec::encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider(
                &non_silent,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            ),
        ),
        (
            "aac-non-silent-scaffold.aac",
            sonare_codec::encode(sonare_codec::Format::Aac, &non_silent),
        ),
    ] {
        let bytes =
            artifact.map_err(|err| format!("oracle-smoke {name} generation failed: {err}"))?;
        let path = out_dir.join(name);
        fs::write(&path, bytes)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        paths.push(path);
    }

    Ok(paths)
}

fn build_reference_manifest(
    artifacts: &[PathBuf],
    ffmpeg: Option<&std::ffi::OsStr>,
) -> Result<String, String> {
    let mut out = String::from(
        "{\n  \"schema\": 1,\n  \"generated_by\": \"cargo run -p xtask -- gen-refs\",\n",
    );
    match ffmpeg {
        Some(_) => out.push_str("  \"oracle\": \"ffmpeg\",\n"),
        None => out.push_str("  \"oracle\": null,\n"),
    }
    out.push_str("  \"artifacts\": [\n");

    for (index, path) in artifacts.iter().enumerate() {
        let bytes =
            fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let format = sonare_codec::detect(&bytes)
            .map(format_name)
            .unwrap_or("unknown");
        let decoded = sonare_codec::decode(&bytes).ok();
        let ffmpeg_accepts = match ffmpeg {
            Some(ffmpeg) => {
                run_ffmpeg_acceptance(ffmpeg, path)?;
                Some(true)
            }
            None => None,
        };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("artifact path has no UTF-8 file name: {}", path.display()))?;

        out.push_str("    {\n");
        out.push_str(&format!("      \"name\": \"{}\",\n", json_escape(name)));
        out.push_str(&format!("      \"format\": \"{format}\",\n"));
        out.push_str(&format!("      \"bytes\": {},\n", bytes.len()));
        out.push_str(&format!(
            "      \"fnv1a64\": \"{:016x}\",\n",
            fnv1a64(&bytes)
        ));
        match decoded {
            Some(decoded) => {
                out.push_str("      \"decode\": {\n");
                out.push_str(&format!(
                    "        \"sample_rate\": {},\n",
                    decoded.sample_rate
                ));
                out.push_str(&format!("        \"channels\": {},\n", decoded.channels));
                out.push_str(&format!("        \"samples\": {}\n", decoded.samples.len()));
                out.push_str("      },\n");
            }
            None => out.push_str("      \"decode\": null,\n"),
        }
        match ffmpeg_accepts {
            Some(accepts) => out.push_str(&format!("      \"ffmpeg_accepts\": {accepts}\n")),
            None => out.push_str("      \"ffmpeg_accepts\": null\n"),
        }
        out.push_str("    }");
        if index + 1 != artifacts.len() {
            out.push(',');
        }
        out.push('\n');
    }

    out.push_str("  ]\n}\n");
    Ok(out)
}

fn verify_refs() -> Result<(), String> {
    let ref_dir = Path::new("tests/refs/oracle-smoke");
    let manifest_path = ref_dir.join("manifest.json");
    let manifest = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("failed to read {}: {err}", manifest_path.display()))?;
    assert_contains(&manifest, "\"schema\": 1", "reference manifest schema")?;
    assert_contains(
        &manifest,
        "\"generated_by\": \"cargo run -p xtask -- gen-refs\"",
        "reference manifest generator",
    )?;

    let tmp_dir = env::temp_dir().join(format!(
        "sonare-codec-ref-check-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    fs::create_dir_all(&tmp_dir)
        .map_err(|err| format!("failed to create {}: {err}", tmp_dir.display()))?;

    let generated = match generate_oracle_smoke_artifacts(&tmp_dir)
        .and_then(|artifacts| compare_refs(ref_dir, &manifest, &artifacts))
    {
        Ok(()) => Ok(()),
        Err(err) => Err(err),
    };

    if let Err(err) = fs::remove_dir_all(&tmp_dir) {
        return Err(format!("failed to remove {}: {err}", tmp_dir.display()));
    }
    generated
}

fn compare_refs(ref_dir: &Path, manifest: &str, generated: &[PathBuf]) -> Result<(), String> {
    for generated_path in generated {
        let name = generated_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                format!(
                    "generated reference path has no UTF-8 file name: {}",
                    generated_path.display()
                )
            })?;
        let ref_path = ref_dir.join(name);
        let expected = fs::read(&ref_path)
            .map_err(|err| format!("failed to read committed ref {}: {err}", ref_path.display()))?;
        let actual = fs::read(generated_path).map_err(|err| {
            format!(
                "failed to read generated ref {}: {err}",
                generated_path.display()
            )
        })?;
        if expected != actual {
            return Err(format!(
                "reference artifact {name} differs from current encoder output; run `cargo run -p xtask -- gen-refs` after intentional encoder changes"
            ));
        }
        verify_manifest_artifact(manifest, name, &expected)?;
    }
    Ok(())
}

fn verify_manifest_artifact(manifest: &str, name: &str, bytes: &[u8]) -> Result<(), String> {
    let artifact = manifest_artifact_block(manifest, name)?;
    let format = sonare_codec::detect(bytes)
        .map(format_name)
        .unwrap_or("unknown");
    assert_contains(
        artifact,
        &format!("\"name\": \"{}\"", json_escape(name)),
        "reference manifest artifact name",
    )?;
    assert_contains(
        artifact,
        &format!("\"format\": \"{format}\""),
        "reference manifest artifact format",
    )?;
    assert_contains(
        artifact,
        &format!("\"bytes\": {}", bytes.len()),
        "reference manifest artifact byte size",
    )?;
    assert_contains(
        artifact,
        &format!("\"fnv1a64\": \"{:016x}\"", fnv1a64(bytes)),
        "reference manifest artifact hash",
    )?;
    if let Ok(decoded) = sonare_codec::decode(bytes) {
        assert_contains(
            artifact,
            &format!("\"sample_rate\": {}", decoded.sample_rate),
            "reference manifest decode sample rate",
        )?;
        assert_contains(
            artifact,
            &format!("\"channels\": {}", decoded.channels),
            "reference manifest decode channels",
        )?;
        assert_contains(
            artifact,
            &format!("\"samples\": {}", decoded.samples.len()),
            "reference manifest decode sample count",
        )?;
    }
    Ok(())
}

fn manifest_artifact_block<'a>(manifest: &'a str, name: &str) -> Result<&'a str, String> {
    let marker = format!("\"name\": \"{}\"", json_escape(name));
    let name_index = manifest
        .find(&marker)
        .ok_or_else(|| format!("reference manifest is missing artifact {name}"))?;
    let before = &manifest[..name_index];
    let start = before
        .rfind("    {")
        .ok_or_else(|| format!("reference manifest artifact {name} has no object start"))?;
    let after = &manifest[name_index..];
    let end_from_name = after
        .find("\n    }")
        .ok_or_else(|| format!("reference manifest artifact {name} has no object end"))?
        + "\n    }".len();
    Ok(&manifest[start..name_index + end_from_name])
}

fn run_ffmpeg_acceptance(ffmpeg: &OsStr, artifact: &Path) -> Result<(), String> {
    let label = format!(
        "{} -v error -i {} -f null -",
        ffmpeg.to_string_lossy(),
        artifact.display()
    );
    eprintln!("running {label}");
    let output = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(artifact)
        .args(["-f", "null", "-"])
        .output()
        .map_err(|err| format!("failed to run {label}: {err}"))?;
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let summary = stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("no stderr output");
        Err(format!(
            "{label} failed with status {}; first stderr line: {summary}",
            output.status
        ))
    }
}

fn run_ffmpeg_clean_acceptance(ffmpeg: &OsStr, artifact: &Path) -> Result<(), String> {
    let label = format!(
        "{} -v error -i {} -f null -",
        ffmpeg.to_string_lossy(),
        artifact.display()
    );
    eprintln!("running {label}");
    let output = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(artifact)
        .args(["-f", "null", "-"])
        .output()
        .map_err(|err| format!("failed to run {label}: {err}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let summary = stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("no stderr output");
    if output.status.success() && output.stderr.is_empty() {
        Ok(())
    } else if output.status.success() {
        Err(format!("{label} produced stderr: {summary}"))
    } else {
        Err(format!(
            "{label} failed with status {}; first stderr line: {summary}",
            output.status
        ))
    }
}

fn run_ffmpeg_decode_f32le(
    ffmpeg: &OsStr,
    artifact: &Path,
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<f32>, String> {
    let label = format!(
        "{} -v error -i {} -f f32le -acodec pcm_f32le -ac {} -ar {} -",
        ffmpeg.to_string_lossy(),
        artifact.display(),
        channels,
        sample_rate
    );
    eprintln!("running {label}");
    let output = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(artifact)
        .args([
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-ac",
            &channels.to_string(),
            "-ar",
            &sample_rate.to_string(),
            "-",
        ])
        .output()
        .map_err(|err| format!("failed to run {label}: {err}"))?;
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if !output.status.success() {
        return Err(format!("{label} failed with status {}", output.status));
    }
    if output.stdout.len() % 4 != 0 {
        return Err("decoded f32le byte count is not divisible by four".to_owned());
    }

    output
        .stdout
        .chunks_exact(4)
        .map(|chunk| {
            let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if sample.is_finite() {
                Ok(sample)
            } else {
                Err("decoded PCM contains non-finite samples".to_owned())
            }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LossyOraclePcmQuality {
    decoded_rms: f64,
    best_correlation: f64,
}

fn validate_lossy_oracle_pcm_quality(
    expected: &[f32],
    decoded: &[f32],
) -> Result<LossyOraclePcmQuality, String> {
    if expected.is_empty() {
        return Err("expected PCM is empty".to_owned());
    }
    if decoded.is_empty() {
        return Err("decoded PCM is empty".to_owned());
    }

    let expected_rms = rms(expected);
    let decoded_rms = rms(decoded);
    if expected_rms <= f64::EPSILON {
        return Err("expected PCM is silent".to_owned());
    }
    if decoded_rms < expected_rms * 0.05 {
        return Err(format!(
            "decoded PCM is effectively silent: decoded_rms={decoded_rms:.6}, expected_rms={expected_rms:.6}"
        ));
    }
    if decoded_rms > expected_rms * 32.0 {
        return Err(format!(
            "decoded PCM is excessively amplified: decoded_rms={decoded_rms:.6}, expected_rms={expected_rms:.6}"
        ));
    }

    let best_correlation = best_normalized_correlation(expected, decoded)?;
    if best_correlation < 0.20 {
        return Err(format!(
            "decoded PCM does not correlate with input: best_correlation={best_correlation:.3}"
        ));
    }

    Ok(LossyOraclePcmQuality {
        decoded_rms,
        best_correlation,
    })
}

fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let power = samples
        .iter()
        .map(|sample| {
            let sample = f64::from(*sample);
            sample * sample
        })
        .sum::<f64>();
    (power / samples.len() as f64).sqrt()
}

fn best_normalized_correlation(expected: &[f32], decoded: &[f32]) -> Result<f64, String> {
    let window_len = expected.len().min(decoded.len());
    if window_len < 64 {
        return Err("not enough decoded PCM to validate correlation".to_owned());
    }

    let expected_window = &expected[..window_len];
    let mut best = -1.0_f64;
    for offset in 0..=decoded.len() - window_len {
        let correlation =
            normalized_correlation(expected_window, &decoded[offset..offset + window_len]);
        if correlation > best {
            best = correlation;
        }
    }
    Ok(best)
}

fn normalized_correlation(left: &[f32], right: &[f32]) -> f64 {
    let mut dot = 0.0_f64;
    let mut left_power = 0.0_f64;
    let mut right_power = 0.0_f64;
    for (&left, &right) in left.iter().zip(right) {
        let left = f64::from(left);
        let right = f64::from(right);
        dot += left * right;
        left_power += left * left;
        right_power += right * right;
    }
    if left_power <= f64::EPSILON || right_power <= f64::EPSILON {
        0.0
    } else {
        dot / (left_power.sqrt() * right_power.sqrt())
    }
}

fn format_name(format: sonare_codec::Format) -> &'static str {
    match format {
        sonare_codec::Format::Wav => "wav",
        sonare_codec::Format::Flac => "flac",
        sonare_codec::Format::Mp3 => "mp3",
        sonare_codec::Format::Vorbis => "vorbis",
        sonare_codec::Format::Opus => "opus",
        sonare_codec::Format::Aac => "aac",
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn decode_hex(input: &str) -> Vec<u8> {
    let hex = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    assert_eq!(hex.len() % 2, 0);

    hex.chunks_exact(2)
        .map(|chunk| (hex_digit(chunk[0]) << 4) | hex_digit(chunk[1]))
        .collect()
}

fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => 0,
    }
}

enum Check<'a> {
    Cargo(&'a [&'a str]),
    Deny(&'a [&'a str]),
    GitHead,
    MaturinBuild,
    PackageMetadata,
    PublishReadiness,
    PublishRustPackages,
    PythonWheelOutput,
    ToolReadiness,
    WasmTarget,
    WasmPackBuild,
    WasmPackOutput,
    NpmPackDryRun,
    NpmPackOutput,
    RegistryNames,
    RegistryNamesIfRequested,
}

#[derive(Clone, Copy)]
enum ToolCommand<'a> {
    Command {
        program: &'a str,
        args: &'a [&'a str],
    },
    EnvCommand {
        env_var: &'a str,
        fallback_program: &'a str,
        args: &'a [&'a str],
    },
    CargoSubcommand {
        env_var: &'a str,
        subcommand: &'a str,
        args: &'a [&'a str],
    },
    CargoToolchainSubcommand {
        toolchain: &'a str,
        subcommand: &'a str,
        args: &'a [&'a str],
    },
    PythonModule {
        module: &'a str,
    },
}

#[derive(Clone, Copy)]
struct ToolCheck<'a> {
    label: &'a str,
    command: ToolCommand<'a>,
    required: bool,
}

enum ToolStatus {
    Present(String),
    Missing(String),
}

impl<'a> ToolCheck<'a> {
    fn command(label: &'a str, args: &'a [&'a str], required: bool) -> Self {
        Self {
            label,
            command: ToolCommand::Command {
                program: label,
                args,
            },
            required,
        }
    }

    fn env_command(label: &'a str, env_var: &'a str, args: &'a [&'a str], required: bool) -> Self {
        Self {
            label,
            command: ToolCommand::EnvCommand {
                env_var,
                fallback_program: label,
                args,
            },
            required,
        }
    }

    fn cargo_subcommand(
        label: &'a str,
        subcommand: &'a str,
        args: &'a [&'a str],
        required: bool,
    ) -> Self {
        Self {
            label,
            command: ToolCommand::CargoSubcommand {
                env_var: "SONARE_CARGO_DENY",
                subcommand,
                args,
            },
            required,
        }
    }

    fn cargo_subcommand_with_env(
        label: &'a str,
        env_var: &'a str,
        subcommand: &'a str,
        args: &'a [&'a str],
        required: bool,
    ) -> Self {
        Self {
            label,
            command: ToolCommand::CargoSubcommand {
                env_var,
                subcommand,
                args,
            },
            required,
        }
    }

    fn cargo_toolchain_subcommand(
        label: &'a str,
        toolchain: &'a str,
        subcommand: &'a str,
        args: &'a [&'a str],
        required: bool,
    ) -> Self {
        Self {
            label,
            command: ToolCommand::CargoToolchainSubcommand {
                toolchain,
                subcommand,
                args,
            },
            required,
        }
    }

    fn python_module(module: &'a str, required: bool) -> Self {
        Self {
            label: module,
            command: ToolCommand::PythonModule { module },
            required,
        }
    }

    fn run(self) -> ToolStatus {
        let output = match self.command {
            ToolCommand::Command { program, args } => Command::new(program).args(args).output(),
            ToolCommand::EnvCommand {
                env_var,
                fallback_program,
                args,
            } => {
                let program =
                    env::var_os(env_var).unwrap_or_else(|| OsString::from(fallback_program));
                Command::new(program).args(args).output()
            }
            ToolCommand::CargoSubcommand {
                env_var,
                subcommand,
                args,
            } => {
                if let Some(path) = env::var_os(env_var) {
                    Command::new(path).args(args).output()
                } else {
                    let mut cargo_args = Vec::with_capacity(args.len() + 1);
                    cargo_args.push(subcommand);
                    cargo_args.extend_from_slice(args);
                    Command::new("cargo").args(cargo_args).output()
                }
            }
            ToolCommand::CargoToolchainSubcommand {
                toolchain,
                subcommand,
                args,
            } => {
                let mut cargo_args = Vec::with_capacity(args.len() + 2);
                cargo_args.push(toolchain);
                cargo_args.push(subcommand);
                cargo_args.extend_from_slice(args);
                Command::new("cargo").args(cargo_args).output()
            }
            ToolCommand::PythonModule { module } => {
                let python =
                    env::var_os("SONARE_PYTHON").unwrap_or_else(|| OsString::from("python"));
                let script = format!(
                    "import importlib.util, sys; module={module:?}; spec=importlib.util.find_spec(module); print(f'python module {{module}}'); sys.exit(0 if spec else 1)"
                );
                Command::new(python).args(["-c", &script]).output()
            }
        };

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let detail = stdout
                    .lines()
                    .next()
                    .or_else(|| stderr.lines().next())
                    .unwrap_or("available")
                    .to_owned();
                ToolStatus::Present(detail)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let detail = stderr
                    .lines()
                    .next()
                    .or_else(|| stdout.lines().next())
                    .unwrap_or("command returned a non-zero status")
                    .to_owned();
                ToolStatus::Missing(detail)
            }
            Err(err) => ToolStatus::Missing(err.to_string()),
        }
    }
}

fn run_check(check: Check<'_>) -> Result<(), String> {
    match check {
        Check::Cargo(args) => run_command("cargo", args, "."),
        Check::Deny(args) => run_deny(args),
        Check::GitHead => run_git_head_check(),
        Check::MaturinBuild => run_maturin_build(),
        Check::PackageMetadata => run_package_metadata_check(),
        Check::PublishReadiness => run_publish_readiness_check(),
        Check::PublishRustPackages => run_publish_rust_packages(),
        Check::PythonWheelOutput => run_python_wheel_output_check(),
        Check::ToolReadiness => run_tool_readiness_check()
            .map_err(|()| "package-preflight required tools are missing".to_owned()),
        Check::WasmTarget => run_wasm_check(),
        Check::WasmPackBuild => run_wasm_pack_build(),
        Check::WasmPackOutput => run_wasm_pack_output_check(),
        Check::NpmPackDryRun => run_npm_pack_dry_run(),
        Check::NpmPackOutput => run_npm_pack_output_check(),
        Check::RegistryNames => run_registry_name_check(),
        Check::RegistryNamesIfRequested => run_registry_name_check_if_requested(),
    }
}

fn run_registry_name_check_if_requested() -> Result<(), String> {
    if env::var_os("SONARE_CHECK_REGISTRY_NAMES").is_none() {
        eprintln!(
            "skipping registry name check: set SONARE_CHECK_REGISTRY_NAMES=1 before first publish"
        );
        return Ok(());
    }

    run_registry_name_check()
}

fn verify_production_lossy_encode_readiness() -> Result<(), String> {
    eprintln!("checking production lossy encode readiness");
    let ffmpeg = env::var_os("SONARE_FFMPEG").ok_or_else(|| {
        "publish-readiness requires SONARE_FFMPEG=/path/to/ffmpeg for production MP3/AAC oracle acceptance"
            .to_owned()
    })?;
    let readiness_cases = [
        ("MP3 32kHz mono", sonare_codec::Format::Mp3, 32_000u32, 1u16),
        (
            "MP3 44.1kHz mono",
            sonare_codec::Format::Mp3,
            44_100u32,
            1u16,
        ),
        ("MP3 48kHz mono", sonare_codec::Format::Mp3, 48_000u32, 1u16),
        (
            "MP3 32kHz stereo",
            sonare_codec::Format::Mp3,
            32_000u32,
            2u16,
        ),
        (
            "MP3 44.1kHz stereo",
            sonare_codec::Format::Mp3,
            44_100u32,
            2u16,
        ),
        (
            "MP3 48kHz stereo",
            sonare_codec::Format::Mp3,
            48_000u32,
            2u16,
        ),
        (
            "AAC-LC 7.35kHz mono",
            sonare_codec::Format::Aac,
            7_350u32,
            1u16,
        ),
        (
            "AAC-LC 8kHz mono",
            sonare_codec::Format::Aac,
            8_000u32,
            1u16,
        ),
        (
            "AAC-LC 11.025kHz mono",
            sonare_codec::Format::Aac,
            11_025u32,
            1u16,
        ),
        (
            "AAC-LC 12kHz mono",
            sonare_codec::Format::Aac,
            12_000u32,
            1u16,
        ),
        (
            "AAC-LC 16kHz mono",
            sonare_codec::Format::Aac,
            16_000u32,
            1u16,
        ),
        (
            "AAC-LC 22.05kHz mono",
            sonare_codec::Format::Aac,
            22_050u32,
            1u16,
        ),
        (
            "AAC-LC 24kHz mono",
            sonare_codec::Format::Aac,
            24_000u32,
            1u16,
        ),
        (
            "AAC-LC 32kHz mono",
            sonare_codec::Format::Aac,
            32_000u32,
            1u16,
        ),
        (
            "AAC-LC 44.1kHz mono",
            sonare_codec::Format::Aac,
            44_100u32,
            1u16,
        ),
        (
            "AAC-LC 48kHz mono",
            sonare_codec::Format::Aac,
            48_000u32,
            1u16,
        ),
        (
            "AAC-LC 64kHz mono",
            sonare_codec::Format::Aac,
            64_000u32,
            1u16,
        ),
        (
            "AAC-LC 88.2kHz mono",
            sonare_codec::Format::Aac,
            88_200u32,
            1u16,
        ),
        (
            "AAC-LC 96kHz mono",
            sonare_codec::Format::Aac,
            96_000u32,
            1u16,
        ),
        (
            "AAC-LC 7.35kHz stereo",
            sonare_codec::Format::Aac,
            7_350u32,
            2u16,
        ),
        (
            "AAC-LC 8kHz stereo",
            sonare_codec::Format::Aac,
            8_000u32,
            2u16,
        ),
        (
            "AAC-LC 11.025kHz stereo",
            sonare_codec::Format::Aac,
            11_025u32,
            2u16,
        ),
        (
            "AAC-LC 12kHz stereo",
            sonare_codec::Format::Aac,
            12_000u32,
            2u16,
        ),
        (
            "AAC-LC 16kHz stereo",
            sonare_codec::Format::Aac,
            16_000u32,
            2u16,
        ),
        (
            "AAC-LC 22.05kHz stereo",
            sonare_codec::Format::Aac,
            22_050u32,
            2u16,
        ),
        (
            "AAC-LC 24kHz stereo",
            sonare_codec::Format::Aac,
            24_000u32,
            2u16,
        ),
        (
            "AAC-LC 32kHz stereo",
            sonare_codec::Format::Aac,
            32_000u32,
            2u16,
        ),
        (
            "AAC-LC 44.1kHz stereo",
            sonare_codec::Format::Aac,
            44_100u32,
            2u16,
        ),
        (
            "AAC-LC 48kHz stereo",
            sonare_codec::Format::Aac,
            48_000u32,
            2u16,
        ),
        (
            "AAC-LC 64kHz stereo",
            sonare_codec::Format::Aac,
            64_000u32,
            2u16,
        ),
        (
            "AAC-LC 88.2kHz stereo",
            sonare_codec::Format::Aac,
            88_200u32,
            2u16,
        ),
        (
            "AAC-LC 96kHz stereo",
            sonare_codec::Format::Aac,
            96_000u32,
            2u16,
        ),
    ];
    let m4a_readiness_cases = [
        ("M4A AAC-LC 7.35kHz mono", 7_350u32, 1u16),
        ("M4A AAC-LC 8kHz mono", 8_000u32, 1u16),
        ("M4A AAC-LC 11.025kHz mono", 11_025u32, 1u16),
        ("M4A AAC-LC 12kHz mono", 12_000u32, 1u16),
        ("M4A AAC-LC 16kHz mono", 16_000u32, 1u16),
        ("M4A AAC-LC 22.05kHz mono", 22_050u32, 1u16),
        ("M4A AAC-LC 24kHz mono", 24_000u32, 1u16),
        ("M4A AAC-LC 32kHz mono", 32_000u32, 1u16),
        ("M4A AAC-LC 44.1kHz mono", 44_100u32, 1u16),
        ("M4A AAC-LC 48kHz mono", 48_000u32, 1u16),
        ("M4A AAC-LC 64kHz mono", 64_000u32, 1u16),
        ("M4A AAC-LC 88.2kHz mono", 88_200u32, 1u16),
        ("M4A AAC-LC 96kHz mono", 96_000u32, 1u16),
        ("M4A AAC-LC 7.35kHz stereo", 7_350u32, 2u16),
        ("M4A AAC-LC 8kHz stereo", 8_000u32, 2u16),
        ("M4A AAC-LC 11.025kHz stereo", 11_025u32, 2u16),
        ("M4A AAC-LC 12kHz stereo", 12_000u32, 2u16),
        ("M4A AAC-LC 16kHz stereo", 16_000u32, 2u16),
        ("M4A AAC-LC 22.05kHz stereo", 22_050u32, 2u16),
        ("M4A AAC-LC 24kHz stereo", 24_000u32, 2u16),
        ("M4A AAC-LC 32kHz stereo", 32_000u32, 2u16),
        ("M4A AAC-LC 44.1kHz stereo", 44_100u32, 2u16),
        ("M4A AAC-LC 48kHz stereo", 48_000u32, 2u16),
        ("M4A AAC-LC 64kHz stereo", 64_000u32, 2u16),
        ("M4A AAC-LC 88.2kHz stereo", 88_200u32, 2u16),
        ("M4A AAC-LC 96kHz stereo", 96_000u32, 2u16),
    ];

    let mut missing = Vec::new();
    let mut encoded_artifacts = Vec::new();
    for (label, format, sample_rate, channels) in readiness_cases {
        let pcm = readiness_pcm(sample_rate, channels)
            .map_err(|err| format!("failed to build {label} readiness PCM: {err}"))?;
        match sonare_codec::encode_with_mode(format, &pcm, sonare_codec::EncodeMode::ProductionOnly)
        {
            Ok(encoded) if !encoded.is_empty() => {
                encoded_artifacts.push((
                    label,
                    ProductionArtifactKind::from_format(format)?,
                    pcm,
                    encoded,
                ));
            }
            Ok(_) => missing.push(format!("{label} production encode returned empty bytes")),
            Err(err) => missing.push(format!("{label} production encode is not ready: {err}")),
        }
    }
    for (label, sample_rate, channels) in m4a_readiness_cases {
        let pcm = readiness_pcm(sample_rate, channels)
            .map_err(|err| format!("failed to build {label} readiness PCM: {err}"))?;
        match sonare_codec::encode_with_mode(
            sonare_codec::Format::Aac,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        ) {
            Ok(adts) if !adts.is_empty() => match sonare_codec::mux_aac_adts_as_m4a(&adts) {
                Ok(m4a) if !m4a.is_empty() => {
                    encoded_artifacts.push((label, ProductionArtifactKind::M4a, pcm, m4a));
                }
                Ok(_) => missing.push(format!("{label} production mux returned empty bytes")),
                Err(err) => missing.push(format!("{label} production mux is not ready: {err}")),
            },
            Ok(_) => missing.push(format!("{label} production encode returned empty bytes")),
            Err(err) => missing.push(format!("{label} production encode is not ready: {err}")),
        }
    }

    let production_oracle = if encoded_artifacts.is_empty() {
        Ok(())
    } else {
        verify_production_lossy_oracle_acceptance(ffmpeg.clone(), &encoded_artifacts)
    };

    if !missing.is_empty() || production_oracle.is_err() {
        let diagnostic_pcm = readiness_pcm(44_100, 1)
            .map_err(|err| format!("failed to build diagnostic readiness PCM: {err}"))?;
        let diagnostics = compatibility_lossy_encode_diagnostics(&ffmpeg, &diagnostic_pcm)?;
        let mut failures = missing;
        if let Err(err) = production_oracle {
            failures.push(err);
        }
        return Err(format!(
            "publish-readiness failed:\n  {}\n\nCompatibility scaffold diagnostics:\n  {}\nDo not publish until all remaining non-silent lossy production encode paths pass.",
            failures.join("\n  "),
            diagnostics.join("\n  ")
        ));
    }

    Ok(())
}

fn verify_diagnostic_lossy_encode_readiness() -> Result<(), String> {
    eprintln!("checking diagnostic lossy encode readiness");
    let ffmpeg = env::var_os("SONARE_FFMPEG").ok_or_else(|| {
        "publish-readiness diagnostics require SONARE_FFMPEG=/path/to/ffmpeg for MP3/AAC acceptance"
            .to_owned()
    })?;
    let pcm = readiness_pcm(44_100, 1)
        .map_err(|err| format!("failed to build diagnostic readiness PCM: {err}"))?;
    let stereo_pcm = readiness_pcm(44_100, 2)
        .map_err(|err| format!("failed to build stereo diagnostic readiness PCM: {err}"))?;
    let out_dir = env::temp_dir().join(format!(
        "sonare-codec-diagnostic-readiness-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("failed to create {}: {err}", out_dir.display()))?;

    let result: Result<DiagnosticLossyQualitySummary, String> = (|| {
        let mp3_quality = mp3_perceptual_nonzero_encode_diagnostic(&ffmpeg, &pcm, &out_dir)?;
        let mp3_reservoir_quality =
            mp3_perceptual_reservoir_nonzero_encode_diagnostic(&ffmpeg, &pcm, &out_dir)?;
        let mp3_stereo_reservoir_quality =
            mp3_perceptual_reservoir_nonzero_encode_diagnostic(&ffmpeg, &stereo_pcm, &out_dir)?;
        let (mp3_production_mono_quality, mp3_production_stereo_quality) =
            validate_mp3_production_benchmark_surface(&ffmpeg, &pcm, &stereo_pcm, &out_dir)?;
        validate_diagnostic_quality_floor(
            "MP3 stereo perceptual reservoir diagnostic",
            mp3_stereo_reservoir_quality,
            MP3_PERCEPTUAL_DIAGNOSTIC_MIN_DECODED_RMS,
            MP3_STEREO_PERCEPTUAL_RESERVOIR_MIN_CORRELATION,
        )?;
        validate_mp3_perceptual_reservoir_production_correlation_gap(
            "MP3 perceptual reservoir mono",
            mp3_reservoir_quality,
            mp3_production_mono_quality,
        )?;
        validate_mp3_perceptual_reservoir_production_correlation_gap(
            "MP3 perceptual reservoir stereo",
            mp3_stereo_reservoir_quality,
            mp3_production_stereo_quality,
        )?;
        validate_aac_standard_id_mixed_workbench()?;
        let aac_quality = standard_aac_lc_nonzero_encode_diagnostic(&ffmpeg, &pcm, &out_dir)?;
        let (aac_standard_surface_mono_quality, aac_standard_surface_stereo_quality) =
            validate_aac_standard_id_high_level_bitrate_surface(&ffmpeg, &pcm, &out_dir)?;
        let (aac_production_mono_quality, aac_production_stereo_quality) =
            validate_aac_production_benchmark_surface(&ffmpeg, &pcm, &out_dir)?;
        validate_aac_standard_id_production_correlation_gap(
            "AAC standard-id mono",
            aac_standard_surface_mono_quality,
            aac_production_mono_quality,
        )?;
        validate_aac_standard_id_production_correlation_gap(
            "AAC standard-id stereo",
            aac_standard_surface_stereo_quality,
            aac_production_stereo_quality,
        )?;
        validate_aac_standard_id_rms_control_advantage(
            "AAC standard-id mono",
            aac_standard_surface_mono_quality,
            aac_production_mono_quality,
            rms(&pcm.samples),
        )?;
        let aac_standard_stereo_pcm = aac_standard_surface_stereo_pcm(&pcm)?;
        validate_aac_standard_id_rms_control_advantage(
            "AAC standard-id stereo",
            aac_standard_surface_stereo_quality,
            aac_production_stereo_quality,
            rms(&aac_standard_stereo_pcm.samples),
        )?;
        let aac_standard_mono_frame_budget =
            compare_aac_standard_id_to_production_frame_selection(&pcm)?;
        let aac_standard_stereo_frame_budget =
            compare_aac_standard_id_to_production_frame_selection(&aac_standard_stereo_pcm)?;
        let aac_standard_mono_production_step_frame_budget =
            compare_aac_standard_id_candidate_set_to_production_frame_selection(
                &pcm,
                sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            )?;
        let aac_standard_stereo_production_step_frame_budget =
            compare_aac_standard_id_candidate_set_to_production_frame_selection(
                &aac_standard_stereo_pcm,
                sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            )?;
        let aac_standard_mono_details =
            aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
                &pcm,
                sonare_codec::aac_lc_default_production_bitrate_bps(1)
                    .map_err(|err| format!("AAC mono production bitrate lookup failed: {err}"))?,
                sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            )?;
        let aac_standard_stereo_details =
            aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
                &aac_standard_stereo_pcm,
                sonare_codec::aac_lc_default_production_bitrate_bps(2)
                    .map_err(|err| format!("AAC stereo production bitrate lookup failed: {err}"))?,
                sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            )?;
        let aac_standard_mono_payload_breakdown =
            aac_standard_id_payload_breakdown_for_frame_selection(
                &pcm,
                &aac_standard_mono_details,
            )?;
        let aac_standard_stereo_payload_breakdown =
            aac_standard_id_payload_breakdown_for_frame_selection(
                &aac_standard_stereo_pcm,
                &aac_standard_stereo_details,
            )?;
        Ok(DiagnosticLossyQualitySummary {
            mp3_quality,
            mp3_reservoir_quality,
            mp3_stereo_reservoir_quality,
            mp3_production_mono_quality,
            mp3_production_stereo_quality,
            aac_quality,
            aac_standard_surface_mono_quality,
            aac_standard_surface_stereo_quality,
            aac_production_mono_quality,
            aac_production_stereo_quality,
            aac_standard_mono_frame_budget,
            aac_standard_stereo_frame_budget,
            aac_standard_mono_production_step_frame_budget,
            aac_standard_stereo_production_step_frame_budget,
            aac_standard_mono_payload_breakdown,
            aac_standard_stereo_payload_breakdown,
        })
    })();

    let cleanup = fs::remove_dir_all(&out_dir)
        .map_err(|err| format!("failed to remove {}: {err}", out_dir.display()));
    match (result, cleanup) {
        (Ok(summary), Ok(())) => {
            let aac_mono_expected_rms = rms(&pcm.samples);
            let aac_standard_stereo_pcm = aac_standard_surface_stereo_pcm(&pcm)?;
            let aac_stereo_expected_rms = rms(&aac_standard_stereo_pcm.samples);
            eprintln!(
                "diagnostic lossy encode readiness: MP3 decoded_rms={:.4}, MP3 best_correlation={:.3}, MP3 reservoir decoded_rms={:.4}, MP3 reservoir best_correlation={:.3}, MP3 stereo reservoir decoded_rms={:.4}, MP3 stereo reservoir best_correlation={:.3}, MP3 production mono best_correlation={:.3}, MP3 reservoir mono correlation_gap={:.3}, MP3 production stereo best_correlation={:.3}, MP3 reservoir stereo correlation_gap={:.3}, AAC decoded_rms={:.4}, AAC best_correlation={:.3}, AAC standard-id mono decoded_rms={:.4}, AAC standard-id mono rms_error={:.4}, AAC standard-id mono best_correlation={:.3}, AAC production mono decoded_rms={:.4}, AAC production mono rms_error={:.4}, AAC production mono best_correlation={:.3}, AAC standard-id mono correlation_gap={:.3}, AAC standard-id mono frame_len_delta={}, AAC standard-id mono min_slack_delta={}, AAC standard-id mono step_delta={:.6}, AAC standard-id mono production-step frame_len_delta={}, AAC standard-id mono production-step min_slack_delta={}, AAC standard-id mono production-step step_delta={:.6}, AAC standard-id mono section_bits={}, AAC standard-id mono scale_factor_bits={}, AAC standard-id mono spectral_bits={}, AAC standard-id mono total_bits={}, AAC standard-id mono escape_sections={}, AAC standard-id mono escape_spectral_bits={}, AAC standard-id mono max_abs={}, AAC standard-id mono dominant_section={:?}, AAC standard-id mono dominant_escape_section={:?}, AAC standard-id stereo decoded_rms={:.4}, AAC standard-id stereo rms_error={:.4}, AAC standard-id stereo best_correlation={:.3}, AAC production stereo decoded_rms={:.4}, AAC production stereo rms_error={:.4}, AAC production stereo best_correlation={:.3}, AAC standard-id stereo correlation_gap={:.3}, AAC standard-id stereo frame_len_delta={}, AAC standard-id stereo min_slack_delta={}, AAC standard-id stereo step_delta={:.6}, AAC standard-id stereo production-step frame_len_delta={}, AAC standard-id stereo production-step min_slack_delta={}, AAC standard-id stereo production-step step_delta={:.6}, AAC standard-id stereo section_bits={}, AAC standard-id stereo scale_factor_bits={}, AAC standard-id stereo spectral_bits={}, AAC standard-id stereo total_bits={}, AAC standard-id stereo escape_sections={}, AAC standard-id stereo escape_spectral_bits={}, AAC standard-id stereo max_abs={}, AAC standard-id stereo dominant_section={:?}, AAC standard-id stereo dominant_escape_section={:?}",
                summary.mp3_quality.decoded_rms,
                summary.mp3_quality.best_correlation,
                summary.mp3_reservoir_quality.decoded_rms,
                summary.mp3_reservoir_quality.best_correlation,
                summary.mp3_stereo_reservoir_quality.decoded_rms,
                summary.mp3_stereo_reservoir_quality.best_correlation,
                summary.mp3_production_mono_quality.best_correlation,
                summary.mp3_production_mono_quality.best_correlation
                    - summary.mp3_reservoir_quality.best_correlation,
                summary.mp3_production_stereo_quality.best_correlation,
                summary.mp3_production_stereo_quality.best_correlation
                    - summary.mp3_stereo_reservoir_quality.best_correlation,
                summary.aac_quality.decoded_rms,
                summary.aac_quality.best_correlation,
                summary.aac_standard_surface_mono_quality.decoded_rms,
                rms_error(summary.aac_standard_surface_mono_quality, aac_mono_expected_rms),
                summary.aac_standard_surface_mono_quality.best_correlation,
                summary.aac_production_mono_quality.decoded_rms,
                rms_error(summary.aac_production_mono_quality, aac_mono_expected_rms),
                summary.aac_production_mono_quality.best_correlation,
                summary.aac_production_mono_quality.best_correlation
                    - summary.aac_standard_surface_mono_quality.best_correlation,
                summary.aac_standard_mono_frame_budget.max_frame_len_delta,
                summary.aac_standard_mono_frame_budget.min_budget_slack_delta,
                summary.aac_standard_mono_frame_budget.max_step_delta,
                summary
                    .aac_standard_mono_production_step_frame_budget
                    .max_frame_len_delta,
                summary
                    .aac_standard_mono_production_step_frame_budget
                    .min_budget_slack_delta,
                summary
                    .aac_standard_mono_production_step_frame_budget
                    .max_step_delta,
                summary.aac_standard_mono_payload_breakdown.section_bits,
                summary
                    .aac_standard_mono_payload_breakdown
                    .scale_factor_bits,
                summary.aac_standard_mono_payload_breakdown.spectral_bits,
                summary.aac_standard_mono_payload_breakdown.total_bits(),
                summary.aac_standard_mono_payload_breakdown.escape_sections,
                summary
                    .aac_standard_mono_payload_breakdown
                    .escape_spectral_bits,
                summary.aac_standard_mono_payload_breakdown.max_abs,
                summary
                    .aac_standard_mono_payload_breakdown
                    .dominant_spectral_section,
                summary
                    .aac_standard_mono_payload_breakdown
                    .dominant_escape_section,
                summary.aac_standard_surface_stereo_quality.decoded_rms,
                rms_error(
                    summary.aac_standard_surface_stereo_quality,
                    aac_stereo_expected_rms
                ),
                summary.aac_standard_surface_stereo_quality.best_correlation,
                summary.aac_production_stereo_quality.decoded_rms,
                rms_error(summary.aac_production_stereo_quality, aac_stereo_expected_rms),
                summary.aac_production_stereo_quality.best_correlation,
                summary.aac_production_stereo_quality.best_correlation
                    - summary.aac_standard_surface_stereo_quality.best_correlation,
                summary.aac_standard_stereo_frame_budget.max_frame_len_delta,
                summary.aac_standard_stereo_frame_budget.min_budget_slack_delta,
                summary.aac_standard_stereo_frame_budget.max_step_delta,
                summary
                    .aac_standard_stereo_production_step_frame_budget
                    .max_frame_len_delta,
                summary
                    .aac_standard_stereo_production_step_frame_budget
                    .min_budget_slack_delta,
                summary
                    .aac_standard_stereo_production_step_frame_budget
                    .max_step_delta,
                summary.aac_standard_stereo_payload_breakdown.section_bits,
                summary
                    .aac_standard_stereo_payload_breakdown
                    .scale_factor_bits,
                summary.aac_standard_stereo_payload_breakdown.spectral_bits,
                summary.aac_standard_stereo_payload_breakdown.total_bits(),
                summary.aac_standard_stereo_payload_breakdown.escape_sections,
                summary
                    .aac_standard_stereo_payload_breakdown
                    .escape_spectral_bits,
                summary.aac_standard_stereo_payload_breakdown.max_abs,
                summary
                    .aac_standard_stereo_payload_breakdown
                    .dominant_spectral_section,
                summary
                    .aac_standard_stereo_payload_breakdown
                    .dominant_escape_section
            );
            Ok(())
        }
        (Err(err), Ok(())) => Err(format!("diagnostic lossy encode readiness failed: {err}")),
        (Ok(_), Err(err)) => Err(err),
        (Err(err), Err(cleanup_err)) => Err(format!(
            "diagnostic lossy encode readiness failed: {err}; cleanup also failed: {cleanup_err}"
        )),
    }
}

#[derive(Clone, Copy, Debug)]
struct DiagnosticLossyQualitySummary {
    mp3_quality: LossyOraclePcmQuality,
    mp3_reservoir_quality: LossyOraclePcmQuality,
    mp3_stereo_reservoir_quality: LossyOraclePcmQuality,
    mp3_production_mono_quality: LossyOraclePcmQuality,
    mp3_production_stereo_quality: LossyOraclePcmQuality,
    aac_quality: LossyOraclePcmQuality,
    aac_standard_surface_mono_quality: LossyOraclePcmQuality,
    aac_standard_surface_stereo_quality: LossyOraclePcmQuality,
    aac_production_mono_quality: LossyOraclePcmQuality,
    aac_production_stereo_quality: LossyOraclePcmQuality,
    aac_standard_mono_frame_budget: AacFrameSelectionComparison,
    aac_standard_stereo_frame_budget: AacFrameSelectionComparison,
    aac_standard_mono_production_step_frame_budget: AacFrameSelectionComparison,
    aac_standard_stereo_production_step_frame_budget: AacFrameSelectionComparison,
    aac_standard_mono_payload_breakdown: AacStandardIdPayloadBreakdown,
    aac_standard_stereo_payload_breakdown: AacStandardIdPayloadBreakdown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct AacFrameSelectionComparison {
    frames: usize,
    production_max_frame_len: usize,
    standard_id_max_frame_len: usize,
    max_frame_len_delta: isize,
    production_min_budget_slack: usize,
    standard_id_min_budget_slack: usize,
    min_budget_slack_delta: isize,
    production_max_step: f32,
    standard_id_max_step: f32,
    max_step_delta: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AacStandardIdPayloadBreakdown {
    frames: usize,
    channels: usize,
    sections: usize,
    escape_sections: usize,
    max_abs: i32,
    section_bits: usize,
    scale_factor_bits: usize,
    spectral_bits: usize,
    escape_spectral_bits: usize,
    dominant_spectral_section: Option<AacStandardIdSpectralSectionBreakdown>,
    dominant_escape_section: Option<AacStandardIdSpectralSectionBreakdown>,
}

impl AacStandardIdPayloadBreakdown {
    fn total_bits(self) -> usize {
        self.section_bits + self.scale_factor_bits + self.spectral_bits
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AacStandardIdSpectralSectionBreakdown {
    frame_index: usize,
    channel: usize,
    start_band: usize,
    end_band: usize,
    start: usize,
    end: usize,
    codebook_id: u8,
    max_abs: i32,
    spectral_bits: usize,
    best_alternative_codebook_id: Option<u8>,
    best_alternative_spectral_bits: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
enum ProductionArtifactKind {
    Mp3,
    Aac,
    M4a,
}

impl ProductionArtifactKind {
    fn from_format(format: sonare_codec::Format) -> Result<Self, String> {
        match format {
            sonare_codec::Format::Mp3 => Ok(Self::Mp3),
            sonare_codec::Format::Aac => Ok(Self::Aac),
            _ => Err(format!(
                "unexpected production lossy format for oracle: {format:?}"
            )),
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "aac",
            Self::M4a => "m4a",
        }
    }
}

fn readiness_pcm(
    sample_rate: u32,
    channels: u16,
) -> Result<sonare_codec::AudioBuffer, sonare_codec::Error> {
    let mut samples = Vec::with_capacity(2304 * usize::from(channels));
    for frame in 0..2304 {
        for channel in 0..channels {
            let phase = if channel == 0 { 0.01 } else { 0.013 };
            samples.push(((frame as f32) * phase).sin() * 0.25);
        }
    }
    sonare_codec::AudioBuffer::new(sample_rate, channels, samples)
}

fn compatibility_lossy_encode_diagnostics(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
) -> Result<Vec<String>, String> {
    let out_dir = env::temp_dir().join(format!(
        "sonare-codec-compatibility-readiness-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("failed to create {}: {err}", out_dir.display()))?;

    let mut diagnostics = Vec::new();
    for (label, format) in [
        ("MP3", sonare_codec::Format::Mp3),
        ("AAC-LC", sonare_codec::Format::Aac),
    ] {
        let diagnostic =
            compatibility_lossy_encode_diagnostic(ffmpeg, expected_pcm, &out_dir, label, format);
        diagnostics.push(match diagnostic {
            Ok(quality) => format!(
                "{label} compatibility scaffold passes current oracle: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            ),
            Err(err) => format!("{label} compatibility scaffold cannot be promoted: {err}"),
        });
    }
    let mp3_standard = standard_mp3_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match mp3_standard {
        Ok(quality) => format!(
            "MP3 standard-table scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => format!("MP3 standard-table scaffold is not publish-ready: {err}"),
    });
    let mp3_perceptual = mp3_perceptual_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match mp3_perceptual {
        Ok(quality) => format!(
            "MP3 perceptual-scale-factor scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => {
            format!("MP3 perceptual-scale-factor scaffold is not publish-ready: {err}")
        }
    });
    let mp3_perceptual_reservoir =
        mp3_perceptual_reservoir_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match mp3_perceptual_reservoir {
        Ok(quality) => format!(
            "MP3 perceptual reservoir scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => {
            format!("MP3 perceptual reservoir scaffold is not publish-ready: {err}")
        }
    });
    let aac_experimental =
        experimental_aac_lc_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match aac_experimental {
        Ok(quality) => format!(
            "AAC-LC experimental nonzero scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => format!("AAC-LC experimental nonzero scaffold is not publish-ready: {err}"),
    });
    let aac_standard = standard_aac_lc_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match aac_standard {
        Ok(quality) => format!(
            "AAC-LC standard-table scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => format!("AAC-LC standard-table scaffold is not publish-ready: {err}"),
    });

    fs::remove_dir_all(&out_dir)
        .map_err(|err| format!("failed to remove {}: {err}", out_dir.display()))?;
    Ok(diagnostics)
}

fn compatibility_lossy_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    label: &str,
    format: sonare_codec::Format,
) -> Result<LossyOraclePcmQuality, String> {
    let encoded = sonare_codec::encode(format, expected_pcm)
        .map_err(|err| format!("compatibility encode failed: {err}"))?;
    let extension = match format {
        sonare_codec::Format::Mp3 => "mp3",
        sonare_codec::Format::Aac => "aac",
        _ => {
            return Err(format!(
                "unexpected compatibility lossy format for oracle: {format:?}"
            ))
        }
    };
    let path = out_dir.join(format!(
        "{}-compatibility.{}",
        label.to_ascii_lowercase().replace('-', ""),
        extension
    ));
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
}

fn standard_mp3_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    let encoded = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider(
        expected_pcm,
        sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        sonare_codec::mpeg1_layer3_standard_table_provider(),
    )
    .map_err(|err| format!("standard-table encode failed: {err}"))?;
    let path = out_dir.join("mp3-standard-table-nonzero.mp3");
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
}

fn mp3_perceptual_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    eprintln!(
        "{}",
        mp3_perceptual_diagnostic_summary(expected_pcm, MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES)?
    );
    let encoded =
        sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
            expected_pcm,
            MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| format!("perceptual-scale-factor encode failed: {err}"))?;
    let path = out_dir.join("mp3-perceptual-scale-factor-nonzero.mp3");
    verify_mp3_cbr_bitrate_budget(
        "MP3 perceptual-scale-factor diagnostic",
        128,
        expected_pcm,
        &encoded,
    )?;
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)?;
    validate_diagnostic_quality_floor(
        "MP3 perceptual-scale-factor diagnostic",
        quality,
        MP3_PERCEPTUAL_DIAGNOSTIC_MIN_DECODED_RMS,
        MP3_PERCEPTUAL_DIAGNOSTIC_MIN_CORRELATION,
    )?;
    Ok(quality)
}

fn mp3_perceptual_reservoir_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    let label = if expected_pcm.channels == 2 {
        "MP3 stereo perceptual reservoir diagnostic"
    } else {
        "MP3 perceptual reservoir diagnostic"
    };
    let encoded =
        sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
            expected_pcm,
            MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| format!("perceptual reservoir encode failed: {err}"))?;
    let path = out_dir.join(if expected_pcm.channels == 2 {
        "mp3-stereo-perceptual-reservoir-nonzero.mp3"
    } else {
        "mp3-perceptual-reservoir-nonzero.mp3"
    });
    verify_mp3_cbr_bitrate_budget(label, 128, expected_pcm, &encoded)?;
    verify_mp3_perceptual_reservoir(label, expected_pcm, &encoded)?;
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)?;
    validate_diagnostic_quality_floor(
        label,
        quality,
        MP3_PERCEPTUAL_DIAGNOSTIC_MIN_DECODED_RMS,
        MP3_PERCEPTUAL_DIAGNOSTIC_MIN_CORRELATION,
    )?;
    Ok(quality)
}

fn validate_mp3_production_benchmark_surface(
    ffmpeg: &OsStr,
    mono_pcm: &sonare_codec::AudioBuffer,
    stereo_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<(LossyOraclePcmQuality, LossyOraclePcmQuality), String> {
    if mono_pcm.channels != 1 {
        return Err("MP3 production benchmark mono PCM must be mono".to_owned());
    }
    if stereo_pcm.channels != 2 {
        return Err("MP3 production benchmark stereo PCM must be stereo".to_owned());
    }
    let mono_quality = validate_mp3_production_benchmark_artifact(
        ffmpeg,
        "MP3 production benchmark mono",
        mono_pcm,
        out_dir,
        "mp3-production-benchmark-mono",
    )?;
    let stereo_quality = validate_mp3_production_benchmark_artifact(
        ffmpeg,
        "MP3 production benchmark stereo",
        stereo_pcm,
        out_dir,
        "mp3-production-benchmark-stereo",
    )?;
    Ok((mono_quality, stereo_quality))
}

fn validate_mp3_production_benchmark_artifact(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    file_stem: &str,
) -> Result<LossyOraclePcmQuality, String> {
    let mp3 = sonare_codec::encode_with_mode(
        sonare_codec::Format::Mp3,
        expected_pcm,
        sonare_codec::EncodeMode::ProductionOnly,
    )
    .map_err(|err| format!("{label} encode failed: {err}"))?;
    verify_mp3_default_production_budget(label, ProductionArtifactKind::Mp3, expected_pcm, &mp3)?;
    let path = out_dir.join(format!("{file_stem}.mp3"));
    fs::write(&path, mp3).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("{label} FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("{label} FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
        .map_err(|err| format!("{label} PCM quality failed: {err}"))?;
    eprintln!(
        "{label}: decoded_rms={:.4}, best_correlation={:.3}",
        quality.decoded_rms, quality.best_correlation
    );
    Ok(quality)
}

fn mp3_perceptual_diagnostic_summary(
    expected_pcm: &sonare_codec::AudioBuffer,
    candidates: &[f32],
) -> Result<String, String> {
    const BITRATE_KBPS: u16 = 128;

    let base_header = sonare_codec::layer3_header_for_capacity(
        expected_pcm.sample_rate,
        expected_pcm.channels,
        BITRATE_KBPS,
        false,
        false,
    )
    .map_err(|err| format!("MP3 perceptual diagnostic header failed: {err}"))?;
    let samples_per_frame = usize::from(base_header.samples_per_frame());
    let channels = usize::from(expected_pcm.channels);
    let frames = expected_pcm.samples.len().div_ceil(channels);
    let frame_count = frames.div_ceil(samples_per_frame).max(1);
    let coefficient = if samples_per_frame == 1152 {
        144_u64
    } else {
        72_u64
    };
    let slots = coefficient
        .checked_mul(u64::from(BITRATE_KBPS))
        .and_then(|value| value.checked_mul(1000))
        .ok_or_else(|| "MP3 perceptual diagnostic bitrate slots overflow".to_owned())?;
    let sample_rate = u64::from(expected_pcm.sample_rate);
    let slot_remainder = slots % sample_rate;
    let mut accumulator = 0_u64;
    let mut padded_frames = 0usize;
    let mut min_step = f32::INFINITY;
    let mut max_step = 0.0_f32;
    let mut max_payload_bits = 0usize;
    let mut min_capacity_bits = usize::MAX;
    let mut nonzero_scale_factors = 0usize;
    let mut max_scale_factor = 0u8;
    let mut scale_factor_sum = 0usize;
    let mut scale_factor_bands = 0usize;
    let mut first_nonzero_scale_factor_step: Option<(f32, usize, usize)> = None;
    let mut first_frame_candidate_profile = Vec::new();
    for frame_index in 0..frame_count {
        accumulator += slot_remainder;
        let padded = if accumulator >= sample_rate {
            accumulator -= sample_rate;
            true
        } else {
            false
        };
        padded_frames += usize::from(padded);
        let frame_header = sonare_codec::layer3_header_for_capacity(
            expected_pcm.sample_rate,
            expected_pcm.channels,
            BITRATE_KBPS,
            padded,
            false,
        )
        .map_err(|err| format!("MP3 perceptual diagnostic frame header failed: {err}"))?;
        let start_frame = frame_index
            .checked_mul(samples_per_frame)
            .ok_or_else(|| "MP3 perceptual diagnostic frame start overflows".to_owned())?;
        let selection =
            sonare_codec::select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider(
                frame_header,
                expected_pcm,
                start_frame,
                candidates,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .map_err(|err| format!("MP3 perceptual diagnostic step selection failed: {err}"))?;
        min_step = min_step.min(selection.step);
        max_step = max_step.max(selection.step);
        max_payload_bits = max_payload_bits.max(selection.payload_bit_len);
        min_capacity_bits = min_capacity_bits.min(selection.frame_capacity_bits);
        for granule in 0..(samples_per_frame / 576).max(1) {
            let granule_start = start_frame
                .checked_add(granule * 576)
                .ok_or_else(|| "MP3 perceptual diagnostic granule start overflows".to_owned())?;
            for channel in 0..usize::from(expected_pcm.channels) {
                let scale_factors =
                    sonare_codec::select_mpeg1_layer3_psychoacoustic_long_scale_factors(
                        expected_pcm,
                        channel,
                        granule_start,
                        selection.step,
                        false,
                        1024,
                    )
                    .map_err(|err| {
                        format!("MP3 perceptual diagnostic scale-factor selection failed: {err}")
                    })?;
                for scale_factor in scale_factors {
                    nonzero_scale_factors += usize::from(scale_factor != 0);
                    max_scale_factor = max_scale_factor.max(scale_factor);
                    scale_factor_sum += usize::from(scale_factor);
                    scale_factor_bands += 1;
                }
            }
        }
    }
    let candidate_profiles =
        sonare_codec::select_mpeg1_layer3_first_frame_perceptual_candidate_profile_with_table_provider(
            expected_pcm,
            candidates,
            BITRATE_KBPS,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| format!("MP3 perceptual diagnostic candidate profile failed: {err}"))?;
    for profile in candidate_profiles {
        first_frame_candidate_profile.push(format!(
            "{}:{}b,{}/{},max{}",
            profile.step,
            profile.payload_bit_len,
            profile.nonzero_scale_factors,
            profile.scale_factor_bands,
            profile.max_scale_factor
        ));
        if profile.nonzero_scale_factors > 0 {
            first_nonzero_scale_factor_step = Some((
                profile.step,
                profile.payload_bit_len,
                profile.frame_capacity_bits,
            ));
            break;
        }
    }
    let first_nonzero_scale_factor_step = first_nonzero_scale_factor_step
        .map(|(step, payload_bits, capacity_bits)| {
            format!("{step} (payload_bits={payload_bits}, capacity_bits={capacity_bits})")
        })
        .unwrap_or_else(|| "none".to_owned());
    let mean_scale_factor = if scale_factor_bands == 0 {
        0.0
    } else {
        scale_factor_sum as f64 / scale_factor_bands as f64
    };
    let first_frame_candidate_profile = first_frame_candidate_profile.join("|");

    Ok(format!(
        "MP3 perceptual-scale-factor diagnostic selection: frames={frame_count}, padded_frames={padded_frames}, bitrate_kbps={BITRATE_KBPS}, step_range={min_step}..{max_step}, max_payload_bits={max_payload_bits}, min_capacity_bits={min_capacity_bits}, nonzero_scale_factors={nonzero_scale_factors}/{scale_factor_bands}, max_scale_factor={max_scale_factor}, mean_scale_factor={mean_scale_factor:.2}, first_nonzero_scale_factor_step={first_nonzero_scale_factor_step}, first_frame_candidate_profile=[{first_frame_candidate_profile}]"
    ))
}

fn experimental_aac_lc_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    if expected_pcm.channels != 1 {
        return Err("experimental AAC-LC diagnostic currently expects mono PCM".to_owned());
    }
    let offsets =
        sonare_codec::aac_lc_long_window_scale_factor_band_offsets(expected_pcm.sample_rate)
            .ok_or_else(|| {
                "experimental nonzero encode requires AAC-LC long-window scale-factor band offsets"
                    .to_owned()
            })?;
    let channel_config = sonare_codec::AacLongBlockConfig::new(
        180,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| "AAC-LC scale-factor band count exceeds max_sfb range".to_owned())?,
    );
    let flat_scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
    let channel = sonare_codec::AacScaleFactorChannel::new(channel_config, &flat_scale_factors);
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();
    let spectral_tables = sonare_codec::aac_unsigned_pairs7_unit_magnitude_spectral_tables();
    let encoded = sonare_codec::encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(expected_pcm.sample_rate, 1),
            channel,
            expected_pcm,
            offsets,
            sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            &scale_factor_table,
            spectral_tables,
        )
    .map_err(|err| format!("experimental nonzero encode failed: {err}"))?;
    let path = out_dir.join("aaclc-experimental-nonzero.aac");
    fs::write(&path, encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
}

fn standard_aac_lc_nonzero_encode_diagnostic(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    if expected_pcm.channels != 1 {
        return Err("standard AAC-LC diagnostic currently expects mono PCM".to_owned());
    }
    let offsets =
        sonare_codec::aac_lc_long_window_scale_factor_band_offsets(expected_pcm.sample_rate)
            .ok_or_else(|| {
                "standard nonzero encode requires AAC-LC long-window scale-factor band offsets"
                    .to_owned()
            })?;
    let max_sfb = u8::try_from(offsets.len() - 1)
        .map_err(|_| "AAC-LC scale-factor band count exceeds max_sfb range".to_owned())?;
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();
    let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(1)
        .map_err(|err| format!("AAC-LC default bitrate lookup failed: {err}"))?;
    let budget =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(expected_pcm.sample_rate, bitrate)
            .map_err(|err| format!("AAC-LC default bitrate budget lookup failed: {err}"))?;
    let expected_rms = rms(&expected_pcm.samples);
    let mut best_candidate: Option<AacStandardDiagnosticCandidate> = None;
    for &global_gain in AAC_STANDARD_DIAGNOSTIC_GLOBAL_GAIN_CANDIDATES {
        match evaluate_aac_standard_diagnostic_candidate(
            ffmpeg,
            expected_pcm,
            out_dir,
            offsets,
            max_sfb,
            global_gain,
            budget,
            bitrate,
            &scale_factor_table,
        ) {
            Ok(candidate) => {
                eprintln!(
                    "AAC-LC standard-table diagnostic candidate: global_gain={}, step={}, frame_len={}, decoded_rms={:.4}, best_correlation={:.3}",
                    candidate.global_gain,
                    candidate.selected.step,
                    candidate.selected.frame_len,
                    candidate.quality.decoded_rms,
                    candidate.quality.best_correlation
                );
                best_candidate = match best_candidate {
                    Some(previous)
                        if aac_standard_candidate_is_at_least_as_good(
                            &previous,
                            &candidate,
                            expected_rms,
                        ) =>
                    {
                        Some(previous)
                    }
                    _ => Some(candidate),
                };
            }
            Err(err) => {
                eprintln!(
                    "AAC-LC standard-table diagnostic candidate rejected: global_gain={global_gain}, {err}"
                );
            }
        }
    }
    let best_candidate = best_candidate.ok_or_else(|| {
        "standard-table diagnostic found no FFmpeg-decodable candidate".to_owned()
    })?;
    eprintln!(
        "AAC-LC standard-table diagnostic selection: scale_factor_mode=fixed-search, global_gain={}, step={}, candidate_frame_len={}",
        best_candidate.global_gain, best_candidate.selected.step, best_candidate.selected.frame_len
    );
    let quantized =
        sonare_codec::quantize_pcm_long_block(expected_pcm, 0, 0, best_candidate.selected.step)
            .map_err(|err| format!("standard-table quantized diagnostic failed: {err}"))?;
    let sections = sonare_codec::plan_sections_by_offsets(
        &quantized,
        offsets,
        sonare_codec::aac_lc_standard_spectral_tables(),
    )
    .map_err(|err| format!("standard-table section diagnostic failed: {err}"))?;
    eprintln!(
        "{}",
        aac_section_diagnostic_summary(
            "AAC-LC standard-table diagnostic sections",
            &sections,
            &quantized
        )
    );
    validate_aac_standard_id_offsets_payload_for_diagnostic(&quantized, offsets)?;
    validate_aac_standard_id_offsets_encoded_candidate(
        ffmpeg,
        expected_pcm,
        out_dir,
        offsets,
        max_sfb,
        &best_candidate,
        budget,
        bitrate,
        &scale_factor_table,
    )?;
    validate_aac_standard_id_offsets_stereo_encoded_candidate(
        ffmpeg,
        expected_pcm,
        out_dir,
        offsets,
        max_sfb,
        &best_candidate,
        &scale_factor_table,
    )?;
    let max_frame_len = max_adts_frame_len(&best_candidate.encoded)
        .map_err(|err| format!("standard-table ADTS frame budget inspection failed: {err}"))?;
    validate_adts_frame_budget(
        "AAC-LC standard-table diagnostic",
        max_frame_len,
        budget,
        bitrate,
    )?;
    eprintln!(
        "AAC-LC standard-table diagnostic ADTS frame budget: max_frame_len={max_frame_len}, default_budget={budget}, default_bitrate_bps={bitrate}"
    );
    validate_diagnostic_quality_floor(
        "AAC-LC standard-table diagnostic",
        best_candidate.quality,
        AAC_STANDARD_DIAGNOSTIC_MIN_DECODED_RMS,
        AAC_STANDARD_DIAGNOSTIC_MIN_CORRELATION,
    )?;
    Ok(best_candidate.quality)
}

fn validate_aac_standard_id_high_level_bitrate_surface(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<(LossyOraclePcmQuality, LossyOraclePcmQuality), String> {
    if expected_pcm.channels != 1 {
        return Err("AAC standard-id high-level surface diagnostic expects mono PCM".to_owned());
    }

    let mono_bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(1)
        .map_err(|err| format!("AAC standard-id surface mono bitrate failed: {err}"))?;
    let mono_candidate = select_aac_standard_id_high_level_gain_candidate(
        ffmpeg,
        "AAC-LC standard-id high-level mono ADTS",
        expected_pcm,
        ProductionArtifactKind::Aac,
        mono_bitrate,
        out_dir,
        "aaclc-standard-id-surface-mono",
    )?;
    eprintln!(
        "AAC-LC standard-id high-level mono ADTS selected global_gain={}, max_frame_len={}, decoded_rms={:.4}, best_correlation={:.3}",
        mono_candidate.global_gain,
        mono_candidate.max_frame_len,
        mono_candidate.quality.decoded_rms,
        mono_candidate.quality.best_correlation
    );

    let mono_m4a = sonare_codec::encode_m4a_with_standard_spectral_offsets_and_bitrate(
        expected_pcm,
        mono_bitrate,
        mono_candidate.global_gain,
    )
    .map_err(|err| format!("AAC standard-id surface mono M4A encode failed: {err}"))?;
    let mono_m4a_quality = validate_aac_standard_id_high_level_artifact(
        ffmpeg,
        "AAC-LC standard-id high-level mono M4A",
        expected_pcm,
        &mono_m4a,
        ProductionArtifactKind::M4a,
        mono_bitrate,
        &out_dir.join("aaclc-standard-id-surface-mono.m4a"),
    )?;
    if mono_m4a_quality.best_correlation + f64::EPSILON < mono_candidate.quality.best_correlation {
        return Err(format!(
            "AAC standard-id surface mono M4A quality lagged ADTS: m4a={mono_m4a_quality:?}, adts={:?}",
            mono_candidate.quality
        ));
    }
    let mono_selected_quality = validate_aac_standard_id_high_level_selected_bias_surface(
        ffmpeg,
        "AAC-LC standard-id high-level selected-scale-factor mono",
        expected_pcm,
        mono_bitrate,
        out_dir,
        "aaclc-standard-id-selected-surface-mono",
    )?;
    if mono_selected_quality.best_correlation + f64::EPSILON
        < mono_candidate.quality.best_correlation
    {
        return Err(format!(
            "AAC standard-id selected surface mono quality lagged fixed surface: selected={mono_selected_quality:?}, fixed={:?}",
            mono_candidate.quality
        ));
    }

    let stereo_pcm = aac_standard_surface_stereo_pcm(expected_pcm)?;
    let stereo_bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(2)
        .map_err(|err| format!("AAC standard-id surface stereo bitrate failed: {err}"))?;
    let stereo_candidate = select_aac_standard_id_high_level_gain_candidate(
        ffmpeg,
        "AAC-LC standard-id high-level stereo ADTS",
        &stereo_pcm,
        ProductionArtifactKind::Aac,
        stereo_bitrate,
        out_dir,
        "aaclc-standard-id-surface-stereo",
    )?;
    eprintln!(
        "AAC-LC standard-id high-level stereo ADTS selected global_gain={}, max_frame_len={}, decoded_rms={:.4}, best_correlation={:.3}",
        stereo_candidate.global_gain,
        stereo_candidate.max_frame_len,
        stereo_candidate.quality.decoded_rms,
        stereo_candidate.quality.best_correlation
    );

    let stereo_m4a = sonare_codec::encode_m4a_with_standard_spectral_offsets_and_bitrate(
        &stereo_pcm,
        stereo_bitrate,
        stereo_candidate.global_gain,
    )
    .map_err(|err| format!("AAC standard-id surface stereo M4A encode failed: {err}"))?;
    let stereo_m4a_quality = validate_aac_standard_id_high_level_artifact(
        ffmpeg,
        "AAC-LC standard-id high-level stereo M4A",
        &stereo_pcm,
        &stereo_m4a,
        ProductionArtifactKind::M4a,
        stereo_bitrate,
        &out_dir.join("aaclc-standard-id-surface-stereo.m4a"),
    )?;
    if stereo_m4a_quality.best_correlation + f64::EPSILON
        < stereo_candidate.quality.best_correlation
    {
        return Err(format!(
            "AAC standard-id surface stereo M4A quality lagged ADTS: m4a={stereo_m4a_quality:?}, adts={:?}",
            stereo_candidate.quality
        ));
    }
    let stereo_selected_quality = validate_aac_standard_id_high_level_selected_bias_surface(
        ffmpeg,
        "AAC-LC standard-id high-level selected-scale-factor stereo",
        &stereo_pcm,
        stereo_bitrate,
        out_dir,
        "aaclc-standard-id-selected-surface-stereo",
    )?;
    if stereo_selected_quality.best_correlation + f64::EPSILON
        < stereo_candidate.quality.best_correlation
    {
        return Err(format!(
            "AAC standard-id selected surface stereo quality lagged fixed surface: selected={stereo_selected_quality:?}, fixed={:?}",
            stereo_candidate.quality
        ));
    }

    Ok((mono_selected_quality, stereo_selected_quality))
}

fn aac_standard_surface_stereo_pcm(
    mono_pcm: &sonare_codec::AudioBuffer,
) -> Result<sonare_codec::AudioBuffer, String> {
    if mono_pcm.channels != 1 {
        return Err("AAC standard-id high-level stereo fixture expects mono PCM".to_owned());
    }
    sonare_codec::AudioBuffer::new(
        mono_pcm.sample_rate,
        2,
        mono_pcm
            .samples
            .iter()
            .enumerate()
            .flat_map(|(index, &sample)| {
                let right = if index % 2 == 0 {
                    -sample * 0.75
                } else {
                    sample * 0.5
                };
                [sample, right]
            })
            .collect(),
    )
    .map_err(|err| format!("AAC standard-id high-level stereo PCM failed: {err}"))
}

fn validate_aac_production_benchmark_surface(
    ffmpeg: &OsStr,
    mono_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
) -> Result<(LossyOraclePcmQuality, LossyOraclePcmQuality), String> {
    if mono_pcm.channels != 1 {
        return Err("AAC production benchmark surface expects mono PCM".to_owned());
    }
    let mono_quality = validate_aac_production_benchmark_artifact(
        ffmpeg,
        "AAC-LC production benchmark mono",
        mono_pcm,
        out_dir,
        "aaclc-production-benchmark-mono",
    )?;
    let stereo_pcm = aac_standard_surface_stereo_pcm(mono_pcm)?;
    let stereo_quality = validate_aac_production_benchmark_artifact(
        ffmpeg,
        "AAC-LC production benchmark stereo",
        &stereo_pcm,
        out_dir,
        "aaclc-production-benchmark-stereo",
    )?;
    Ok((mono_quality, stereo_quality))
}

fn validate_aac_production_benchmark_artifact(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    file_stem: &str,
) -> Result<LossyOraclePcmQuality, String> {
    let adts = sonare_codec::encode_with_mode(
        sonare_codec::Format::Aac,
        expected_pcm,
        sonare_codec::EncodeMode::ProductionOnly,
    )
    .map_err(|err| format!("{label} ADTS encode failed: {err}"))?;
    let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
        u8::try_from(expected_pcm.channels)
            .map_err(|_| format!("{label} channel count exceeds AAC production range"))?,
    )
    .map_err(|err| format!("{label} default bitrate failed: {err}"))?;
    let adts_quality = validate_aac_standard_id_high_level_artifact(
        ffmpeg,
        &format!("{label} ADTS"),
        expected_pcm,
        &adts,
        ProductionArtifactKind::Aac,
        bitrate,
        &out_dir.join(format!("{file_stem}.aac")),
    )?;

    let m4a = sonare_codec::mux_aac_adts_as_m4a(&adts)
        .map_err(|err| format!("{label} M4A mux failed: {err}"))?;
    let m4a_quality = validate_aac_standard_id_high_level_artifact(
        ffmpeg,
        &format!("{label} M4A"),
        expected_pcm,
        &m4a,
        ProductionArtifactKind::M4a,
        bitrate,
        &out_dir.join(format!("{file_stem}.m4a")),
    )?;
    if m4a_quality.best_correlation + f64::EPSILON < adts_quality.best_correlation {
        return Err(format!(
            "{label} M4A quality lagged ADTS: m4a={m4a_quality:?}, adts={adts_quality:?}"
        ));
    }
    eprintln!(
        "{label}: adts_rms={:.4}, adts_correlation={:.3}, m4a_rms={:.4}, m4a_correlation={:.3}",
        adts_quality.decoded_rms,
        adts_quality.best_correlation,
        m4a_quality.decoded_rms,
        m4a_quality.best_correlation
    );
    Ok(adts_quality)
}

fn validate_aac_standard_id_high_level_selected_bias_surface(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bitrate: u32,
    out_dir: &Path,
    file_stem: &str,
) -> Result<LossyOraclePcmQuality, String> {
    let expected_rms = rms(&expected_pcm.samples);
    let mut selected: Option<AacStandardSelectedHighLevelCandidate> = None;
    let mut last_rejection: Option<String> = None;
    for &global_gain in AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES {
        for &magnitude_bias in AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES {
            let frame_details = match sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                magnitude_bias,
            ) {
                Ok(details) => details,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: frame detail selection failed: {err}"
                    ));
                    continue;
                }
            };
            let adts = match sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                magnitude_bias,
            ) {
                Ok(adts) => adts,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: ADTS encode failed: {err}"
                    ));
                    continue;
                }
            };
            let adts_quality = match validate_aac_standard_id_high_level_artifact(
                ffmpeg,
                &format!("{label} ADTS gain {global_gain} bias {magnitude_bias}"),
                expected_pcm,
                &adts,
                ProductionArtifactKind::Aac,
                bitrate,
                &out_dir.join(format!(
                    "{file_stem}-gain-{global_gain}-bias-{magnitude_bias}.aac"
                )),
            ) {
                Ok(quality) => quality,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: {err}"
                    ));
                    continue;
                }
            };

            let m4a = match sonare_codec::encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                magnitude_bias,
            ) {
                Ok(m4a) => m4a,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: M4A encode failed: {err}"
                    ));
                    continue;
                }
            };
            let m4a_quality = match validate_aac_standard_id_high_level_artifact(
                ffmpeg,
                &format!("{label} M4A gain {global_gain} bias {magnitude_bias}"),
                expected_pcm,
                &m4a,
                ProductionArtifactKind::M4a,
                bitrate,
                &out_dir.join(format!(
                    "{file_stem}-gain-{global_gain}-bias-{magnitude_bias}.m4a"
                )),
            ) {
                Ok(quality) => quality,
                Err(err) => {
                    last_rejection = Some(format!(
                        "global_gain={global_gain}, magnitude_bias={magnitude_bias}: {err}"
                    ));
                    continue;
                }
            };
            if m4a_quality.best_correlation + f64::EPSILON < adts_quality.best_correlation {
                last_rejection = Some(format!(
                    "global_gain={global_gain}, magnitude_bias={magnitude_bias}: M4A quality lagged ADTS: m4a={m4a_quality:?}, adts={adts_quality:?}"
                ));
                continue;
            }

            let candidate = AacStandardSelectedHighLevelCandidate {
                global_gain,
                magnitude_bias,
                frame_details,
                adts_quality,
                m4a_quality,
            };
            selected = match selected {
                Some(previous)
                    if lossy_oracle_quality_is_at_least_as_good(
                        &previous.adts_quality,
                        &candidate.adts_quality,
                        expected_rms,
                    ) =>
                {
                    Some(previous)
                }
                _ => Some(candidate),
            };
        }
    }

    let selected = selected.ok_or_else(|| {
        format!(
            "{label} found no selected-scale-factor candidate: last rejection={}",
            last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    let step_summary = selected
        .frame_details
        .iter()
        .map(|selection| selection.step.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let selection_summary = aac_step_selection_summary(&selected.frame_details);
    eprintln!(
        "{label}: global_gain={}, scale_factor_magnitude_bias={}, steps=[{}], {}, adts_rms={:.4}, adts_correlation={:.3}, m4a_rms={:.4}, m4a_correlation={:.3}",
        selected.global_gain,
        selected.magnitude_bias,
        step_summary,
        selection_summary,
        selected.adts_quality.decoded_rms,
        selected.adts_quality.best_correlation,
        selected.m4a_quality.decoded_rms,
        selected.m4a_quality.best_correlation
    );
    Ok(selected.adts_quality)
}

fn select_aac_standard_id_high_level_gain_candidate(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    kind: ProductionArtifactKind,
    bitrate: u32,
    out_dir: &Path,
    file_stem: &str,
) -> Result<AacStandardHighLevelCandidate, String> {
    let expected_rms = rms(&expected_pcm.samples);
    let mut selected: Option<AacStandardHighLevelCandidate> = None;
    let mut last_rejection: Option<String> = None;
    for &global_gain in AAC_STANDARD_HIGH_LEVEL_FIXED_SURFACE_GLOBAL_GAIN_CANDIDATES {
        let bytes = match kind {
            ProductionArtifactKind::Aac => {
                match sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_bitrate(
                    expected_pcm,
                    bitrate,
                    global_gain,
                ) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        last_rejection = Some(format!(
                            "{label} global_gain={global_gain} encode failed: {err}"
                        ));
                        eprintln!("{label} candidate rejected: global_gain={global_gain}, {err}");
                        continue;
                    }
                }
            }
            ProductionArtifactKind::M4a => {
                match sonare_codec::encode_m4a_with_standard_spectral_offsets_and_bitrate(
                    expected_pcm,
                    bitrate,
                    global_gain,
                ) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        last_rejection = Some(format!(
                            "{label} global_gain={global_gain} M4A encode failed: {err}"
                        ));
                        eprintln!("{label} candidate rejected: global_gain={global_gain}, {err}");
                        continue;
                    }
                }
            }
            ProductionArtifactKind::Mp3 => {
                return Err(format!("{label} gain sweep received MP3 artifact kind"));
            }
        };
        let extension = match kind {
            ProductionArtifactKind::Aac => "aac",
            ProductionArtifactKind::M4a => "m4a",
            ProductionArtifactKind::Mp3 => unreachable!(),
        };
        let path = out_dir.join(format!("{file_stem}-gain-{global_gain}.{extension}"));
        let quality = match validate_aac_standard_id_high_level_artifact(
            ffmpeg,
            &format!("{label} gain {global_gain}"),
            expected_pcm,
            &bytes,
            kind,
            bitrate,
            &path,
        ) {
            Ok(quality) => quality,
            Err(err) => {
                last_rejection = Some(err.clone());
                eprintln!("{label} candidate rejected: global_gain={global_gain}, {err}");
                continue;
            }
        };
        let adts = match kind {
            ProductionArtifactKind::Aac => bytes,
            ProductionArtifactKind::M4a => match sonare_codec::demux_m4a_as_aac_adts(&bytes) {
                Ok(adts) => adts,
                Err(err) => {
                    last_rejection = Some(format!(
                        "{label} global_gain={global_gain} demux failed: {err}"
                    ));
                    eprintln!(
                        "{label} candidate rejected: global_gain={global_gain}, demux failed: {err}"
                    );
                    continue;
                }
            },
            ProductionArtifactKind::Mp3 => unreachable!(),
        };
        let max_frame_len = match max_adts_frame_len(&adts) {
            Ok(max_frame_len) => max_frame_len,
            Err(err) => {
                last_rejection = Some(format!(
                    "{label} global_gain={global_gain} ADTS inspect failed: {err}"
                ));
                eprintln!(
                    "{label} candidate rejected: global_gain={global_gain}, ADTS inspect failed: {err}"
                );
                continue;
            }
        };
        let candidate = AacStandardHighLevelCandidate {
            global_gain,
            max_frame_len,
            quality,
        };
        selected = match selected {
            Some(previous)
                if lossy_oracle_quality_is_at_least_as_good(
                    &previous.quality,
                    &candidate.quality,
                    expected_rms,
                ) =>
            {
                Some(previous)
            }
            _ => Some(candidate),
        };
    }
    selected.ok_or_else(|| {
        format!(
            "{label} found no global_gain candidate: last rejection={}",
            last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })
}

fn validate_aac_standard_id_high_level_artifact(
    ffmpeg: &OsStr,
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
    kind: ProductionArtifactKind,
    bitrate: u32,
    path: &Path,
) -> Result<LossyOraclePcmQuality, String> {
    let adts = match kind {
        ProductionArtifactKind::Mp3 => {
            return Err(format!(
                "{label} high-level AAC surface received MP3 artifact kind"
            ));
        }
        ProductionArtifactKind::Aac => bytes.to_vec(),
        ProductionArtifactKind::M4a => sonare_codec::demux_m4a_as_aac_adts(bytes)
            .map_err(|err| format!("{label} demux failed: {err}"))?,
    };
    let budget =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(expected_pcm.sample_rate, bitrate)
            .map_err(|err| format!("{label} bitrate budget failed: {err}"))?;
    let max_frame_len = max_adts_frame_len(&adts)
        .map_err(|err| format!("{label} ADTS inspection failed: {err}"))?;
    validate_adts_frame_budget(label, max_frame_len, budget, bitrate)?;

    fs::write(path, bytes).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, path)
        .map_err(|err| format!("{label} FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("{label} FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
        .map_err(|err| format!("{label} PCM quality failed: {err}"))?;
    validate_diagnostic_quality_floor(
        label,
        quality,
        AAC_STANDARD_DIAGNOSTIC_MIN_DECODED_RMS,
        AAC_STANDARD_DIAGNOSTIC_MIN_CORRELATION,
    )?;
    eprintln!(
        "{label}: max_frame_len={max_frame_len}, default_budget={budget}, decoded_rms={:.4}, best_correlation={:.3}",
        quality.decoded_rms, quality.best_correlation
    );
    Ok(quality)
}

#[allow(clippy::too_many_arguments)]
fn validate_aac_standard_id_offsets_encoded_candidate(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    offsets: &[usize],
    max_sfb: u8,
    candidate: &AacStandardDiagnosticCandidate,
    budget: usize,
    bitrate: u32,
    scale_factor_table: &[sonare_codec::HuffmanEntry<sonare_codec::AacScaleFactorDelta>],
) -> Result<(), String> {
    let channel_config = sonare_codec::AacLongBlockConfig::new(candidate.global_gain, max_sfb);
    let frame_count = expected_pcm
        .samples
        .len()
        .div_ceil(usize::from(expected_pcm.channels) * 1024);
    let scale_factors_by_frame = (0..frame_count)
        .map(|_| vec![i16::from(channel_config.global_gain); offsets.len() - 1])
        .collect::<Vec<_>>();
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let mut selected: Option<(f32, Vec<u8>, usize)> = None;
    let mut last_rejection: Option<String> = None;
    let path = out_dir.join(format!(
        "aaclc-standard-id-offsets-gain-{}.aac",
        candidate.global_gain
    ));
    for &step in sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES {
        let encoded = match
            sonare_codec::encode_pcm_mono_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(expected_pcm.sample_rate, 1),
            sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            expected_pcm,
            0,
                step,
            offsets,
            scale_factor_table,
        ) {
            Ok(encoded) => encoded,
            Err(err) => {
                last_rejection = Some(format!("step={step}: {err}"));
                continue;
            }
        };
        let max_frame_len = max_adts_frame_len(&encoded)
            .map_err(|err| format!("AAC standard-id offsets ADTS inspection failed: {err}"))?;
        if max_frame_len <= budget {
            fs::write(&path, &encoded)
                .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            if let Err(err) = run_ffmpeg_clean_acceptance(ffmpeg, &path) {
                last_rejection = Some(format!("step={step}: {err}"));
                continue;
            }
            selected = Some((step, encoded, max_frame_len));
            break;
        }
        last_rejection = Some(format!(
            "step={step}: max_frame_len={max_frame_len} exceeds budget {budget}"
        ));
    }
    let (selected_step, encoded, max_frame_len) = selected.ok_or_else(|| {
        format!(
            "AAC standard-id offsets stream encode diagnostic found no step within budget {budget}: last rejection={}",
            last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    fs::write(&path, &encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    validate_adts_frame_budget(
        "AAC-LC standard-id offsets diagnostic",
        max_frame_len,
        budget,
        bitrate,
    )?;
    eprintln!(
        "AAC-LC standard-id offsets diagnostic ADTS frame budget: selected_step={selected_step}, max_frame_len={max_frame_len}, default_budget={budget}, default_bitrate_bps={bitrate}"
    );

    let expected_rms = rms(&expected_pcm.samples);
    let mut selected_scale_factor_candidate: Option<(
        u8,
        i16,
        Vec<sonare_codec::AacPcmFrameStepSelection>,
        usize,
        LossyOraclePcmQuality,
    )> = None;
    let mut selected_scale_factor_last_rejection: Option<String> = None;
    for &global_gain in AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES {
        for &scale_factor_magnitude_bias in
            AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES
        {
            let selected_scale_factor_details = match sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                scale_factor_magnitude_bias,
            ) {
                Ok(details) => details,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: step selection failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, step selection failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_encoded = match sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                expected_pcm,
                bitrate,
                global_gain,
                scale_factor_magnitude_bias,
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: encode failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, encode failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_path = out_dir.join(format!(
                "aaclc-standard-id-offsets-selected-sf-gain-{global_gain}-bias-{scale_factor_magnitude_bias}.aac"
            ));
            fs::write(&selected_scale_factor_path, &selected_scale_factor_encoded).map_err(
                |err| {
                    format!(
                        "failed to write {}: {err}",
                        selected_scale_factor_path.display()
                    )
                },
            )?;
            if let Err(err) = run_ffmpeg_clean_acceptance(ffmpeg, &selected_scale_factor_path) {
                selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                eprintln!(
                    "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                continue;
            }
            let selected_scale_factor_max_frame_len = match max_adts_frame_len(
                &selected_scale_factor_encoded,
            ) {
                Ok(max_frame_len) => max_frame_len,
                Err(err) => {
                    selected_scale_factor_last_rejection = Some(format!(
                    "global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: ADTS inspection failed: {err}"
                ));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, ADTS inspection failed: {err}"
                    );
                    continue;
                }
            };
            if let Err(err) = validate_adts_frame_budget(
                "AAC-LC standard-id selected-scale-factor offsets diagnostic",
                selected_scale_factor_max_frame_len,
                budget,
                bitrate,
            ) {
                selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                eprintln!(
                    "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                continue;
            }
            let selected_scale_factor_decoded = match run_ffmpeg_decode_f32le(
                ffmpeg,
                &selected_scale_factor_path,
                expected_pcm.sample_rate,
                expected_pcm.channels,
            ) {
                Ok(decoded) => decoded,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: decode failed: {err}"));
                    eprintln!(
                    "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, decode failed: {err}"
                );
                    continue;
                }
            };
            let selected_scale_factor_quality = match validate_lossy_oracle_pcm_quality(
                &expected_pcm.samples,
                &selected_scale_factor_decoded,
            ) {
                Ok(quality) => quality,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                    eprintln!(
                    "AAC-LC standard-id selected-scale-factor offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                    continue;
                }
            };
            selected_scale_factor_candidate = match selected_scale_factor_candidate {
                Some((
                    previous_gain,
                    previous_bias,
                    previous_details,
                    previous_max_frame_len,
                    previous_quality,
                )) if lossy_oracle_quality_is_at_least_as_good(
                    &previous_quality,
                    &selected_scale_factor_quality,
                    expected_rms,
                ) =>
                {
                    Some((
                        previous_gain,
                        previous_bias,
                        previous_details,
                        previous_max_frame_len,
                        previous_quality,
                    ))
                }
                _ => Some((
                    global_gain,
                    scale_factor_magnitude_bias,
                    selected_scale_factor_details,
                    selected_scale_factor_max_frame_len,
                    selected_scale_factor_quality,
                )),
            };
        }
    }
    let (
        selected_scale_factor_global_gain,
        selected_scale_factor_magnitude_bias,
        selected_scale_factor_details,
        selected_scale_factor_max_frame_len,
        selected_scale_factor_quality,
    ) = selected_scale_factor_candidate.ok_or_else(|| {
        format!(
            "AAC standard-id selected-scale-factor diagnostic found no gain candidate: last rejection={}",
            selected_scale_factor_last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    let selected_scale_factor_step_summary = selected_scale_factor_details
        .iter()
        .map(|selection| selection.step.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let selected_scale_factor_selection_summary =
        aac_step_selection_summary(&selected_scale_factor_details);
    eprintln!(
        "AAC-LC standard-id selected-scale-factor offsets diagnostic: global_gain={selected_scale_factor_global_gain}, scale_factor_magnitude_bias={selected_scale_factor_magnitude_bias}, steps=[{selected_scale_factor_step_summary}], {selected_scale_factor_selection_summary}, max_frame_len={selected_scale_factor_max_frame_len}, decoded_rms={:.4}, best_correlation={:.3}",
        selected_scale_factor_quality.decoded_rms,
        selected_scale_factor_quality.best_correlation
    );
    Ok(())
}

fn validate_aac_standard_id_offsets_stereo_encoded_candidate(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    offsets: &[usize],
    max_sfb: u8,
    candidate: &AacStandardDiagnosticCandidate,
    scale_factor_table: &[sonare_codec::HuffmanEntry<sonare_codec::AacScaleFactorDelta>],
) -> Result<(), String> {
    let stereo_pcm = sonare_codec::AudioBuffer::new(
        expected_pcm.sample_rate,
        2,
        expected_pcm
            .samples
            .iter()
            .enumerate()
            .flat_map(|(index, &sample)| {
                let right = if index % 2 == 0 {
                    -sample * 0.75
                } else {
                    sample * 0.5
                };
                [sample, right]
            })
            .collect(),
    )
    .map_err(|err| format!("AAC standard-id offsets stereo diagnostic PCM failed: {err}"))?;
    let channel_config = sonare_codec::AacLongBlockConfig::new(candidate.global_gain, max_sfb);
    let frame_count = stereo_pcm
        .samples
        .len()
        .div_ceil(usize::from(stereo_pcm.channels) * 1024);
    let scale_factors_by_frame = (0..frame_count)
        .map(|_| vec![i16::from(channel_config.global_gain); offsets.len() - 1])
        .collect::<Vec<_>>();
    let scale_factor_refs = scale_factors_by_frame
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(2)
        .map_err(|err| format!("AAC standard-id offsets stereo bitrate failed: {err}"))?;
    let budget =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(stereo_pcm.sample_rate, bitrate)
            .map_err(|err| format!("AAC standard-id offsets stereo budget failed: {err}"))?;
    let mut selected: Option<(f32, Vec<u8>, usize)> = None;
    let mut last_rejection: Option<String> = None;
    let path = out_dir.join(format!(
        "aaclc-standard-id-offsets-stereo-gain-{}.aac",
        candidate.global_gain
    ));
    for &step in sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES {
        let encoded = match sonare_codec::encode_pcm_stereo_long_block_adts_stream_with_standard_spectral_offsets_and_scale_factors_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(stereo_pcm.sample_rate, 2),
            sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            sonare_codec::AacScaleFactorSequence::new(channel_config, &scale_factor_refs),
            &stereo_pcm,
            0,
            step,
            offsets,
            scale_factor_table,
        ) {
            Ok(encoded) => encoded,
            Err(err) => {
                last_rejection = Some(format!("step={step}: {err}"));
                continue;
            }
        };
        let max_frame_len = max_adts_frame_len(&encoded).map_err(|err| {
            format!("AAC standard-id offsets stereo ADTS inspection failed: {err}")
        })?;
        if max_frame_len <= budget {
            fs::write(&path, &encoded)
                .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            if let Err(err) = run_ffmpeg_clean_acceptance(ffmpeg, &path) {
                last_rejection = Some(format!("step={step}: {err}"));
                continue;
            }
            selected = Some((step, encoded, max_frame_len));
            break;
        }
        last_rejection = Some(format!(
            "step={step}: max_frame_len={max_frame_len} exceeds budget {budget}"
        ));
    }
    let (selected_step, encoded, max_frame_len) = selected.ok_or_else(|| {
        format!(
            "AAC standard-id offsets stereo stream encode diagnostic found no step within budget {budget}: last rejection={}",
            last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    fs::write(&path, &encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    validate_adts_frame_budget(
        "AAC-LC standard-id offsets stereo diagnostic",
        max_frame_len,
        budget,
        bitrate,
    )?;
    eprintln!(
        "AAC-LC standard-id offsets stereo diagnostic ADTS frame budget: selected_step={selected_step}, max_frame_len={max_frame_len}, default_budget={budget}, default_bitrate_bps={bitrate}"
    );

    let expected_rms = rms(&stereo_pcm.samples);
    let mut selected_scale_factor_candidate: Option<(
        u8,
        i16,
        Vec<sonare_codec::AacPcmFrameStepSelection>,
        usize,
        LossyOraclePcmQuality,
    )> = None;
    let mut selected_scale_factor_last_rejection: Option<String> = None;
    for &global_gain in AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES {
        for &scale_factor_magnitude_bias in
            AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES
        {
            let selected_scale_factor_details = match sonare_codec::aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
                &stereo_pcm,
                bitrate,
                global_gain,
                scale_factor_magnitude_bias,
            ) {
                Ok(details) => details,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: step selection failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, step selection failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_encoded = match sonare_codec::encode_aac_adts_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
                &stereo_pcm,
                bitrate,
                global_gain,
                scale_factor_magnitude_bias,
            ) {
                Ok(encoded) => encoded,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: encode failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, encode failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_path = out_dir.join(format!(
                "aaclc-standard-id-offsets-stereo-selected-sf-gain-{global_gain}-bias-{scale_factor_magnitude_bias}.aac"
            ));
            fs::write(&selected_scale_factor_path, &selected_scale_factor_encoded).map_err(
                |err| {
                    format!(
                        "failed to write {}: {err}",
                        selected_scale_factor_path.display()
                    )
                },
            )?;
            if let Err(err) = run_ffmpeg_clean_acceptance(ffmpeg, &selected_scale_factor_path) {
                selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                eprintln!(
                    "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                continue;
            }
            let selected_scale_factor_max_frame_len = match max_adts_frame_len(
                &selected_scale_factor_encoded,
            ) {
                Ok(max_frame_len) => max_frame_len,
                Err(err) => {
                    selected_scale_factor_last_rejection = Some(format!(
                            "global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: ADTS inspection failed: {err}"
                        ));
                    eprintln!(
                            "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, ADTS inspection failed: {err}"
                        );
                    continue;
                }
            };
            if let Err(err) = validate_adts_frame_budget(
                "AAC-LC standard-id selected-scale-factor stereo offsets diagnostic",
                selected_scale_factor_max_frame_len,
                budget,
                bitrate,
            ) {
                selected_scale_factor_last_rejection =
                    Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                eprintln!(
                    "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                );
                continue;
            }
            let selected_scale_factor_decoded = match run_ffmpeg_decode_f32le(
                ffmpeg,
                &selected_scale_factor_path,
                stereo_pcm.sample_rate,
                stereo_pcm.channels,
            ) {
                Ok(decoded) => decoded,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: decode failed: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, decode failed: {err}"
                    );
                    continue;
                }
            };
            let selected_scale_factor_quality = match validate_lossy_oracle_pcm_quality(
                &stereo_pcm.samples,
                &selected_scale_factor_decoded,
            ) {
                Ok(quality) => quality,
                Err(err) => {
                    selected_scale_factor_last_rejection =
                        Some(format!("global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}: {err}"));
                    eprintln!(
                        "AAC-LC standard-id selected-scale-factor stereo offsets candidate rejected: global_gain={global_gain}, scale_factor_magnitude_bias={scale_factor_magnitude_bias}, {err}"
                    );
                    continue;
                }
            };
            selected_scale_factor_candidate = match selected_scale_factor_candidate {
                Some((
                    previous_gain,
                    previous_bias,
                    previous_details,
                    previous_max_frame_len,
                    previous_quality,
                )) if lossy_oracle_quality_is_at_least_as_good(
                    &previous_quality,
                    &selected_scale_factor_quality,
                    expected_rms,
                ) =>
                {
                    Some((
                        previous_gain,
                        previous_bias,
                        previous_details,
                        previous_max_frame_len,
                        previous_quality,
                    ))
                }
                _ => Some((
                    global_gain,
                    scale_factor_magnitude_bias,
                    selected_scale_factor_details,
                    selected_scale_factor_max_frame_len,
                    selected_scale_factor_quality,
                )),
            };
        }
    }
    let (
        selected_scale_factor_global_gain,
        selected_scale_factor_magnitude_bias,
        selected_scale_factor_details,
        selected_scale_factor_max_frame_len,
        selected_scale_factor_quality,
    ) = selected_scale_factor_candidate.ok_or_else(|| {
        format!(
            "AAC standard-id selected-scale-factor stereo diagnostic found no gain candidate: last rejection={}",
            selected_scale_factor_last_rejection.unwrap_or_else(|| "none".to_owned())
        )
    })?;
    let selected_scale_factor_step_summary = selected_scale_factor_details
        .iter()
        .map(|selection| selection.step.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let selected_scale_factor_selection_summary =
        aac_step_selection_summary(&selected_scale_factor_details);
    eprintln!(
        "AAC-LC standard-id selected-scale-factor stereo offsets diagnostic: global_gain={selected_scale_factor_global_gain}, scale_factor_magnitude_bias={selected_scale_factor_magnitude_bias}, steps=[{selected_scale_factor_step_summary}], {selected_scale_factor_selection_summary}, max_frame_len={selected_scale_factor_max_frame_len}, decoded_rms={:.4}, best_correlation={:.3}",
        selected_scale_factor_quality.decoded_rms,
        selected_scale_factor_quality.best_correlation
    );
    Ok(())
}

fn validate_aac_standard_id_offsets_payload_for_diagnostic(
    quantized: &[i32],
    offsets: &[usize],
) -> Result<(), String> {
    let sections = sonare_codec::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
        quantized, offsets,
    )
    .map_err(|err| format!("AAC standard-id offsets diagnostic planning failed: {err}"))?;
    let split =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            quantized, offsets,
        )
        .map_err(|err| format!("AAC standard-id offsets diagnostic split failed: {err}"))?;
    let packed =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            quantized, offsets,
        )
        .map_err(|err| format!("AAC standard-id offsets diagnostic packing failed: {err}"))?;
    let expected_bit_len = split
        .section_and_scale_factor_bits
        .bit_len
        .checked_add(split.spectral_bits.bit_len)
        .ok_or_else(|| "AAC standard-id offsets diagnostic bit length overflowed".to_owned())?;
    if packed.bit_len != expected_bit_len {
        return Err(format!(
            "AAC standard-id offsets diagnostic split/packed bit lengths diverged: split={expected_bit_len}, packed={}",
            packed.bit_len
        ));
    }
    if split.spectral_bits.bit_len == 0 {
        return Err("AAC standard-id offsets diagnostic produced empty spectral bits".to_owned());
    }
    eprintln!(
        "{}",
        aac_spectral_section_diagnostic_summary(
            "AAC-LC standard-id offsets diagnostic sections",
            &sections,
            quantized,
            split.section_and_scale_factor_bits.bit_len,
            split.spectral_bits.bit_len,
            packed.bit_len,
        )
    );
    Ok(())
}

fn validate_aac_standard_id_mixed_workbench() -> Result<(), String> {
    let quantized = [1, -1, 0, 1, 17, 0, 0, 0];
    let band_width = 4;
    let offsets = [0, 4, 8];
    let sections =
        sonare_codec::plan_aac_lc_standard_spectral_sections_by_bit_cost(&quantized, band_width)
            .map_err(|err| format!("AAC standard-id mixed workbench planning failed: {err}"))?;
    let flattened = sections
        .iter()
        .flat_map(|section| [section.start, section.end, usize::from(section.codebook_id)])
        .collect::<Vec<_>>();
    if flattened != [0, 4, 4, 4, 8, 11] {
        return Err(format!(
            "AAC standard-id mixed workbench selected unexpected sections: {flattened:?}"
        ));
    }
    let offset_sections =
        sonare_codec::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
            &quantized, &offsets,
        )
        .map_err(|err| format!("AAC standard-id mixed offsets workbench planning failed: {err}"))?;
    let offset_flattened = offset_sections
        .iter()
        .flat_map(|section| [section.start, section.end, usize::from(section.codebook_id)])
        .collect::<Vec<_>>();
    if offset_flattened != flattened {
        return Err(format!(
            "AAC standard-id mixed offsets workbench diverged: offsets={offset_flattened:?}, fixed={flattened:?}"
        ));
    }

    let split = sonare_codec::split_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        &quantized, band_width,
    )
    .map_err(|err| format!("AAC standard-id mixed workbench split failed: {err}"))?;
    let packed = sonare_codec::pack_aac_lc_standard_spectral_payload_with_sign_bits_by_bit_cost(
        &quantized, band_width,
    )
    .map_err(|err| format!("AAC standard-id mixed workbench packing failed: {err}"))?;
    let expected_bit_len = split
        .section_and_scale_factor_bits
        .bit_len
        .checked_add(split.spectral_bits.bit_len)
        .ok_or_else(|| "AAC standard-id mixed workbench bit length overflowed".to_owned())?;
    if packed.bit_len != expected_bit_len {
        return Err(format!(
            "AAC standard-id mixed workbench split/packed bit lengths diverged: split={expected_bit_len}, packed={}",
            packed.bit_len
        ));
    }
    if split.spectral_bits.bit_len == 0 {
        return Err("AAC standard-id mixed workbench produced empty spectral bits".to_owned());
    }
    let offset_split =
        sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            &quantized, &offsets,
        )
        .map_err(|err| format!("AAC standard-id mixed offsets workbench split failed: {err}"))?;
    let offset_packed =
        sonare_codec::pack_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
            &quantized, &offsets,
        )
        .map_err(|err| format!("AAC standard-id mixed offsets workbench packing failed: {err}"))?;
    if offset_split.section_and_scale_factor_bits.bit_len
        != split.section_and_scale_factor_bits.bit_len
        || offset_split.spectral_bits.bit_len != split.spectral_bits.bit_len
        || offset_packed.bit_len != packed.bit_len
    {
        return Err(format!(
            "AAC standard-id mixed offsets workbench bit lengths diverged: fixed=({}, {}, {}), offsets=({}, {}, {})",
            split.section_and_scale_factor_bits.bit_len,
            split.spectral_bits.bit_len,
            packed.bit_len,
            offset_split.section_and_scale_factor_bits.bit_len,
            offset_split.spectral_bits.bit_len,
            offset_packed.bit_len
        ));
    }
    eprintln!(
        "AAC standard-id mixed workbench: sections={flattened:?}, section_bits={}, spectral_bits={}, packed_bits={}, offsets_section_bits={}",
        split.section_and_scale_factor_bits.bit_len,
        split.spectral_bits.bit_len,
        packed.bit_len,
        offset_split.section_and_scale_factor_bits.bit_len
    );
    Ok(())
}

fn validate_diagnostic_quality_floor(
    label: &str,
    quality: LossyOraclePcmQuality,
    min_decoded_rms: f64,
    min_correlation: f64,
) -> Result<(), String> {
    if quality.decoded_rms < min_decoded_rms {
        return Err(format!(
            "{label} decoded RMS regressed below diagnostic floor: decoded_rms={:.4}, min_decoded_rms={min_decoded_rms:.4}",
            quality.decoded_rms
        ));
    }
    if quality.best_correlation < min_correlation {
        return Err(format!(
            "{label} correlation regressed below diagnostic floor: best_correlation={:.3}, min_correlation={min_correlation:.3}",
            quality.best_correlation
        ));
    }
    Ok(())
}

fn validate_aac_standard_id_production_correlation_gap(
    label: &str,
    standard_id_quality: LossyOraclePcmQuality,
    production_quality: LossyOraclePcmQuality,
) -> Result<(), String> {
    let gap = production_quality.best_correlation - standard_id_quality.best_correlation;
    if gap > AAC_STANDARD_ID_MAX_PRODUCTION_CORRELATION_GAP {
        return Err(format!(
            "{label} correlation gap to production exceeded diagnostic limit: standard_id_correlation={:.3}, production_correlation={:.3}, gap={gap:.3}, max_gap={:.3}",
            standard_id_quality.best_correlation,
            production_quality.best_correlation,
            AAC_STANDARD_ID_MAX_PRODUCTION_CORRELATION_GAP
        ));
    }
    Ok(())
}

fn validate_aac_standard_id_rms_control_advantage(
    label: &str,
    standard_id_quality: LossyOraclePcmQuality,
    production_quality: LossyOraclePcmQuality,
    expected_rms: f64,
) -> Result<(), String> {
    let standard_id_error = rms_error(standard_id_quality, expected_rms);
    let production_error = rms_error(production_quality, expected_rms);
    if standard_id_error > production_error {
        return Err(format!(
            "{label} RMS control regressed behind production: standard_id_rms={:.4}, production_rms={:.4}, expected_rms={expected_rms:.4}, standard_id_error={standard_id_error:.4}, production_error={production_error:.4}",
            standard_id_quality.decoded_rms,
            production_quality.decoded_rms
        ));
    }
    Ok(())
}

fn compare_aac_standard_id_to_production_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
) -> Result<AacFrameSelectionComparison, String> {
    compare_aac_standard_id_candidate_set_to_production_frame_selection(
        pcm,
        sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
    )
}

fn compare_aac_standard_id_candidate_set_to_production_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
    candidates: &[f32],
) -> Result<AacFrameSelectionComparison, String> {
    let bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
        u8::try_from(pcm.channels)
            .map_err(|_| "AAC production frame comparison requires mono/stereo PCM".to_owned())?,
    )
    .map_err(|err| format!("AAC default production bitrate lookup failed: {err}"))?;
    let production_details = sonare_codec::aac_selected_scale_factor_frame_details_with_bitrate(
        pcm, bitrate,
    )
    .map_err(|err| format!("AAC production selected-scale-factor frame details failed: {err}"))?;
    let standard_id_details =
        aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
            pcm, bitrate, candidates,
        )?;

    compare_aac_frame_selection_details(&production_details, &standard_id_details)
}

fn aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
    pcm: &sonare_codec::AudioBuffer,
    target_bitrate_bps: u32,
    candidates: &[f32],
) -> Result<Vec<sonare_codec::AacPcmFrameStepSelection>, String> {
    let channels = u8::try_from(pcm.channels)
        .map_err(|_| "AAC standard-id frame comparison requires mono/stereo PCM".to_owned())?;
    let adts = sonare_codec::AdtsConfig::aac_lc(pcm.sample_rate, channels);
    let offsets = sonare_codec::aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or_else(|| "AAC standard-id frame comparison requires AAC-LC offsets".to_owned())?;
    let (global_gain, scale_factor_magnitude_bias) =
        sonare_codec::aac_standard_id_selected_scale_factor_parameters(pcm.channels)
            .map_err(|err| format!("AAC standard-id selected parameters failed: {err}"))?;
    let channel_config = sonare_codec::AacLongBlockConfig::new(
        global_gain,
        u8::try_from(offsets.len() - 1)
            .map_err(|_| "AAC scale-factor band count exceeds u8".to_owned())?,
    );
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();
    let max_frame_len_bytes =
        sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(pcm.sample_rate, target_bitrate_bps)
            .map_err(|err| format!("AAC bitrate frame budget failed: {err}"))?;

    match pcm.channels {
        1 => sonare_codec::select_aac_lc_mono_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
            adts,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            candidates,
            max_frame_len_bytes,
            &scale_factor_table,
        )
        .map_err(|err| {
            format!("AAC mono standard-id selected-scale-factor frame details failed: {err}")
        }),
        2 => sonare_codec::select_aac_lc_stereo_pcm_stream_frame_details_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_max_frame_len_by_bit_cost(
            adts,
            channel_config,
            channel_config,
            pcm,
            0,
            offsets,
            scale_factor_magnitude_bias,
            candidates,
            max_frame_len_bytes,
            &scale_factor_table,
        )
        .map_err(|err| {
            format!("AAC stereo standard-id selected-scale-factor frame details failed: {err}")
        }),
        _ => Err("AAC standard-id frame comparison requires mono/stereo PCM".to_owned()),
    }
}

fn aac_scale_factor_band_index(offsets: &[usize], offset: usize) -> Result<usize, String> {
    offsets
        .iter()
        .position(|candidate| *candidate == offset)
        .ok_or_else(|| format!("AAC offset {offset} is not a scale-factor band boundary"))
}

fn aac_spectral_pairs_for_slice(
    quantized: &[i32],
) -> Result<Vec<sonare_codec::AacSpectralPair>, String> {
    if quantized.len() % 2 != 0 {
        return Err("AAC spectral pair slice length must be even".to_owned());
    }
    quantized
        .chunks_exact(2)
        .map(|pair| {
            Ok(sonare_codec::AacSpectralPair::new(
                i16::try_from(pair[0]).map_err(|_| "AAC spectral pair x exceeds i16".to_owned())?,
                i16::try_from(pair[1]).map_err(|_| "AAC spectral pair y exceeds i16".to_owned())?,
            ))
        })
        .collect()
}

fn aac_spectral_quads_for_slice(
    quantized: &[i32],
) -> Result<Vec<sonare_codec::AacSpectralQuad>, String> {
    if quantized.len() % 4 != 0 {
        return Err("AAC spectral quad slice length must be divisible by four".to_owned());
    }
    quantized
        .chunks_exact(4)
        .map(|quad| {
            Ok(sonare_codec::AacSpectralQuad::new(
                i16::try_from(quad[0]).map_err(|_| "AAC spectral quad v exceeds i16".to_owned())?,
                i16::try_from(quad[1]).map_err(|_| "AAC spectral quad w exceeds i16".to_owned())?,
                i16::try_from(quad[2]).map_err(|_| "AAC spectral quad x exceeds i16".to_owned())?,
                i16::try_from(quad[3]).map_err(|_| "AAC spectral quad y exceeds i16".to_owned())?,
            ))
        })
        .collect()
}

fn aac_standard_id_section_codebook_costs(quantized: &[i32]) -> Result<Vec<(u8, usize)>, String> {
    if quantized.iter().all(|coeff| *coeff == 0) {
        return Ok(vec![(0, 0)]);
    }

    let mut costs = Vec::new();
    if quantized.len() % 4 == 0 {
        let quads = aac_spectral_quads_for_slice(quantized)?;
        for (codebook_id, table) in [
            (1, sonare_codec::aac_signed_quads1_table()),
            (2, sonare_codec::aac_signed_quads2_table()),
        ] {
            if let Ok(packed) = sonare_codec::pack_spectral_quads_with_table(&quads, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
        for (codebook_id, table) in [
            (3, sonare_codec::aac_unsigned_quads3_table()),
            (4, sonare_codec::aac_unsigned_quads4_table()),
        ] {
            if let Ok(packed) = sonare_codec::pack_spectral_quads_with_sign_bits(&quads, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
    }

    if quantized.len() % 2 == 0 {
        let pairs = aac_spectral_pairs_for_slice(quantized)?;
        for (codebook_id, table) in [
            (5, sonare_codec::aac_signed_pairs5_table()),
            (6, sonare_codec::aac_signed_pairs6_table()),
        ] {
            if let Ok(packed) = sonare_codec::pack_spectral_pairs_with_table(&pairs, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
        for (codebook_id, table) in [
            (7, sonare_codec::aac_unsigned_pairs7_table()),
            (8, sonare_codec::aac_unsigned_pairs8_table()),
            (9, sonare_codec::aac_unsigned_pairs9_table()),
            (10, sonare_codec::aac_unsigned_pairs10_table()),
            (11, sonare_codec::aac_escape_table()),
        ] {
            if let Ok(packed) = sonare_codec::pack_spectral_pairs_with_sign_bits(&pairs, table) {
                costs.push((codebook_id, packed.bit_len));
            }
        }
    }

    if costs.is_empty() {
        return Err("AAC section has no packable standard-id codebook candidates".to_owned());
    }
    costs.sort_by_key(|(codebook_id, bit_len)| (*bit_len, *codebook_id));
    costs.dedup_by_key(|(codebook_id, _)| *codebook_id);
    Ok(costs)
}

fn aac_standard_id_payload_breakdown_for_frame_selection(
    pcm: &sonare_codec::AudioBuffer,
    details: &[sonare_codec::AacPcmFrameStepSelection],
) -> Result<AacStandardIdPayloadBreakdown, String> {
    let offsets = sonare_codec::aac_lc_long_window_scale_factor_band_offsets(pcm.sample_rate)
        .ok_or_else(|| "AAC standard-id payload breakdown requires AAC-LC offsets".to_owned())?;
    let (global_gain, scale_factor_magnitude_bias) =
        sonare_codec::aac_standard_id_selected_scale_factor_parameters(pcm.channels)
            .map_err(|err| format!("AAC standard-id selected parameters failed: {err}"))?;
    let scale_factor_table = sonare_codec::aac_scale_factor_delta_table();

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
            .ok_or_else(|| "AAC frame index overflows".to_owned())?;
        for channel in 0..usize::from(pcm.channels) {
            let quantized =
                sonare_codec::quantize_pcm_long_block(pcm, channel, start_frame, detail.step)
                    .map_err(|err| format!("AAC standard-id quantization failed: {err}"))?;
            let planned_sections =
                sonare_codec::plan_aac_lc_standard_spectral_sections_by_offsets_by_bit_cost(
                    &quantized, offsets,
                )
                .map_err(|err| format!("AAC standard-id section planning failed: {err}"))?;
            let scale_factors =
                sonare_codec::select_scale_factors_for_quantized_bands_by_offsets_with_magnitude_bias(
                    &quantized,
                    offsets,
                    i16::from(global_gain),
                    scale_factor_magnitude_bias,
                )
                .map_err(|err| format!("AAC standard-id scale-factor selection failed: {err}"))?;
            let scale_factor_deltas = sonare_codec::plan_spectral_scale_factor_deltas_by_offsets(
                &planned_sections,
                offsets,
                &scale_factors,
                i16::from(global_gain),
            )
            .map_err(|err| format!("AAC standard-id scale-factor delta planning failed: {err}"))?;
            let packed_scale_factors = sonare_codec::pack_scale_factor_deltas_with_table(
                &scale_factor_deltas,
                &scale_factor_table,
            )
            .map_err(|err| format!("AAC standard-id scale-factor packing failed: {err}"))?;
            let split_without_scale_factors =
                sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_by_bit_cost(
                    &quantized,
                    offsets,
                )
                .map_err(|err| format!("AAC standard-id payload split failed: {err}"))?;
            let split_with_scale_factors =
                sonare_codec::split_aac_lc_standard_spectral_payload_with_offsets_and_sign_bits_and_scale_factor_bits_by_bit_cost(
                    &quantized,
                    offsets,
                    packed_scale_factors,
                )
                .map_err(|err| format!("AAC standard-id payload split with scale factors failed: {err}"))?;

            if split_without_scale_factors.spectral_bits.bit_len
                != split_with_scale_factors.spectral_bits.bit_len
            {
                return Err(
                    "AAC standard-id payload split changed spectral bits when adding scale factors"
                        .to_owned(),
                );
            }

            sections += planned_sections.len();
            escape_sections += planned_sections
                .iter()
                .filter(|section| section.codebook_id == 11)
                .count();
            for section in &planned_sections {
                let section_payload =
                    sonare_codec::split_aac_lc_standard_sectioned_spectral_payload_with_offsets_and_sign_bits(
                        std::slice::from_ref(section),
                        &quantized,
                        offsets,
                    )
                    .map_err(|err| {
                        format!("AAC standard-id section-level spectral packing failed: {err}")
                    })?;
                let section_spectral_bits = section_payload.spectral_bits.bit_len;
                let section_max_abs = quantized[section.start..section.end]
                    .iter()
                    .map(|coeff| coeff.checked_abs())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| "AAC spectral coefficient overflows".to_owned())?
                    .into_iter()
                    .max()
                    .unwrap_or(0);
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
            max_abs = max_abs.max(
                quantized
                    .iter()
                    .map(|coeff| coeff.checked_abs())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| "AAC spectral coefficient overflows".to_owned())?
                    .into_iter()
                    .max()
                    .unwrap_or(0),
            );
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
                .ok_or_else(|| "AAC scale-factor bit count underflowed".to_owned())?;
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

fn compare_aac_frame_selection_details(
    production_details: &[sonare_codec::AacPcmFrameStepSelection],
    standard_id_details: &[sonare_codec::AacPcmFrameStepSelection],
) -> Result<AacFrameSelectionComparison, String> {
    if production_details.len() != standard_id_details.len() {
        return Err(format!(
            "AAC standard-id frame count diverged from production: production={}, standard_id={}",
            production_details.len(),
            standard_id_details.len()
        ));
    }
    if production_details.is_empty() {
        return Err("AAC frame selection comparison requires at least one frame".to_owned());
    }

    let production_max_frame_len = production_details
        .iter()
        .map(|selection| selection.frame_len)
        .max()
        .unwrap_or(0);
    let standard_id_max_frame_len = standard_id_details
        .iter()
        .map(|selection| selection.frame_len)
        .max()
        .unwrap_or(0);
    let production_min_budget_slack = production_details
        .iter()
        .map(|selection| {
            selection
                .frame_capacity_bytes
                .saturating_sub(selection.frame_len)
        })
        .min()
        .unwrap_or(0);
    let standard_id_min_budget_slack = standard_id_details
        .iter()
        .map(|selection| {
            selection
                .frame_capacity_bytes
                .saturating_sub(selection.frame_len)
        })
        .min()
        .unwrap_or(0);
    let production_max_step = production_details
        .iter()
        .map(|selection| selection.step)
        .fold(0.0_f32, f32::max);
    let standard_id_max_step = standard_id_details
        .iter()
        .map(|selection| selection.step)
        .fold(0.0_f32, f32::max);

    Ok(AacFrameSelectionComparison {
        frames: production_details.len(),
        production_max_frame_len,
        standard_id_max_frame_len,
        max_frame_len_delta: standard_id_max_frame_len as isize - production_max_frame_len as isize,
        production_min_budget_slack,
        standard_id_min_budget_slack,
        min_budget_slack_delta: standard_id_min_budget_slack as isize
            - production_min_budget_slack as isize,
        production_max_step,
        standard_id_max_step,
        max_step_delta: standard_id_max_step - production_max_step,
    })
}

fn validate_mp3_perceptual_reservoir_production_correlation_gap(
    label: &str,
    reservoir_quality: LossyOraclePcmQuality,
    production_quality: LossyOraclePcmQuality,
) -> Result<(), String> {
    let gap = production_quality.best_correlation - reservoir_quality.best_correlation;
    if gap > MP3_PERCEPTUAL_RESERVOIR_MAX_PRODUCTION_CORRELATION_GAP {
        return Err(format!(
            "{label} correlation gap to production exceeded diagnostic limit: reservoir_correlation={:.3}, production_correlation={:.3}, gap={gap:.3}, max_gap={:.3}",
            reservoir_quality.best_correlation,
            production_quality.best_correlation,
            MP3_PERCEPTUAL_RESERVOIR_MAX_PRODUCTION_CORRELATION_GAP
        ));
    }
    Ok(())
}

fn aac_standard_candidate_is_at_least_as_good(
    previous: &AacStandardDiagnosticCandidate,
    candidate: &AacStandardDiagnosticCandidate,
    expected_rms: f64,
) -> bool {
    lossy_oracle_quality_is_at_least_as_good(&previous.quality, &candidate.quality, expected_rms)
}

fn lossy_oracle_quality_is_at_least_as_good(
    previous: &LossyOraclePcmQuality,
    candidate: &LossyOraclePcmQuality,
    expected_rms: f64,
) -> bool {
    let correlation_delta = previous.best_correlation - candidate.best_correlation;
    if correlation_delta.abs() > 1.0e-6 {
        return correlation_delta > 0.0;
    }
    let previous_rms_error = (previous.decoded_rms - expected_rms).abs();
    let candidate_rms_error = (candidate.decoded_rms - expected_rms).abs();
    previous_rms_error <= candidate_rms_error
}

fn rms_error(quality: LossyOraclePcmQuality, expected_rms: f64) -> f64 {
    (quality.decoded_rms - expected_rms).abs()
}

fn aac_step_selection_summary(details: &[sonare_codec::AacPcmFrameStepSelection]) -> String {
    let frames = details.len();
    let min_step = details
        .iter()
        .map(|selection| selection.step)
        .fold(f32::INFINITY, f32::min);
    let max_step = details
        .iter()
        .map(|selection| selection.step)
        .fold(0.0_f32, f32::max);
    let max_frame_len = details
        .iter()
        .map(|selection| selection.frame_len)
        .max()
        .unwrap_or(0);
    let min_budget_slack = details
        .iter()
        .map(|selection| {
            selection
                .frame_capacity_bytes
                .saturating_sub(selection.frame_len)
        })
        .min()
        .unwrap_or(0);
    format!(
        "frames={frames}, min_step={min_step}, max_step={max_step}, max_frame_len={max_frame_len}, min_budget_slack={min_budget_slack}"
    )
}

struct AacStandardDiagnosticCandidate {
    global_gain: u8,
    selected: sonare_codec::AacPcmFrameStepSelection,
    encoded: Vec<u8>,
    quality: LossyOraclePcmQuality,
}

struct AacStandardHighLevelCandidate {
    global_gain: u8,
    max_frame_len: usize,
    quality: LossyOraclePcmQuality,
}

struct AacStandardSelectedHighLevelCandidate {
    global_gain: u8,
    magnitude_bias: i16,
    frame_details: Vec<sonare_codec::AacPcmFrameStepSelection>,
    adts_quality: LossyOraclePcmQuality,
    m4a_quality: LossyOraclePcmQuality,
}

#[allow(clippy::too_many_arguments)]
fn evaluate_aac_standard_diagnostic_candidate(
    ffmpeg: &OsStr,
    expected_pcm: &sonare_codec::AudioBuffer,
    out_dir: &Path,
    offsets: &[usize],
    max_sfb: u8,
    global_gain: u8,
    budget: usize,
    bitrate: u32,
    scale_factor_table: &[sonare_codec::HuffmanEntry<sonare_codec::AacScaleFactorDelta>],
) -> Result<AacStandardDiagnosticCandidate, String> {
    let channel_config = sonare_codec::AacLongBlockConfig::new(global_gain, max_sfb);
    let flat_scale_factors = vec![i16::from(channel_config.global_gain); offsets.len() - 1];
    let channel = sonare_codec::AacScaleFactorChannel::new(channel_config, &flat_scale_factors);
    let selected =
        sonare_codec::select_aac_lc_mono_pcm_frame_step_details_with_offsets_and_scale_factors_and_max_frame_len_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(expected_pcm.sample_rate, 1),
            channel,
            expected_pcm,
            0,
            offsets,
            sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            budget,
            scale_factor_table,
            sonare_codec::aac_lc_standard_spectral_tables(),
        )
        .map_err(|err| format!("standard-table step selection failed: {err}"))?;
    let encoded = sonare_codec::encode_pcm_mono_long_block_adts_stream_with_offsets_and_scale_factors_and_bitrate_by_bit_cost(
            sonare_codec::AdtsConfig::aac_lc(expected_pcm.sample_rate, 1),
            channel,
            expected_pcm,
            offsets,
            sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            bitrate,
            scale_factor_table,
            sonare_codec::aac_lc_standard_spectral_tables(),
        )
    .map_err(|err| format!("standard-table nonzero encode failed: {err}"))?;
    let path = out_dir.join(format!(
        "aaclc-standard-table-nonzero-gain-{global_gain}.aac"
    ));
    fs::write(&path, &encoded)
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    run_ffmpeg_acceptance(ffmpeg, &path)
        .map_err(|err| format!("FFmpeg acceptance failed: {err}"))?;
    let decoded = run_ffmpeg_decode_f32le(
        ffmpeg,
        &path,
        expected_pcm.sample_rate,
        expected_pcm.channels,
    )
    .map_err(|err| format!("FFmpeg PCM decode failed: {err}"))?;
    let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)?;
    Ok(AacStandardDiagnosticCandidate {
        global_gain,
        selected,
        encoded,
        quality,
    })
}

fn aac_section_diagnostic_summary(
    label: &str,
    sections: &[sonare_codec::AacSection],
    quantized: &[i32],
) -> String {
    let mut zero_bands = 0usize;
    let mut unsigned7_bands = 0usize;
    let mut unsigned8_bands = 0usize;
    let mut unsigned9_bands = 0usize;
    let mut unsigned10_bands = 0usize;
    let mut escape_bands = 0usize;
    let mut signed_or_other_bands = 0usize;
    let mut max_abs = 0i32;
    let mut max_nonzero_section_width = 0usize;
    for section in sections {
        let width = section.end.saturating_sub(section.start);
        let section_max = quantized
            .get(section.start..section.end)
            .unwrap_or(&[])
            .iter()
            .filter_map(|coeff| coeff.checked_abs())
            .max()
            .unwrap_or(0);
        max_abs = max_abs.max(section_max);
        if section.codebook != sonare_codec::AacCodebook::Zero {
            max_nonzero_section_width = max_nonzero_section_width.max(width);
        }
        match section.codebook {
            sonare_codec::AacCodebook::Zero => zero_bands += 1,
            sonare_codec::AacCodebook::UnsignedPairs7 => unsigned7_bands += 1,
            sonare_codec::AacCodebook::UnsignedPairs8 => unsigned8_bands += 1,
            sonare_codec::AacCodebook::UnsignedPairs9 => unsigned9_bands += 1,
            sonare_codec::AacCodebook::UnsignedPairs10 => unsigned10_bands += 1,
            sonare_codec::AacCodebook::Escape => escape_bands += 1,
            _ => signed_or_other_bands += 1,
        }
    }
    format!(
        "{label}: sections={}, zero={}, unsigned7={}, unsigned8={}, unsigned9={}, unsigned10={}, escape={}, signed_or_other={}, max_abs={}, max_nonzero_width={}",
        sections.len(),
        zero_bands,
        unsigned7_bands,
        unsigned8_bands,
        unsigned9_bands,
        unsigned10_bands,
        escape_bands,
        signed_or_other_bands,
        max_abs,
        max_nonzero_section_width
    )
}

fn aac_spectral_section_diagnostic_summary(
    label: &str,
    sections: &[sonare_codec::AacSpectralSection],
    quantized: &[i32],
    section_bits: usize,
    spectral_bits: usize,
    packed_bits: usize,
) -> String {
    let mut zero_sections = 0usize;
    let mut quad_sections = 0usize;
    let mut signed_pair_sections = 0usize;
    let mut unsigned_pair_sections = 0usize;
    let mut escape_sections = 0usize;
    let mut max_abs = 0i32;
    let mut max_nonzero_section_width = 0usize;
    for section in sections {
        let width = section.end.saturating_sub(section.start);
        let section_max = quantized
            .get(section.start..section.end)
            .unwrap_or(&[])
            .iter()
            .filter_map(|coeff| coeff.checked_abs())
            .max()
            .unwrap_or(0);
        max_abs = max_abs.max(section_max);
        if section.codebook_id != 0 {
            max_nonzero_section_width = max_nonzero_section_width.max(width);
        }
        match section.codebook_id {
            0 => zero_sections += 1,
            1..=4 => quad_sections += 1,
            5 | 6 => signed_pair_sections += 1,
            7..=10 => unsigned_pair_sections += 1,
            11 => escape_sections += 1,
            _ => {}
        }
    }
    format!(
        "{label}: sections={}, zero={}, quad={}, signed_pairs={}, unsigned_pairs={}, escape={}, max_abs={}, max_nonzero_width={}, section_bits={}, spectral_bits={}, packed_bits={}",
        sections.len(),
        zero_sections,
        quad_sections,
        signed_pair_sections,
        unsigned_pair_sections,
        escape_sections,
        max_abs,
        max_nonzero_section_width,
        section_bits,
        spectral_bits,
        packed_bits
    )
}

fn verify_production_lossy_oracle_acceptance(
    ffmpeg: OsString,
    artifacts: &[(
        &str,
        ProductionArtifactKind,
        sonare_codec::AudioBuffer,
        Vec<u8>,
    )],
) -> Result<(), String> {
    let out_dir = env::temp_dir().join(format!(
        "sonare-codec-production-readiness-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis())
    ));
    fs::create_dir_all(&out_dir)
        .map_err(|err| format!("failed to create {}: {err}", out_dir.display()))?;

    for (label, kind, expected_pcm, bytes) in artifacts {
        verify_mp3_default_production_budget(label, *kind, expected_pcm, bytes)?;
        verify_aac_default_production_budget(label, *kind, expected_pcm, bytes)?;

        let extension = kind.extension();
        let path = out_dir.join(format!(
            "{}.{}",
            label.to_ascii_lowercase().replace('-', ""),
            extension
        ));
        fs::write(&path, bytes)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        run_ffmpeg_acceptance(&ffmpeg, &path)
            .map_err(|err| format!("{label} production oracle acceptance failed: {err}"))?;
        let decoded = run_ffmpeg_decode_f32le(
            &ffmpeg,
            &path,
            expected_pcm.sample_rate,
            expected_pcm.channels,
        )
        .map_err(|err| format!("{label} production oracle PCM decode failed: {err}"))?;
        let quality = validate_lossy_oracle_pcm_quality(&expected_pcm.samples, &decoded)
            .map_err(|err| format!("{label} production oracle PCM quality failed: {err}"))?;
        let min_correlation = production_lossy_min_correlation(*kind, expected_pcm.channels)?;
        if quality.best_correlation < min_correlation {
            return Err(format!(
                "{label} production oracle PCM quality regressed below floor {min_correlation:.3}: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            ));
        }
        eprintln!(
            "{label} production oracle PCM quality: decoded_rms={:.4}, best_correlation={:.3}, min_correlation={min_correlation:.3}",
            quality.decoded_rms, quality.best_correlation
        );
    }

    fs::remove_dir_all(&out_dir)
        .map_err(|err| format!("failed to remove {}: {err}", out_dir.display()))
}

fn production_lossy_min_correlation(
    kind: ProductionArtifactKind,
    channels: u16,
) -> Result<f64, String> {
    match (kind, channels) {
        (ProductionArtifactKind::Mp3, 1) => Ok(MP3_PRODUCTION_MONO_MIN_CORRELATION),
        (ProductionArtifactKind::Mp3, 2) => Ok(MP3_PRODUCTION_STEREO_MIN_CORRELATION),
        (ProductionArtifactKind::Aac | ProductionArtifactKind::M4a, 1 | 2) => {
            Ok(AAC_PRODUCTION_MIN_CORRELATION)
        }
        (ProductionArtifactKind::Mp3, _) => {
            Err("MP3 production oracle floor supports mono/stereo only".to_owned())
        }
        (ProductionArtifactKind::Aac | ProductionArtifactKind::M4a, _) => {
            Err("AAC-LC production oracle floor supports mono/stereo only".to_owned())
        }
    }
}

fn verify_mp3_default_production_budget(
    label: &str,
    kind: ProductionArtifactKind,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    if !matches!(kind, ProductionArtifactKind::Mp3) {
        return Ok(());
    }
    verify_mp3_cbr_bitrate_budget(label, 128, expected_pcm, bytes)?;
    verify_mp3_production_reservoir(label, expected_pcm, bytes)
}

fn verify_mp3_production_reservoir(
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    let reservoir_details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
            expected_pcm,
            sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| {
            format!("{label} MP3 entropy-targeted perceptual reservoir detail selection failed: {err}")
        })?;

    let mut offset = 0usize;
    let mut frame_count = 0usize;
    let mut max_main_data_begin = 0u32;
    while offset < bytes.len() {
        let header = sonare_codec::FrameHeader::parse(&bytes[offset..])
            .map_err(|err| format!("{label} MP3 reservoir check failed: {err}"))?;
        let Some(detail) = reservoir_details.get(frame_count) else {
            return Err(format!(
                "{label} MP3 reservoir check failed: encoded stream has more frames than selector details"
            ));
        };
        let borrowed_budget_bits = detail
            .frame_capacity_bytes
            .checked_add(detail.main_data_begin)
            .and_then(|bytes| bytes.checked_mul(8))
            .ok_or_else(|| format!("{label} MP3 reservoir detail budget overflows"))?;
        if detail.payload_bit_len > borrowed_budget_bits {
            return Err(format!(
                "{label} MP3 reservoir check failed: selector detail frame {frame_count} payload_bits={} exceeds borrowed budget {borrowed_budget_bits}",
                detail.payload_bit_len
            ));
        }
        let side_info_offset = offset
            .checked_add(4)
            .ok_or_else(|| format!("{label} MP3 reservoir check offset overflows"))?;
        if side_info_offset + 1 >= bytes.len() {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame side-info extends past stream length {}",
                bytes.len()
            ));
        }
        let main_data_begin = (u32::from(bytes[side_info_offset]) << 1)
            | (u32::from(bytes[side_info_offset + 1]) >> 7);
        if detail.main_data_begin != main_data_begin as usize {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame {frame_count} side-info main_data_begin={main_data_begin} does not match selector detail {}",
                detail.main_data_begin
            ));
        }
        max_main_data_begin = max_main_data_begin.max(main_data_begin);
        offset = offset
            .checked_add(header.frame_len())
            .ok_or_else(|| format!("{label} MP3 reservoir check frame length overflows"))?;
        frame_count += 1;
    }
    if frame_count != reservoir_details.len() {
        return Err(format!(
            "{label} MP3 reservoir check failed: encoded frame_count={frame_count} does not match selector detail count {}",
            reservoir_details.len()
        ));
    }
    if max_main_data_begin == 0 {
        return Err(format!(
            "{label} MP3 reservoir check failed: production stream never used main_data_begin"
        ));
    }
    let granules_per_frame = if expected_pcm.channels == 1 {
        2_usize
    } else {
        4_usize
    };
    for (frame_index, detail) in reservoir_details.iter().enumerate() {
        if detail.perceptual_granules + detail.calibrated_granules != granules_per_frame {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame {frame_index} granule telemetry is inconsistent: perceptual={}, calibrated={}, expected={granules_per_frame}",
                detail.perceptual_granules, detail.calibrated_granules
            ));
        }
        if detail.quality_guard_compared_granules != 0
            || detail.quality_guard_distortion_delta != 0.0
        {
            return Err(format!(
                "{label} MP3 reservoir check failed: production unexpectedly reported quality guard telemetry on frame {frame_index}"
            ));
        }
        if detail.entropy_target_bits == 0 {
            return Err(format!(
                "{label} MP3 reservoir check failed: frame {frame_index} did not receive entropy target bits"
            ));
        }
    }
    let max_reservoir_after = reservoir_details
        .iter()
        .map(|detail| detail.reservoir_after)
        .max()
        .unwrap_or(0);
    let min_step = reservoir_details
        .iter()
        .map(|detail| detail.step)
        .fold(f32::INFINITY, f32::min);
    let max_payload_bits = reservoir_details
        .iter()
        .map(|detail| detail.payload_bit_len)
        .max()
        .unwrap_or(0);
    let perceptual_granules = reservoir_details
        .iter()
        .map(|detail| detail.perceptual_granules)
        .sum::<usize>();
    let calibrated_granules = reservoir_details
        .iter()
        .map(|detail| detail.calibrated_granules)
        .sum::<usize>();
    let quality_guard_compared_granules = reservoir_details
        .iter()
        .map(|detail| detail.quality_guard_compared_granules)
        .sum::<usize>();
    let quality_guard_distortion_delta = reservoir_details
        .iter()
        .map(|detail| detail.quality_guard_distortion_delta)
        .sum::<f64>();
    let entropy_target_bits = reservoir_details
        .iter()
        .map(|detail| detail.entropy_target_bits)
        .sum::<usize>();
    let capacity_bits = reservoir_details
        .iter()
        .map(|detail| detail.frame_capacity_bytes * 8)
        .sum::<usize>();
    if entropy_target_bits != capacity_bits {
        return Err(format!(
            "{label} MP3 reservoir check failed: entropy target bits {entropy_target_bits} did not match capacity bits {capacity_bits}"
        ));
    }
    let entropy_target_budget_frames = reservoir_details
        .iter()
        .filter(|detail| detail.used_entropy_target_budget)
        .count();
    if entropy_target_budget_frames == 0 {
        return Err(format!(
            "{label} MP3 reservoir check failed: no frame used the entropy target budget path"
        ));
    }
    eprintln!(
        "{label} MP3 production entropy-targeted reservoir: min_step={min_step}, max_payload_bits={max_payload_bits}, max_main_data_begin={max_main_data_begin}, max_reservoir_after={max_reservoir_after}, perceptual_granules={perceptual_granules}, calibrated_granules={calibrated_granules}, quality_guard_compared_granules={quality_guard_compared_granules}, quality_guard_distortion_delta={quality_guard_distortion_delta:.9e}, entropy_target_bits={entropy_target_bits}, entropy_target_budget_frames={entropy_target_budget_frames}"
    );
    Ok(())
}

fn verify_mp3_perceptual_reservoir(
    label: &str,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    let reservoir_details =
        sonare_codec::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
            expected_pcm,
            MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .map_err(|err| {
            format!("{label} MP3 perceptual reservoir detail selection failed: {err}")
        })?;

    let mut offset = 0usize;
    let mut frame_count = 0usize;
    let mut max_main_data_begin = 0u32;
    while offset < bytes.len() {
        let header = sonare_codec::FrameHeader::parse(&bytes[offset..])
            .map_err(|err| format!("{label} MP3 perceptual reservoir check failed: {err}"))?;
        let Some(detail) = reservoir_details.get(frame_count) else {
            return Err(format!(
                "{label} MP3 perceptual reservoir check failed: encoded stream has more frames than selector details"
            ));
        };
        let borrowed_budget_bits = detail
            .frame_capacity_bytes
            .checked_add(detail.main_data_begin)
            .and_then(|bytes| bytes.checked_mul(8))
            .ok_or_else(|| format!("{label} MP3 perceptual reservoir detail budget overflows"))?;
        if detail.payload_bit_len > borrowed_budget_bits {
            return Err(format!(
                "{label} MP3 perceptual reservoir check failed: selector detail frame {frame_count} payload_bits={} exceeds borrowed budget {borrowed_budget_bits}",
                detail.payload_bit_len
            ));
        }
        let side_info_offset = offset
            .checked_add(4)
            .ok_or_else(|| format!("{label} MP3 perceptual reservoir check offset overflows"))?;
        if side_info_offset + 1 >= bytes.len() {
            return Err(format!(
                "{label} MP3 perceptual reservoir check failed: frame side-info extends past stream length {}",
                bytes.len()
            ));
        }
        let main_data_begin = (u32::from(bytes[side_info_offset]) << 1)
            | (u32::from(bytes[side_info_offset + 1]) >> 7);
        if detail.main_data_begin != main_data_begin as usize {
            return Err(format!(
                "{label} MP3 perceptual reservoir check failed: frame {frame_count} side-info main_data_begin={main_data_begin} does not match selector detail {}",
                detail.main_data_begin
            ));
        }
        max_main_data_begin = max_main_data_begin.max(main_data_begin);
        offset = offset.checked_add(header.frame_len()).ok_or_else(|| {
            format!("{label} MP3 perceptual reservoir check frame length overflows")
        })?;
        frame_count += 1;
    }
    if frame_count != reservoir_details.len() {
        return Err(format!(
            "{label} MP3 perceptual reservoir check failed: encoded frame_count={frame_count} does not match selector detail count {}",
            reservoir_details.len()
        ));
    }
    if max_main_data_begin == 0 {
        return Err(format!(
            "{label} MP3 perceptual reservoir check failed: stream never used main_data_begin"
        ));
    }
    let max_reservoir_after = reservoir_details
        .iter()
        .map(|detail| detail.reservoir_after)
        .max()
        .unwrap_or(0);
    let min_step = reservoir_details
        .iter()
        .map(|detail| detail.step)
        .fold(f32::INFINITY, f32::min);
    let max_payload_bits = reservoir_details
        .iter()
        .map(|detail| detail.payload_bit_len)
        .max()
        .unwrap_or(0);
    let perceptual_granules = reservoir_details
        .iter()
        .map(|detail| detail.perceptual_granules)
        .sum::<usize>();
    let calibrated_granules = reservoir_details
        .iter()
        .map(|detail| detail.calibrated_granules)
        .sum::<usize>();
    let quality_guard_compared_granules = reservoir_details
        .iter()
        .map(|detail| detail.quality_guard_compared_granules)
        .sum::<usize>();
    let quality_guard_distortion_delta = reservoir_details
        .iter()
        .map(|detail| detail.quality_guard_distortion_delta)
        .sum::<f64>();
    eprintln!(
        "{label} MP3 perceptual reservoir: min_step={min_step}, max_payload_bits={max_payload_bits}, max_main_data_begin={max_main_data_begin}, max_reservoir_after={max_reservoir_after}, perceptual_granules={perceptual_granules}, calibrated_granules={calibrated_granules}, quality_guard_compared_granules={quality_guard_compared_granules}, quality_guard_distortion_delta={quality_guard_distortion_delta:.9e}"
    );
    Ok(())
}

fn verify_mp3_cbr_bitrate_budget(
    label: &str,
    bitrate_kbps: u16,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    let expected_header = sonare_codec::layer3_header_for_capacity(
        expected_pcm.sample_rate,
        expected_pcm.channels,
        bitrate_kbps,
        false,
        false,
    )
    .map_err(|err| format!("{label} MP3 CBR budget failed: {err}"))?;
    let expected_frames = expected_pcm
        .frames()
        .div_ceil(usize::from(expected_header.samples_per_frame()))
        .max(1);
    let slot_remainder = 144 * usize::from(bitrate_kbps) * 1000 % expected_pcm.sample_rate as usize;

    let mut offset = 0usize;
    let mut frame_count = 0usize;
    let mut padding_accumulator = 0usize;
    let mut padded_frames = 0usize;
    while offset < bytes.len() {
        let mut expected_frame_header = expected_header;
        padding_accumulator += slot_remainder;
        if padding_accumulator >= expected_pcm.sample_rate as usize {
            padding_accumulator -= expected_pcm.sample_rate as usize;
            expected_frame_header.padding = true;
            padded_frames += 1;
        }
        let header = sonare_codec::FrameHeader::parse(&bytes[offset..])
            .map_err(|err| format!("{label} MP3 CBR budget failed: {err}"))?;
        if header != expected_frame_header {
            return Err(format!(
                "{label} MP3 CBR budget failed: frame {frame_count} header {header:?} does not match expected {bitrate_kbps}kbps CBR header {expected_frame_header:?}"
            ));
        }
        let frame_len = header.frame_len();
        let expected_frame_len = expected_frame_header.frame_len();
        if frame_len != expected_frame_len {
            return Err(format!(
                "{label} MP3 CBR budget failed: frame {frame_count} length {frame_len} does not match expected {expected_frame_len}"
            ));
        }
        let next = offset
            .checked_add(frame_len)
            .ok_or_else(|| format!("{label} MP3 CBR frame length overflows"))?;
        if next > bytes.len() {
            return Err(format!(
                "{label} MP3 CBR budget failed: frame {frame_count} extends past stream length {}",
                bytes.len()
            ));
        }
        let capacity = sonare_codec::layer3_main_data_capacity_bytes(header)
            .map_err(|err| format!("{label} MP3 CBR capacity failed: {err}"))?;
        let expected_capacity =
            sonare_codec::layer3_main_data_capacity_bytes(expected_frame_header)
                .map_err(|err| format!("{label} MP3 CBR capacity failed: {err}"))?;
        if capacity != expected_capacity {
            return Err(format!(
                "{label} MP3 CBR budget failed: frame {frame_count} capacity {capacity} does not match expected {expected_capacity}"
            ));
        }
        frame_count += 1;
        offset = next;
    }

    if frame_count == 0 {
        return Err(format!(
            "{label} MP3 CBR budget failed: stream has no complete frames"
        ));
    }
    if frame_count != expected_frames {
        return Err(format!(
            "{label} MP3 CBR budget failed: frame_count={frame_count} does not match expected {expected_frames}"
        ));
    }

    eprintln!(
        "{label} MP3 CBR budget: frames={frame_count}, padded_frames={padded_frames}, bitrate_kbps={bitrate_kbps}"
    );
    Ok(())
}

fn verify_aac_default_production_budget(
    label: &str,
    kind: ProductionArtifactKind,
    expected_pcm: &sonare_codec::AudioBuffer,
    bytes: &[u8],
) -> Result<(), String> {
    let adts = match kind {
        ProductionArtifactKind::Mp3 => return Ok(()),
        ProductionArtifactKind::Aac => bytes.to_vec(),
        ProductionArtifactKind::M4a => sonare_codec::demux_m4a_as_aac_adts(bytes)
            .map_err(|err| format!("{label} production M4A demux for budget failed: {err}"))?,
    };
    let default_bitrate = sonare_codec::aac_lc_default_production_bitrate_bps(
        u8::try_from(expected_pcm.channels)
            .map_err(|_| format!("{label} production channel count exceeds AAC-LC range"))?,
    )
    .map_err(|err| format!("{label} production AAC default bitrate failed: {err}"))?;
    let max_budget = sonare_codec::aac_lc_adts_max_frame_len_for_bitrate(
        expected_pcm.sample_rate,
        default_bitrate,
    )
    .map_err(|err| format!("{label} production AAC frame budget failed: {err}"))?;
    let max_frame_len = max_adts_frame_len(&adts)
        .map_err(|err| format!("{label} production ADTS frame budget failed: {err}"))?;
    let frame_details = sonare_codec::aac_selected_scale_factor_frame_details_with_bitrate(
        expected_pcm,
        default_bitrate,
    )
    .map_err(|err| format!("{label} production AAC frame details failed: {err}"))?;
    let selector_max_frame_len = frame_details
        .iter()
        .map(|selection| selection.frame_len)
        .max()
        .unwrap_or(0);
    if selector_max_frame_len != max_frame_len {
        return Err(format!(
            "{label} production AAC selector detail mismatch: selector_max_frame_len={selector_max_frame_len}, encoded_max_frame_len={max_frame_len}"
        ));
    }

    validate_adts_frame_budget(label, max_frame_len, max_budget, default_bitrate)?;

    eprintln!(
        "{label} production ADTS frame budget: max_frame_len={max_frame_len}, default_budget={max_budget}, default_bitrate_bps={default_bitrate}, {}",
        aac_step_selection_summary(&frame_details)
    );
    Ok(())
}

fn validate_adts_frame_budget(
    label: &str,
    max_frame_len: usize,
    max_budget: usize,
    bitrate_bps: u32,
) -> Result<(), String> {
    if max_frame_len > max_budget {
        return Err(format!(
            "{label} ADTS frame budget failed: max_frame_len={max_frame_len} exceeds budget {max_budget} for {bitrate_bps}bps"
        ));
    }
    Ok(())
}

fn max_adts_frame_len(stream: &[u8]) -> Result<usize, String> {
    let mut offset = 0usize;
    let mut max_frame_len = 0usize;
    let mut frame_count = 0usize;
    while offset + 7 <= stream.len() {
        if stream[offset] != 0xff || stream[offset + 1] & 0xf0 != 0xf0 {
            return Err(format!("missing ADTS syncword at byte offset {offset}"));
        }
        let frame_len = (((stream[offset + 3] & 0x03) as usize) << 11)
            | ((stream[offset + 4] as usize) << 3)
            | ((stream[offset + 5] as usize) >> 5);
        if frame_len < 7 {
            return Err(format!(
                "invalid ADTS frame length {frame_len} at byte offset {offset}"
            ));
        }
        let next = offset
            .checked_add(frame_len)
            .ok_or_else(|| "ADTS frame length overflow".to_owned())?;
        if next > stream.len() {
            return Err(format!(
                "ADTS frame at byte offset {offset} extends past stream length {}",
                stream.len()
            ));
        }
        max_frame_len = max_frame_len.max(frame_len);
        frame_count += 1;
        offset = next;
    }

    if frame_count == 0 {
        return Err("ADTS stream has no complete frames".to_owned());
    }
    if offset != stream.len() {
        return Err(format!(
            "ADTS stream has {} trailing byte(s) after the last complete frame",
            stream.len() - offset
        ));
    }

    Ok(max_frame_len)
}

fn run_publish_rust_packages() -> Result<(), String> {
    for package in RUST_PUBLISH_PACKAGES {
        let package_list =
            run_command_output("cargo", &["package", "--list", "-p", package.package], ".")?;
        verify_rust_package_file_list(package.package, &package_list)?;
        if package.package_before_first_publish {
            run_command(
                "cargo",
                &["package", "--no-verify", "-p", package.package],
                ".",
            )?;
        } else {
            eprintln!(
                "skipping cargo package archive for {} until its internal dependencies are published",
                package.package
            );
        }
    }
    Ok(())
}

fn verify_rust_package_file_list(package: &str, package_list: &str) -> Result<(), String> {
    for required in ["Cargo.toml", "LICENSE", "NOTICE", "README.md", "src/lib.rs"] {
        if !package_list.lines().any(|line| line == required) {
            return Err(format!(
                "cargo package --list for {package} is missing {required}"
            ));
        }
    }
    for forbidden_prefix in ["backup/", "target/", "bindings/wasm/pkg/"] {
        if package_list
            .lines()
            .any(|line| line.starts_with(forbidden_prefix))
        {
            return Err(format!(
                "cargo package --list for {package} includes forbidden path prefix {forbidden_prefix}"
            ));
        }
    }
    Ok(())
}

fn run_package_metadata_check() -> Result<(), String> {
    eprintln!("checking package metadata consistency");

    for package in RUST_PUBLISH_PACKAGES {
        let manifest = fs::read_to_string(package.manifest)
            .map_err(|err| format!("failed to read {}: {err}", package.manifest))?;
        let manifest_name = toml_string_value(&manifest, "name")
            .ok_or_else(|| format!("missing package name in {}", package.manifest))?;
        let manifest_version = toml_string_value(&manifest, "version")
            .ok_or_else(|| format!("missing package version in {}", package.manifest))?;
        let readme = toml_string_value(&manifest, "readme")
            .ok_or_else(|| format!("missing package readme in {}", package.manifest))?;

        if manifest_name != package.package {
            return Err(format!(
                "{} package name {manifest_name} does not match publish list name {}",
                package.manifest, package.package
            ));
        }
        if manifest_version != RELEASE_VERSION {
            return Err(format!(
                "{} package version {manifest_version} does not match expected workspace release version {RELEASE_VERSION}",
                package.manifest
            ));
        }
        if readme != "README.md" {
            return Err(format!(
                "{} package readme {readme} does not match README.md",
                package.manifest
            ));
        }
        let readme_path = Path::new(package.manifest).with_file_name(readme);
        if !readme_path.is_file() {
            return Err(format!("{} is missing", readme_path.display()));
        }
        assert_contains(
            &manifest,
            "license.workspace = true",
            &format!("{} license metadata", package.manifest),
        )?;
        assert_contains(
            &manifest,
            "repository.workspace = true",
            &format!("{} repository metadata", package.manifest),
        )?;
        assert_contains(
            &manifest,
            "homepage.workspace = true",
            &format!("{} homepage metadata", package.manifest),
        )?;
        assert_contains(
            &manifest,
            "rust-version.workspace = true",
            &format!("{} rust-version metadata", package.manifest),
        )?;
        assert_contains(
            &manifest,
            "keywords.workspace = true",
            &format!("{} keywords metadata", package.manifest),
        )?;
        assert_contains(
            &manifest,
            "categories.workspace = true",
            &format!("{} categories metadata", package.manifest),
        )?;
        assert_contains(
            &manifest,
            "include = [\"Cargo.toml\", \"LICENSE\", \"NOTICE\", \"README.md\", \"src/**",
            &format!("{} package include list", package.manifest),
        )?;
        let license_path = Path::new(package.manifest).with_file_name("LICENSE");
        if !license_path.is_file() {
            return Err(format!("{} is missing", license_path.display()));
        }
        let notice_path = Path::new(package.manifest).with_file_name("NOTICE");
        if !notice_path.is_file() {
            return Err(format!("{} is missing", notice_path.display()));
        }
    }

    let workspace = fs::read_to_string("Cargo.toml")
        .map_err(|err| format!("failed to read Cargo.toml: {err}"))?;
    let readme = fs::read_to_string("README.md")
        .map_err(|err| format!("failed to read README.md: {err}"))?;
    let deny = fs::read_to_string("deny.toml")
        .map_err(|err| format!("failed to read deny.toml: {err}"))?;
    let rust = fs::read_to_string("crates/sonare-codec/Cargo.toml")
        .map_err(|err| format!("failed to read crates/sonare-codec/Cargo.toml: {err}"))?;
    let npm = fs::read_to_string("bindings/wasm/package.json")
        .map_err(|err| format!("failed to read bindings/wasm/package.json: {err}"))?;
    let npm_index = fs::read_to_string("bindings/wasm/index.js")
        .map_err(|err| format!("failed to read bindings/wasm/index.js: {err}"))?;
    let npm_types = fs::read_to_string("bindings/wasm/index.d.ts")
        .map_err(|err| format!("failed to read bindings/wasm/index.d.ts: {err}"))?;
    let pyproject = fs::read_to_string("bindings/python/pyproject.toml")
        .map_err(|err| format!("failed to read bindings/python/pyproject.toml: {err}"))?;
    let python_types = fs::read_to_string("bindings/python/sonare_codec.pyi")
        .map_err(|err| format!("failed to read bindings/python/sonare_codec.pyi: {err}"))?;
    let python_manifest = fs::read_to_string("bindings/python/Cargo.toml")
        .map_err(|err| format!("failed to read bindings/python/Cargo.toml: {err}"))?;
    let notice =
        fs::read_to_string("NOTICE").map_err(|err| format!("failed to read NOTICE: {err}"))?;

    let rust_name = toml_string_value(&rust, "name")
        .ok_or("missing package name in crates/sonare-codec/Cargo.toml")?;
    let rust_version = toml_string_value(&rust, "version")
        .ok_or("missing package version in crates/sonare-codec/Cargo.toml")?;
    let npm_name = json_string_value(&npm, "name")
        .ok_or("missing package name in bindings/wasm/package.json")?;
    let npm_version = json_string_value(&npm, "version")
        .ok_or("missing package version in bindings/wasm/package.json")?;
    let python_name = toml_string_value(&pyproject, "name")
        .ok_or("missing project name in bindings/python/pyproject.toml")?;
    let python_version = toml_string_value(&pyproject, "version")
        .ok_or("missing project version in bindings/python/pyproject.toml")?;
    let workspace_license =
        toml_string_value(&workspace, "license").ok_or("missing workspace package license")?;
    let workspace_repository = toml_string_value(&workspace, "repository")
        .ok_or("missing workspace package repository")?;
    let workspace_homepage =
        toml_string_value(&workspace, "homepage").ok_or("missing workspace package homepage")?;

    if rust_name != "sonare-codec" {
        return Err(format!("unexpected Rust package name {rust_name}"));
    }
    if npm_name != NPM_PACKAGE_NAME {
        return Err(format!("unexpected npm package name {npm_name}"));
    }
    if python_name != PYTHON_PACKAGE_NAME {
        return Err(format!("unexpected Python package name {python_name}"));
    }
    if npm_version != rust_version {
        return Err(format!(
            "npm package version {npm_version} does not match Rust package version {rust_version}"
        ));
    }
    if python_version != rust_version {
        return Err(format!(
            "Python package version {python_version} does not match Rust package version {rust_version}"
        ));
    }
    if workspace_license != PROJECT_LICENSE {
        return Err(format!(
            "workspace license {workspace_license} does not match expected {PROJECT_LICENSE}"
        ));
    }
    if workspace_repository != PROJECT_REPOSITORY {
        return Err(format!(
            "workspace repository {workspace_repository} does not match expected {PROJECT_REPOSITORY}"
        ));
    }
    if workspace_homepage != PROJECT_REPOSITORY {
        return Err(format!(
            "workspace homepage {workspace_homepage} does not match expected {PROJECT_REPOSITORY}"
        ));
    }
    assert_contains(
        &readme,
        "## Development Policy & Provenance",
        "README provenance policy",
    )?;
    assert_contains(
        &readme,
        "Decode integration uses Symphonia's public API through `sc-decode`",
        "README decode provenance",
    )?;
    assert_contains(
        &readme,
        "published specifications, not LAME/FAAC/fdk-aac source",
        "README clean-room policy",
    )?;
    assert_contains(
        &readme,
        "not copied from Symphonia test assets",
        "README test vector provenance",
    )?;
    assert_contains(
        &readme,
        "not grant patent licenses beyond the Apache-2.0 license text",
        "README patent policy",
    )?;
    assert_contains(
        &readme,
        "GPL/LGPL tools may be used locally as black-box oracles",
        "README oracle policy",
    )?;
    assert_contains(
        &readme,
        "GPL/LGPL/AGPL are\n  intentionally absent from the allow-list",
        "README dependency license policy",
    )?;
    assert_contains(&deny, "\"MPL-2.0\"", "cargo-deny MPL allowance")?;
    for forbidden_license in ["\"GPL", "\"LGPL", "\"AGPL"] {
        if deny.contains(forbidden_license) {
            return Err(format!(
                "deny.toml must not allow copyleft license pattern {forbidden_license}"
            ));
        }
    }
    assert_contains(&npm, "\"license\": \"Apache-2.0\"", "npm package license")?;
    assert_contains(
        &npm,
        "\"url\": \"git+https://github.com/libraz/sonare-codec.git\"",
        "npm package repository",
    )?;
    assert_contains(
        &npm,
        "\"homepage\": \"https://github.com/libraz/sonare-codec#readme\"",
        "npm package homepage",
    )?;
    assert_contains(
        &npm,
        "\"url\": \"https://github.com/libraz/sonare-codec/issues\"",
        "npm package issue tracker",
    )?;
    assert_contains(&npm, "\"main\": \"./index.js\"", "npm package main")?;
    assert_contains(&npm, "\"module\": \"./index.js\"", "npm package module")?;
    assert_contains(&npm, "\"types\": \"index.d.ts\"", "npm package types")?;
    assert_contains(&npm, "\"pkg\"", "npm package files")?;
    assert_contains(&npm, "\"index.js\"", "npm package files")?;
    assert_contains(&npm, "\"index.d.ts\"", "npm package files")?;
    assert_contains(&npm, "\"NOTICE\"", "npm package files")?;
    assert_contains(
        &npm_index,
        "./pkg/sonare_codec_wasm.js",
        "npm wrapper entrypoint",
    )?;
    for function in PUBLIC_BINDING_FUNCTIONS {
        assert_contains(&npm_types, function, "npm TypeScript definitions")?;
    }
    assert_contains(&npm_types, "StreamDecoder", "npm TypeScript definitions")?;
    assert_contains(
        &pyproject,
        "license = \"Apache-2.0\"",
        "Python package license",
    )?;
    assert_contains(
        &pyproject,
        "license-files = [\"LICENSE\", \"NOTICE\"]",
        "Python package license files",
    )?;
    assert_contains(
        &pyproject,
        "Homepage = \"https://github.com/libraz/sonare-codec\"",
        "Python package homepage",
    )?;
    assert_contains(
        &pyproject,
        "Repository = \"https://github.com/libraz/sonare-codec\"",
        "Python package repository",
    )?;
    assert_contains(
        &pyproject,
        "Issues = \"https://github.com/libraz/sonare-codec/issues\"",
        "Python package issue tracker",
    )?;
    assert_contains(
        &pyproject,
        "module-name = \"sonare_codec\"",
        "Python module name",
    )?;
    assert_contains(
        &pyproject,
        "features = [\"extension-module\"]",
        "Python build features",
    )?;
    assert_contains(&python_types, "EncodedFormat", "Python type definitions")?;
    assert_contains(&python_types, "PcmTuple", "Python type definitions")?;
    assert_contains(&python_types, "StreamDecoder", "Python type definitions")?;
    for function in PUBLIC_BINDING_FUNCTIONS {
        assert_contains(&python_types, function, "Python type definitions")?;
    }
    for function in PYTHON_ONLY_BINDING_FUNCTIONS {
        assert_contains(&python_types, function, "Python type definitions")?;
    }
    assert_contains(
        &python_manifest,
        "name = \"sonare_codec\"",
        "Python Rust cdylib name",
    )?;
    assert_contains(&notice, "Apache License, Version 2.0", "NOTICE license")?;
    assert_contains(&notice, "Symphonia", "NOTICE dependency attribution")?;
    assert_contains(&notice, "MPL-2.0", "NOTICE dependency license")?;

    Ok(())
}

fn run_git_head_check() -> Result<(), String> {
    eprintln!("running git rev-parse --verify HEAD");
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .map_err(|err| format!("failed to run git rev-parse --verify HEAD: {err}"))?;
    if output.status.success() {
        return Ok(());
    }

    Err(
        "package-preflight requires a valid git HEAD commit because cargo package reads committed metadata; create the initial commit before running publish preflight"
            .to_owned(),
    )
}

fn run_deny(args: &[&str]) -> Result<(), String> {
    if let Ok(path) = env::var("SONARE_CARGO_DENY") {
        return run_command(path, args, ".");
    }

    let mut cargo_args = Vec::with_capacity(args.len() + 1);
    cargo_args.push("deny");
    cargo_args.extend_from_slice(args);
    run_command("cargo", &cargo_args, ".")
}

fn run_optional_nextest() -> Result<(), String> {
    match env::var_os("SONARE_CARGO_NEXTEST") {
        Some(path) => run_command(path, &["nextest", "run", "--workspace"], "."),
        None if cargo_subcommand_available("nextest") => {
            run_command("cargo", &["nextest", "run", "--workspace"], ".")
        }
        None => skip_optional_qa_tool(
            "nextest",
            "cargo-nextest",
            "set SONARE_CARGO_NEXTEST or install cargo-nextest",
        ),
    }
}

fn run_optional_machete() -> Result<(), String> {
    match env::var_os("SONARE_CARGO_MACHETE") {
        Some(path) => run_command(path, &[] as &[&str], "."),
        None if cargo_subcommand_available("machete") => run_command("cargo", &["machete"], "."),
        None => skip_optional_qa_tool(
            "machete",
            "cargo-machete",
            "set SONARE_CARGO_MACHETE or install cargo-machete",
        ),
    }
}

fn run_optional_audit() -> Result<(), String> {
    match env::var_os("SONARE_CARGO_AUDIT") {
        Some(path) => run_command(path, &["audit"], "."),
        None if cargo_subcommand_available("audit") => run_command("cargo", &["audit"], "."),
        None => skip_optional_qa_tool(
            "audit",
            "cargo-audit",
            "set SONARE_CARGO_AUDIT or install cargo-audit",
        ),
    }
}

fn run_optional_semver_checks() -> Result<(), String> {
    if !git_head_available()? {
        return skip_optional_qa_tool(
            "semver-checks",
            "cargo-semver-checks",
            "create a git HEAD baseline before running semver checks",
        );
    }

    match env::var_os("SONARE_CARGO_SEMVER_CHECKS") {
        Some(path) => run_command(
            path,
            &[
                "semver-checks",
                "--workspace",
                "--all-features",
                "--baseline-rev",
                "HEAD",
            ],
            ".",
        ),
        None if cargo_subcommand_available("semver-checks") => run_command(
            "cargo",
            &[
                "semver-checks",
                "--workspace",
                "--all-features",
                "--baseline-rev",
                "HEAD",
            ],
            ".",
        ),
        None => skip_optional_qa_tool(
            "semver-checks",
            "cargo-semver-checks",
            "set SONARE_CARGO_SEMVER_CHECKS or install cargo-semver-checks",
        ),
    }
}

fn run_optional_miri() -> Result<(), String> {
    if !cargo_toolchain_subcommand_available("+nightly", "miri") {
        return skip_optional_qa_tool(
            "miri",
            "cargo-miri",
            "install nightly with the miri component",
        );
    }

    run_command(
        "cargo",
        &[
            "+nightly",
            "miri",
            "test",
            "-p",
            "sc-core",
            "-p",
            "sc-wav",
            "-p",
            "sonare-codec",
            "--features",
            "wav",
        ],
        ".",
    )
}

fn run_optional_coverage() -> Result<(), String> {
    match env::var_os("SONARE_CARGO_LLVM_COV") {
        Some(path) => run_command(
            path,
            &[
                "llvm-cov",
                "--workspace",
                "--lcov",
                "--output-path",
                "lcov.info",
            ],
            ".",
        ),
        None if cargo_subcommand_available("llvm-cov") => run_command(
            "cargo",
            &[
                "llvm-cov",
                "--workspace",
                "--lcov",
                "--output-path",
                "lcov.info",
            ],
            ".",
        ),
        None => skip_optional_qa_tool(
            "llvm-cov",
            "cargo-llvm-cov",
            "set SONARE_CARGO_LLVM_COV or install cargo-llvm-cov",
        ),
    }
}

fn skip_optional_qa_tool(
    tool: &'static str,
    label: &'static str,
    install_hint: &'static str,
) -> Result<(), String> {
    if required_qa_tool(tool) {
        return Err(format!(
            "{label} is required by {REQUIRED_QA_TOOLS_ENV} but is unavailable; {install_hint}"
        ));
    }

    eprintln!("skipping {label}: {install_hint}");
    Ok(())
}

fn required_qa_tool(tool: &str) -> bool {
    env::var_os(REQUIRED_QA_TOOLS_ENV)
        .and_then(|value| value.into_string().ok())
        .is_some_and(|value| required_qa_tool_in_list(&value, tool))
}

fn required_qa_tool_in_list(value: &str, tool: &str) -> bool {
    value
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
        .filter(|item| !item.is_empty())
        .any(|item| item == "all" || item == tool)
}

fn cargo_subcommand_available(subcommand: &str) -> bool {
    Command::new("cargo")
        .args([subcommand, "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn cargo_toolchain_subcommand_available(toolchain: &str, subcommand: &str) -> bool {
    Command::new("cargo")
        .args([toolchain, subcommand, "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

fn git_head_available() -> Result<bool, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .map_err(|err| format!("failed to run git rev-parse --verify HEAD: {err}"))?;
    Ok(output.status.success())
}

fn run_wasm_check() -> Result<(), String> {
    if !wasm_target_installed()? {
        eprintln!("skipping wasm check: wasm32-unknown-unknown target is not installed");
        return Ok(());
    }
    run_command(
        "cargo",
        &[
            "check",
            "-p",
            "sonare-codec-wasm",
            "--target",
            "wasm32-unknown-unknown",
        ],
        ".",
    )
}

fn wasm_target_installed() -> Result<bool, String> {
    let output = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .map_err(|err| format!("failed to inspect rust sysroot: {err}"))?;
    if !output.status.success() {
        return Err("failed to inspect rust sysroot".to_owned());
    }
    let sysroot = String::from_utf8(output.stdout)
        .map_err(|err| format!("rust sysroot output is not UTF-8: {err}"))?;
    Ok(Path::new(sysroot.trim())
        .join("lib/rustlib/wasm32-unknown-unknown/lib")
        .exists())
}

fn run_npm_pack_dry_run() -> Result<(), String> {
    let cache = env::var_os("npm_config_cache")
        .unwrap_or_else(|| OsString::from("/private/tmp/sonare-codec-npm-cache"));
    let mut command = Command::new("npm");
    command
        .args(["pack", "--dry-run", "--ignore-scripts"])
        .current_dir("bindings/wasm")
        .env("npm_config_cache", cache);
    run_prepared_command(&mut command, "npm pack --dry-run --ignore-scripts")
}

fn run_npm_pack_output_check() -> Result<(), String> {
    let cache = env::var_os("npm_config_cache")
        .unwrap_or_else(|| OsString::from("/private/tmp/sonare-codec-npm-cache"));
    let script = r#"
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const output = execFileSync("npm", ["pack", "--ignore-scripts", "--json"], {
  cwd: "bindings/wasm",
  encoding: "utf8",
  env: { ...process.env, npm_config_cache: process.env.npm_config_cache },
});
const packs = JSON.parse(output);
if (!Array.isArray(packs) || packs.length !== 1) {
  throw new Error("npm pack did not return one package entry");
}
const pack = packs[0];
const expected = {
  id: "@libraz/sonare-codec@0.1.0",
  filename: "libraz-sonare-codec-0.1.0.tgz",
  name: "@libraz/sonare-codec",
  version: "0.1.0",
};
for (const [key, value] of Object.entries(expected)) {
  if (pack[key] !== value) {
    throw new Error(`npm pack ${key} ${pack[key]} does not match ${value}`);
  }
}
const files = new Set(pack.files.map((file) => file.path));
for (const path of [
  "LICENSE",
  "NOTICE",
  "README.md",
  "index.js",
  "index.d.ts",
  "package.json",
  "pkg/sonare_codec_wasm.js",
  "pkg/sonare_codec_wasm.d.ts",
  "pkg/sonare_codec_wasm_bg.wasm",
]) {
  if (!files.has(path)) {
    throw new Error(`npm package is missing ${path}`);
  }
}
const packagePath = `bindings/wasm/${pack.filename}`;
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "sonare-codec-npm-pack-"));
try {
  execFileSync("tar", ["-xzf", packagePath, "-C", tmp]);
  const generatedEntry = fs.readFileSync(
    path.join(tmp, "package/pkg/sonare_codec_wasm.js"),
    "utf8",
  );
  const generatedGlue = fs.readFileSync(
    path.join(tmp, "package/pkg/sonare_codec_wasm_bg.js"),
    "utf8",
  );
  const expectedExports = [
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
    "encode_mp3",
    "encode_mp3_with_bitrate",
    "encode_mp3_cbr_with_bitrate",
    "encode_mp3_perceptual_active_cbr_with_bitrate",
    "encode_mp3_perceptual_reservoir_with_bitrate",
    "encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate",
    "encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate",
    "mp3_reservoir_frame_details_with_bitrate",
    "mp3_perceptual_reservoir_frame_details_with_bitrate",
    "mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate",
    "mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate",
    "encode_aac",
    "encode_aac_with_bitrate",
    "encode_aac_with_selected_scale_factors_and_bitrate",
    "encode_aac_with_standard_spectral_offsets_and_bitrate",
    "encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate",
    "encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate",
    "encode_m4a",
    "encode_m4a_with_bitrate",
    "encode_m4a_with_selected_scale_factors_and_bitrate",
    "encode_m4a_with_standard_spectral_offsets_and_bitrate",
    "encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate",
    "encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate",
    "demux_m4a_as_aac_adts",
    "aac_lc_adts_max_frame_len_for_bitrate",
    "aac_lc_default_production_bitrate_bps",
    "aac_lc_pcm_step_candidates",
    "aac_standard_id_pcm_step_candidates",
    "aac_standard_id_selected_scale_factor_global_gain",
    "aac_standard_id_selected_scale_factor_magnitude_bias",
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
    "aac_selected_scale_factor_frame_details_with_bitrate",
    "mp3_layer3_main_data_capacity_bytes",
    "mp3_layer3_main_data_capacity_bits",
    "mp3_pcm_step_candidates",
    "mp3_first_frame_perceptual_candidate_profile_with_bitrate",
    "mp3_perceptual_bit_allocation_with_bitrate",
    "mp3_standard_big_value_table_selects",
    "mp3_missing_standard_big_value_table_selects",
    "mp3_standard_count1_table_selects",
  ];
  for (const [label, source] of [
    ["generated wasm entrypoint", generatedEntry],
    ["generated wasm glue", generatedGlue],
  ]) {
    for (const exportName of expectedExports) {
      if (!source.includes(exportName)) {
        throw new Error(`${label} is missing ${exportName}`);
      }
    }
  }
  const smokeScript = `
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const packageRoot = process.argv[1];
const glue = await import(pathToFileURL(path.join(packageRoot, "pkg/sonare_codec_wasm_bg.js")).href);
const bytes = fs.readFileSync(path.join(packageRoot, "pkg/sonare_codec_wasm_bg.wasm"));
const { instance } = await WebAssembly.instantiate(bytes, { "./sonare_codec_wasm_bg.js": glue });
glue.__wbg_set_wasm(instance.exports);
instance.exports.__wbindgen_start();

function maxAdtsFrameLen(stream) {
  let maxLen = 0;
  let offset = 0;
  while (offset + 7 <= stream.length) {
    const frameLen = ((stream[offset + 3] & 0x03) << 11) | (stream[offset + 4] << 3) | (stream[offset + 5] >> 5);
    maxLen = Math.max(maxLen, frameLen);
    offset += frameLen;
  }
  if (offset !== stream.length) {
    throw new Error("npm selected-scale-factor AAC bitrate helper returned malformed ADTS");
  }
  return maxLen;
}

function mp3FrameInfo(stream) {
  if (stream.length < 4 || stream[0] !== 0xff || (stream[1] & 0xe0) !== 0xe0) {
    throw new Error("npm MP3 helper returned malformed frame sync");
  }
  const versionBits = (stream[1] >> 3) & 0x03;
  const layerBits = (stream[1] >> 1) & 0x03;
  if (versionBits !== 0x03 || layerBits !== 0x01) {
    throw new Error("npm MP3 helper did not return MPEG-1 Layer III");
  }
  const bitrateKbps = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320][stream[2] >> 4];
  const sampleRate = [44100, 48000, 32000][(stream[2] >> 2) & 0x03];
  const padding = (stream[2] & 0x02) !== 0 ? 1 : 0;
  const channels = ((stream[3] >> 6) & 0x03) === 0x03 ? 1 : 2;
  const frameLen = Math.floor(144 * bitrateKbps * 1000 / sampleRate) + padding;
  return { bitrateKbps, sampleRate, channels, frameLen };
}

function mp3MainDataBegins(stream) {
  const begins = [];
  let offset = 0;
  while (offset < stream.length) {
    const info = mp3FrameInfo(stream.subarray(offset));
    begins.push((stream[offset + 4] << 1) | (stream[offset + 5] >> 7));
    offset += info.frameLen;
  }
  if (offset !== stream.length) {
    throw new Error("npm MP3 helper returned non-tiling frames");
  }
  return begins;
}

function hasApprox(values, expected) {
  return values.some((value) => Math.abs(value - expected) < 1e-6);
}

const selectedAac10k = glue.encode_aac_with_selected_scale_factors_and_bitrate(44100, 1, new Float32Array(2048), 10000);
if (glue.aac_lc_default_production_bitrate_bps(1) !== 128000 || glue.aac_lc_default_production_bitrate_bps(2) !== 256000) {
  throw new Error("npm AAC default production bitrate helper returned unexpected values");
}
const aacProductionSteps = Array.from(glue.aac_lc_pcm_step_candidates());
const aacStandardIdSteps = Array.from(glue.aac_standard_id_pcm_step_candidates());
if (!hasApprox(aacProductionSteps, 0.2) || hasApprox(aacProductionSteps, 0.15)) {
  throw new Error(` + "`npm AAC production step candidates returned ${JSON.stringify(aacProductionSteps)}`" + `);
}
if (!hasApprox(aacStandardIdSteps, 0.075) || !hasApprox(aacStandardIdSteps, 0.15) || aacStandardIdSteps.length <= aacProductionSteps.length) {
  throw new Error(` + "`npm AAC standard-id step candidates returned ${JSON.stringify(aacStandardIdSteps)}`" + `);
}
if (glue.aac_standard_id_selected_scale_factor_global_gain(1) !== 128 ||
    glue.aac_standard_id_selected_scale_factor_global_gain(2) !== 126 ||
    glue.aac_standard_id_selected_scale_factor_magnitude_bias() !== 16) {
  throw new Error("npm AAC standard-id selected-scale-factor recommended parameters returned unexpected values");
}
const aacRecommendedMonoParameters = Array.from(glue.aac_standard_id_selected_scale_factor_parameters(1));
const aacRecommendedStereoParameters = Array.from(glue.aac_standard_id_selected_scale_factor_parameters(2));
if (JSON.stringify(aacRecommendedMonoParameters) !== JSON.stringify([128, 16]) ||
    JSON.stringify(aacRecommendedStereoParameters) !== JSON.stringify([126, 16])) {
  throw new Error(` + "`npm AAC standard-id selected-scale-factor parameter helper returned ${JSON.stringify({aacRecommendedMonoParameters, aacRecommendedStereoParameters})}`" + `);
}
if (!(selectedAac10k instanceof Uint8Array) || selectedAac10k[0] !== 0xff || selectedAac10k[1] !== 0xf1 || maxAdtsFrameLen(selectedAac10k) > 30) {
  throw new Error("npm selected-scale-factor AAC bitrate helper returned unexpected bytes");
}
const selectedM4a10k = glue.encode_m4a_with_selected_scale_factors_and_bitrate(44100, 1, new Float32Array(2048), 10000);
if (!(selectedM4a10k instanceof Uint8Array) || selectedM4a10k[4] !== 0x66 || selectedM4a10k[5] !== 0x74 || selectedM4a10k[6] !== 0x79 || selectedM4a10k[7] !== 0x70) {
  throw new Error("npm selected-scale-factor M4A bitrate helper returned unexpected bytes");
}
const selectedM4aAdts = glue.demux_m4a_as_aac_adts(selectedM4a10k);
if (selectedM4aAdts.length !== selectedAac10k.length || !selectedM4aAdts.every((byte, index) => byte === selectedAac10k[index])) {
  throw new Error("npm selected-scale-factor M4A bitrate helper did not mux the expected ADTS");
}

const signedPairs5 = Array.from(glue.aac_signed_pairs5_table());
if (signedPairs5.length !== 324 || JSON.stringify(signedPairs5.slice(0, 4)) !== JSON.stringify([-4, -4, 0x1fff, 13]) || JSON.stringify(signedPairs5.slice(160, 164)) !== JSON.stringify([0, 0, 0, 1]) || JSON.stringify(signedPairs5.slice(-4)) !== JSON.stringify([4, 4, 0x1ffe, 13])) {
  throw new Error("npm AAC signed-pairs codebook 5 helper returned unexpected entries");
}
const signedPairs6 = Array.from(glue.aac_signed_pairs6_table());
if (signedPairs6.length !== 324 || JSON.stringify(signedPairs6.slice(0, 4)) !== JSON.stringify([-4, -4, 0x7fe, 11]) || JSON.stringify(signedPairs6.slice(160, 164)) !== JSON.stringify([0, 0, 0, 4]) || JSON.stringify(signedPairs6.slice(-4)) !== JSON.stringify([4, 4, 0x7fc, 11])) {
  throw new Error("npm AAC signed-pairs codebook 6 helper returned unexpected entries");
}
const signedQuads1 = Array.from(glue.aac_signed_quads1_table());
if (signedQuads1.length !== 486 || JSON.stringify(signedQuads1.slice(0, 6)) !== JSON.stringify([-1, -1, -1, -1, 0x7f8, 11]) || JSON.stringify(signedQuads1.slice(240, 246)) !== JSON.stringify([0, 0, 0, 0, 0, 1]) || JSON.stringify(signedQuads1.slice(-6)) !== JSON.stringify([1, 1, 1, 1, 0x7f4, 11])) {
  throw new Error("npm AAC signed-quad codebook 1 helper returned unexpected entries");
}
const signedQuads2 = Array.from(glue.aac_signed_quads2_table());
if (signedQuads2.length !== 486 || JSON.stringify(signedQuads2.slice(0, 6)) !== JSON.stringify([-1, -1, -1, -1, 0x1f3, 9]) || JSON.stringify(signedQuads2.slice(240, 246)) !== JSON.stringify([0, 0, 0, 0, 0, 3]) || JSON.stringify(signedQuads2.slice(-6)) !== JSON.stringify([1, 1, 1, 1, 0x1f6, 9])) {
  throw new Error("npm AAC signed-quad codebook 2 helper returned unexpected entries");
}
const quads3 = Array.from(glue.aac_unsigned_quads3_table());
if (quads3.length !== 486 || JSON.stringify(quads3.slice(0, 6)) !== JSON.stringify([0, 0, 0, 0, 0, 1]) || JSON.stringify(quads3.slice(240, 246)) !== JSON.stringify([1, 1, 1, 1, 0x74, 7]) || JSON.stringify(quads3.slice(-6)) !== JSON.stringify([2, 2, 2, 2, 0x7ffa, 15])) {
  throw new Error("npm AAC unsigned-quad codebook 3 helper returned unexpected entries");
}
const quads4 = Array.from(glue.aac_unsigned_quads4_table());
if (quads4.length !== 486 || JSON.stringify(quads4.slice(0, 6)) !== JSON.stringify([0, 0, 0, 0, 0x7, 4]) || JSON.stringify(quads4.slice(240, 246)) !== JSON.stringify([1, 1, 1, 1, 0, 4]) || JSON.stringify(quads4.slice(-6)) !== JSON.stringify([2, 2, 2, 2, 0x7fc, 11])) {
  throw new Error("npm AAC unsigned-quad codebook 4 helper returned unexpected entries");
}

const sections = Array.from(glue.aac_codebook6_unit_section_plan(new Int32Array([1, -1, 0, 0]), 2));
const expected = [0, 2, 6, 2, 4, 0];
if (JSON.stringify(sections) !== JSON.stringify(expected)) {
  throw new Error(` + "`npm AAC codebook 6 section planner returned ${JSON.stringify(sections)}`" + `);
}
const quadSections = Array.from(glue.aac_quad_unit_section_plan(new Int32Array([1, -1, 0, 1, 0, 1, -1, 0, 0, 0, 0, 0]), 4));
const expectedQuadSections = [0, 8, 3, 8, 12, 0];
if (JSON.stringify(quadSections) !== JSON.stringify(expectedQuadSections)) {
  throw new Error(` + "`npm AAC quad section planner returned ${JSON.stringify(quadSections)}`" + `);
}
const mixedSections = Array.from(glue.aac_mixed_unit_section_plan(new Int32Array([1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0]), 4));
const expectedMixedSections = [0, 4, 3, 4, 8, 6, 8, 12, 0];
if (JSON.stringify(mixedSections) !== JSON.stringify(expectedMixedSections)) {
  throw new Error(` + "`npm AAC mixed section planner returned ${JSON.stringify(mixedSections)}`" + `);
}
const mixedBitLengths = Array.from(glue.aac_mixed_unit_payload_bit_lengths(new Int32Array([1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0]), 4));
const expectedMixedBitLengths = [27, 11, 38, 29, 11, 40];
if (JSON.stringify(mixedBitLengths) !== JSON.stringify(expectedMixedBitLengths)) {
  throw new Error(` + "`npm AAC mixed payload bit lengths returned ${JSON.stringify(mixedBitLengths)}`" + `);
}
const standardSections = Array.from(glue.aac_standard_unit_section_plan(new Int32Array([1, -1, 17, 0]), 2));
const expectedStandardSections = [0, 2, 6, 2, 4, 11];
if (JSON.stringify(standardSections) !== JSON.stringify(expectedStandardSections)) {
  throw new Error(` + "`npm AAC standard section planner returned ${JSON.stringify(standardSections)}`" + `);
}
const standardSignedPairs5Section = Array.from(glue.aac_standard_unit_section_plan(new Int32Array([0, 1]), 2));
if (JSON.stringify(standardSignedPairs5Section) !== JSON.stringify([0, 2, 5])) {
  throw new Error(` + "`npm AAC standard signed-pairs codebook 5 planner returned ${JSON.stringify(standardSignedPairs5Section)}`" + `);
}
const standardMixedSections = Array.from(glue.aac_standard_unit_section_plan(new Int32Array([1, -1, 0, 1, 17, 0, 0, 0]), 4));
const expectedStandardMixedSections = [0, 4, 4, 4, 8, 11];
if (JSON.stringify(standardMixedSections) !== JSON.stringify(expectedStandardMixedSections)) {
  throw new Error(` + "`npm AAC standard mixed section planner returned ${JSON.stringify(standardMixedSections)}`" + `);
}
const standardMixedOffsetsSections = Array.from(glue.aac_standard_offsets_section_plan(new Int32Array([1, -1, 0, 1, 17, 0, 0, 0]), new Uint32Array([0, 4, 8])));
if (JSON.stringify(standardMixedOffsetsSections) !== JSON.stringify(expectedStandardMixedSections)) {
  throw new Error(` + "`npm AAC standard mixed offsets section planner returned ${JSON.stringify(standardMixedOffsetsSections)}`" + `);
}
const standardEscapeBitLengths = Array.from(glue.aac_standard_escape_payload_bit_lengths());
const expectedStandardEscapeBitLengths = [9, 15, 24];
if (JSON.stringify(standardEscapeBitLengths) !== JSON.stringify(expectedStandardEscapeBitLengths)) {
  throw new Error(` + "`npm AAC standard escape payload bit lengths returned ${JSON.stringify(standardEscapeBitLengths)}`" + `);
}
const standardMixedBitLengths = Array.from(glue.aac_standard_mixed_payload_bit_lengths(new Int32Array([1, -1, 0, 1, 17, 0, 0, 0]), 4));
const expectedStandardMixedBitLengths = [18, 26, 44, 20, 26, 46];
if (JSON.stringify(standardMixedBitLengths) !== JSON.stringify(expectedStandardMixedBitLengths)) {
  throw new Error(` + "`npm AAC standard mixed payload bit lengths returned ${JSON.stringify(standardMixedBitLengths)}`" + `);
}
const standardMixedOffsetsBitLengths = Array.from(glue.aac_standard_mixed_offsets_payload_bit_lengths(new Int32Array([1, -1, 0, 1, 17, 0, 0, 0]), new Uint32Array([0, 4, 8])));
if (JSON.stringify(standardMixedOffsetsBitLengths) !== JSON.stringify(expectedStandardMixedBitLengths)) {
  throw new Error(` + "`npm AAC standard mixed offsets payload bit lengths returned ${JSON.stringify(standardMixedOffsetsBitLengths)}`" + `);
}
const standardMonoAdts = glue.encode_aac_standard_mono_offsets_with_step(44100, new Float32Array(2048), 20, 128);
if (!(standardMonoAdts instanceof Uint8Array) || standardMonoAdts[0] !== 0xff || standardMonoAdts[1] !== 0xf1 || maxAdtsFrameLen(standardMonoAdts) > 16) {
  throw new Error("npm AAC standard mono offsets stream helper returned unexpected ADTS");
}
const standardMonoBitrateAdts = glue.encode_aac_standard_mono_offsets_with_bitrate(44100, new Float32Array(2048), 128000, 128);
if (!(standardMonoBitrateAdts instanceof Uint8Array) || standardMonoBitrateAdts[0] !== 0xff || standardMonoBitrateAdts[1] !== 0xf1 || maxAdtsFrameLen(standardMonoBitrateAdts) > 372) {
  throw new Error("npm AAC standard mono offsets bitrate stream helper returned unexpected ADTS");
}
const standardGenericAdts = glue.encode_aac_with_standard_spectral_offsets_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128);
if (!(standardGenericAdts instanceof Uint8Array) || standardGenericAdts[0] !== 0xff || standardGenericAdts[1] !== 0xf1 || maxAdtsFrameLen(standardGenericAdts) > 372) {
  throw new Error("npm AAC standard spectral-offset bitrate helper returned unexpected ADTS");
}
const standardGenericM4a = glue.encode_m4a_with_standard_spectral_offsets_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128);
if (!(standardGenericM4a instanceof Uint8Array) || String.fromCharCode(...standardGenericM4a.slice(4, 8)) !== "ftyp") {
  throw new Error("npm M4A standard spectral-offset bitrate helper returned unexpected container");
}
const standardSelectedGenericAdts = glue.encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16);
if (!(standardSelectedGenericAdts instanceof Uint8Array) || standardSelectedGenericAdts[0] !== 0xff || standardSelectedGenericAdts[1] !== 0xf1 || maxAdtsFrameLen(standardSelectedGenericAdts) > 372) {
  throw new Error("npm AAC standard selected spectral-offset bitrate helper returned unexpected ADTS");
}
const recommendedStandardSelectedGenericAdts = glue.encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, new Float32Array(2048), 128000);
if (Buffer.compare(Buffer.from(recommendedStandardSelectedGenericAdts), Buffer.from(standardSelectedGenericAdts)) !== 0) {
  throw new Error("npm AAC recommended standard selected helper did not match explicit mono parameters");
}
const standardSelectedGenericM4a = glue.encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16);
if (!(standardSelectedGenericM4a instanceof Uint8Array) || String.fromCharCode(...standardSelectedGenericM4a.slice(4, 8)) !== "ftyp") {
  throw new Error("npm M4A standard selected spectral-offset bitrate helper returned unexpected container");
}
const recommendedStandardSelectedGenericM4a = glue.encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, new Float32Array(2048), 128000);
if (Buffer.compare(Buffer.from(recommendedStandardSelectedGenericM4a), Buffer.from(standardSelectedGenericM4a)) !== 0) {
  throw new Error("npm M4A recommended standard selected helper did not match explicit mono parameters");
}
const standardSelectedDetails = Array.from(glue.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16));
if (standardSelectedDetails.length !== 8 || standardSelectedDetails[0] !== 0 || standardSelectedDetails[4] !== 1 || standardSelectedDetails[2] > 372 || standardSelectedDetails[6] > 372) {
  throw new Error(` + "`npm AAC standard selected bitrate details returned ${JSON.stringify(standardSelectedDetails)}`" + `);
}
const recommendedStandardSelectedDetails = Array.from(glue.aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (JSON.stringify(recommendedStandardSelectedDetails) !== JSON.stringify(standardSelectedDetails)) {
  throw new Error(` + "`npm AAC recommended standard selected bitrate details returned ${JSON.stringify(recommendedStandardSelectedDetails)}`" + `);
}
const productionSelectedDetails = Array.from(glue.aac_selected_scale_factor_frame_details_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (productionSelectedDetails.length !== 8 || productionSelectedDetails[0] !== 0 || productionSelectedDetails[4] !== 1 || productionSelectedDetails[2] > 372 || productionSelectedDetails[6] > 372) {
  throw new Error(` + "`npm AAC production selected bitrate details returned ${JSON.stringify(productionSelectedDetails)}`" + `);
}
const standardMonoBitrateDetails = Array.from(glue.aac_standard_mono_offsets_bitrate_frame_details(44100, new Float32Array(2048), 128000, 128));
if (standardMonoBitrateDetails.length !== 8 || standardMonoBitrateDetails[0] !== 0 || standardMonoBitrateDetails[4] !== 1 || standardMonoBitrateDetails[2] > 372 || standardMonoBitrateDetails[6] > 372) {
  throw new Error(` + "`npm AAC standard mono offsets bitrate details returned ${JSON.stringify(standardMonoBitrateDetails)}`" + `);
}
const standardStereoAdts = glue.encode_aac_standard_stereo_offsets_with_step(44100, new Float32Array(4096), 20, 128);
if (!(standardStereoAdts instanceof Uint8Array) || standardStereoAdts[0] !== 0xff || standardStereoAdts[1] !== 0xf1 || maxAdtsFrameLen(standardStereoAdts) > 28) {
  throw new Error("npm AAC standard stereo offsets stream helper returned unexpected ADTS");
}
const standardStereoBitrateAdts = glue.encode_aac_standard_stereo_offsets_with_bitrate(44100, new Float32Array(4096), 256000, 128);
if (!(standardStereoBitrateAdts instanceof Uint8Array) || standardStereoBitrateAdts[0] !== 0xff || standardStereoBitrateAdts[1] !== 0xf1 || maxAdtsFrameLen(standardStereoBitrateAdts) > 744) {
  throw new Error("npm AAC standard stereo offsets bitrate stream helper returned unexpected ADTS");
}
const standardStereoBitrateDetails = Array.from(glue.aac_standard_stereo_offsets_bitrate_frame_details(44100, new Float32Array(4096), 256000, 128));
if (standardStereoBitrateDetails.length !== 8 || standardStereoBitrateDetails[0] !== 0 || standardStereoBitrateDetails[4] !== 1 || standardStereoBitrateDetails[2] > 744 || standardStereoBitrateDetails[6] > 744) {
  throw new Error(` + "`npm AAC standard stereo offsets bitrate details returned ${JSON.stringify(standardStereoBitrateDetails)}`" + `);
}
const standardSelectedStereoDetails = Array.from(glue.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(44100, 2, new Float32Array(4096), 256000, 128, 16));
if (standardSelectedStereoDetails.length !== 8 || standardSelectedStereoDetails[0] !== 0 || standardSelectedStereoDetails[4] !== 1 || standardSelectedStereoDetails[2] > 744 || standardSelectedStereoDetails[6] > 744) {
  throw new Error(` + "`npm AAC standard selected stereo bitrate details returned ${JSON.stringify(standardSelectedStereoDetails)}`" + `);
}
const recommendedStandardSelectedStereoDetails = Array.from(glue.aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(44100, 2, new Float32Array(4096), 256000));
const explicitRecommendedStandardSelectedStereoDetails = Array.from(glue.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(44100, 2, new Float32Array(4096), 256000, 126, 16));
if (JSON.stringify(recommendedStandardSelectedStereoDetails) !== JSON.stringify(explicitRecommendedStandardSelectedStereoDetails)) {
  throw new Error(` + "`npm AAC recommended standard selected stereo details returned ${JSON.stringify(recommendedStandardSelectedStereoDetails)}`" + `);
}
const productionSelectedStereoDetails = Array.from(glue.aac_selected_scale_factor_frame_details_with_bitrate(44100, 2, new Float32Array(4096), 256000));
if (productionSelectedStereoDetails.length !== 8 || productionSelectedStereoDetails[0] !== 0 || productionSelectedStereoDetails[4] !== 1 || productionSelectedStereoDetails[2] > 744 || productionSelectedStereoDetails[6] > 744) {
  throw new Error(` + "`npm AAC production selected stereo bitrate details returned ${JSON.stringify(productionSelectedStereoDetails)}`" + `);
}

if (glue.mp3_layer3_main_data_capacity_bytes(44100, 1, 96, false, false) !== 292 ||
    glue.mp3_layer3_main_data_capacity_bits(44100, 1, 96, false, false) !== 2336) {
  throw new Error("npm MP3 96kbps capacity helper returned an unexpected value");
}
const mp3Steps = Array.from(glue.mp3_pcm_step_candidates());
if (!hasApprox(mp3Steps, 0.2) || hasApprox(mp3Steps, 0.15)) {
  throw new Error(` + "`npm MP3 step candidates returned ${JSON.stringify(mp3Steps)}`" + `);
}
const mp3StandardTables = Array.from(glue.mp3_standard_big_value_table_selects());
if (JSON.stringify(mp3StandardTables) !== JSON.stringify([1,2,3,5,6,7,8,9,10,11,12,13,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31])) {
  throw new Error(` + "`npm MP3 standard table selector helper returned ${JSON.stringify(mp3StandardTables)}`" + `);
}
if (glue.mp3_missing_standard_big_value_table_selects().length !== 0) {
  throw new Error("npm MP3 missing standard table selector helper returned non-empty values");
}
if (JSON.stringify(Array.from(glue.mp3_standard_count1_table_selects())) !== JSON.stringify([0,1])) {
  throw new Error("npm MP3 count1 selector helper returned unexpected values");
}
const mp3_96k = glue.encode_mp3_with_bitrate(44100, 1, new Float32Array(1152), 96, false, false);
const mp3Info = mp3FrameInfo(mp3_96k);
if (!(mp3_96k instanceof Uint8Array) || mp3Info.bitrateKbps !== 96 || mp3Info.sampleRate !== 44100 || mp3Info.channels !== 1 || mp3Info.frameLen !== mp3_96k.length) {
  throw new Error("npm MP3 bitrate encode helper returned an unexpected frame budget");
}
const mp3Cbr128k = glue.encode_mp3_cbr_with_bitrate(44100, 1, new Float32Array(1152 * 3), 128, false);
const mp3CbrFirst = mp3FrameInfo(mp3Cbr128k);
const mp3CbrSecond = mp3FrameInfo(mp3Cbr128k.subarray(mp3CbrFirst.frameLen));
const mp3CbrThird = mp3FrameInfo(mp3Cbr128k.subarray(mp3CbrFirst.frameLen + mp3CbrSecond.frameLen));
if (!(mp3Cbr128k instanceof Uint8Array) ||
    mp3CbrFirst.frameLen !== 417 ||
    mp3CbrSecond.frameLen !== 418 ||
    mp3CbrThird.frameLen !== 418 ||
    mp3Cbr128k.length !== 1253) {
  throw new Error("npm MP3 CBR bitrate helper returned an unexpected padding schedule");
}
const perceptualSamples = Float32Array.from({ length: 1152 * 3 }, (_, index) => Math.sin(index * 0.013) * 0.25);
const mp3CandidateProfile = Array.from(glue.mp3_first_frame_perceptual_candidate_profile_with_bitrate(44100, 1, perceptualSamples, 128, false));
if (mp3CandidateProfile.length < 6 ||
    mp3CandidateProfile.length % 6 !== 0 ||
    !hasApprox([mp3CandidateProfile[0]], 0.0005) ||
    mp3CandidateProfile[4] !== 42 ||
    !mp3CandidateProfile.some((value, index) => index % 6 === 3 && value > 0)) {
  throw new Error(` + "`npm MP3 first-frame perceptual candidate profile returned ${JSON.stringify(mp3CandidateProfile)}`" + `);
}
const mp3BitAllocation = Array.from(glue.mp3_perceptual_bit_allocation_with_bitrate(44100, 1, perceptualSamples, 128, false, 0));
const mp3TargetBits = mp3BitAllocation.filter((_, index) => index % 5 === 4).reduce((sum, value) => sum + value, 0);
if (mp3BitAllocation.length !== 30 ||
    mp3BitAllocation[0] !== 0 ||
    mp3BitAllocation[1] !== 0 ||
    mp3BitAllocation[2] !== 0 ||
    !Number.isFinite(mp3BitAllocation[3]) ||
    mp3TargetBits !== 9520) {
  throw new Error(` + "`npm MP3 perceptual bit allocation returned ${JSON.stringify(mp3BitAllocation)}`" + `);
}
const mp3PerceptualCbr128k = glue.encode_mp3_perceptual_active_cbr_with_bitrate(44100, 1, perceptualSamples, 128, false);
const mp3PerceptualFirst = mp3FrameInfo(mp3PerceptualCbr128k);
const mp3PerceptualSecond = mp3FrameInfo(mp3PerceptualCbr128k.subarray(mp3PerceptualFirst.frameLen));
const mp3PerceptualThird = mp3FrameInfo(mp3PerceptualCbr128k.subarray(mp3PerceptualFirst.frameLen + mp3PerceptualSecond.frameLen));
if (!(mp3PerceptualCbr128k instanceof Uint8Array) ||
    mp3PerceptualFirst.frameLen !== 417 ||
    mp3PerceptualSecond.frameLen !== 418 ||
    mp3PerceptualThird.frameLen !== 418 ||
    mp3PerceptualCbr128k.length !== 1253) {
  throw new Error("npm MP3 perceptual active CBR helper returned an unexpected padding schedule");
}
const reservoirSamples = Float32Array.from({ length: 1152 * 8 }, (_, index) => {
  const frame = Math.floor(index / 1152);
  const t = index % 1152;
  return frame % 2 === 0
    ? 0.3 * (Math.sin(t * 0.043) + Math.sin(t * 0.131) + Math.sin(t * 0.277) + Math.sin(t * 0.611))
    : 0.02 * Math.sin(t * 0.05);
});
const reservoirStereoSamples = Float32Array.from({ length: 1152 * 8 * 2 }, (_, index) => {
  const frame = Math.floor(index / (1152 * 2));
  const t = Math.floor((index / 2) % 1152);
  const right = index % 2 === 1;
  if (frame % 2 !== 0) {
    return (right ? 0.018 : 0.02) * Math.sin(t * (right ? 0.047 : 0.041));
  }
  return right
    ? 0.24 * (Math.sin(t * 0.053) + Math.sin(t * 0.173) + Math.sin(t * 0.337))
    : 0.28 * (Math.sin(t * 0.037) + Math.sin(t * 0.149) + Math.sin(t * 0.419));
});
function checkMp3ProductionReservoir(label, channels, samples) {
  const detailWidth = 14;
  const granulesPerFrame = channels === 1 ? 2 : 4;
  const detailHelper = glue.mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate;
  const reservoirDetails = Array.from(detailHelper(44100, channels, samples, 128, false, 0));
  if (reservoirDetails.length !== 8 * detailWidth || reservoirDetails[0] !== 0 || reservoirDetails[6] !== 0) {
    throw new Error(` + "`npm ${label} MP3 reservoir detail helper returned malformed frame details`" + `);
  }
  let reservoirBorrowed = false;
  let entropyTargetBits = 0;
  let capacityBits = 0;
  let entropyTargetBudgetFrames = 0;
  for (let offset = 0; offset < reservoirDetails.length; offset += detailWidth) {
    const payloadBits = reservoirDetails[offset + 2];
    const frameLen = reservoirDetails[offset + 3];
    const padding = reservoirDetails[offset + 4];
    const capacityBytes = reservoirDetails[offset + 5];
    const mainDataBegin = reservoirDetails[offset + 6];
    const perceptualGranules = reservoirDetails[offset + 8];
    const calibratedGranules = reservoirDetails[offset + 9];
    const qualityGuardComparedGranules = reservoirDetails[offset + 10];
    const qualityGuardDistortionDelta = reservoirDetails[offset + 11];
    const frameEntropyTargetBits = reservoirDetails[offset + 12];
    const usedEntropyTargetBudget = reservoirDetails[offset + 13];
    entropyTargetBits += frameEntropyTargetBits;
    capacityBits += capacityBytes * 8;
    if (usedEntropyTargetBudget === 1) {
      entropyTargetBudgetFrames += 1;
    }
    if (mainDataBegin > 0) {
      reservoirBorrowed = true;
    }
    if (![417, 418].includes(frameLen) || (padding !== 0 && padding !== 1)) {
      throw new Error(` + "`npm ${label} MP3 reservoir detail helper reported an unexpected CBR frame slot`" + `);
    }
    if (payloadBits > (capacityBytes + mainDataBegin) * 8) {
      throw new Error(` + "`npm ${label} MP3 reservoir detail helper reported an over-budget frame`" + `);
    }
    if (perceptualGranules + calibratedGranules !== granulesPerFrame) {
      throw new Error(` + "`npm ${label} MP3 reservoir detail helper reported inconsistent granule telemetry`" + `);
    }
    if (perceptualGranules !== granulesPerFrame || calibratedGranules !== 0) {
      throw new Error(` + "`npm ${label} MP3 production reservoir did not report perceptual granules`" + `);
    }
    if (qualityGuardComparedGranules !== 0 || qualityGuardDistortionDelta !== 0) {
      throw new Error(` + "`npm ${label} MP3 production reservoir unexpectedly reported quality guard telemetry`" + `);
    }
  }
  if (!reservoirBorrowed) {
    throw new Error(` + "`npm ${label} MP3 reservoir detail helper never reported main_data_begin borrowing`" + `);
  }
  if (entropyTargetBits !== capacityBits || entropyTargetBudgetFrames === 0) {
    throw new Error(` + "`npm ${label} MP3 entropy-targeted production reservoir failed target checks`" + `);
  }
  const reservoirProduction = glue.encode_audio_production("mp3", 44100, channels, samples);
  const entropyTargetedProduction = glue.encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(44100, channels, samples, 128, false, 0);
  if (Buffer.compare(Buffer.from(reservoirProduction), Buffer.from(entropyTargetedProduction)) !== 0) {
    throw new Error(` + "`npm ${label} MP3 production did not use the entropy-targeted perceptual reservoir path`" + `);
  }
  const productionMainDataBegins = mp3MainDataBegins(reservoirProduction);
  if (productionMainDataBegins.length * detailWidth !== reservoirDetails.length) {
    throw new Error(` + "`npm ${label} MP3 production reservoir frame count did not match selector details`" + `);
  }
  for (let frame = 0; frame < productionMainDataBegins.length; frame += 1) {
    if (productionMainDataBegins[frame] !== reservoirDetails[frame * detailWidth + 6]) {
      throw new Error(` + "`npm ${label} MP3 production reservoir side-info did not match selector details`" + `);
    }
  }
}
checkMp3ProductionReservoir("mono", 1, reservoirSamples);
checkMp3ProductionReservoir("stereo", 2, reservoirStereoSamples);
const perceptualReservoirDetails = Array.from(glue.mp3_perceptual_reservoir_frame_details_with_bitrate(44100, 1, reservoirSamples, 128, false));
const reservoirDetailWidth = 12;
if (perceptualReservoirDetails.length !== 8 * reservoirDetailWidth || perceptualReservoirDetails[0] !== 0 || perceptualReservoirDetails[6] !== 0) {
  throw new Error("npm MP3 perceptual reservoir detail helper returned malformed frame details");
}
let perceptualReservoirBorrowed = false;
for (let offset = 0; offset < perceptualReservoirDetails.length; offset += reservoirDetailWidth) {
  const payloadBits = perceptualReservoirDetails[offset + 2];
  const frameLen = perceptualReservoirDetails[offset + 3];
  const padding = perceptualReservoirDetails[offset + 4];
  const capacityBytes = perceptualReservoirDetails[offset + 5];
  const mainDataBegin = perceptualReservoirDetails[offset + 6];
  const perceptualGranules = perceptualReservoirDetails[offset + 8];
  const calibratedGranules = perceptualReservoirDetails[offset + 9];
  const qualityGuardComparedGranules = perceptualReservoirDetails[offset + 10];
  const qualityGuardDistortionDelta = perceptualReservoirDetails[offset + 11];
  if (mainDataBegin > 0) {
    perceptualReservoirBorrowed = true;
  }
  if (![417, 418].includes(frameLen) || (padding !== 0 && padding !== 1)) {
    throw new Error("npm MP3 perceptual reservoir detail helper reported an unexpected CBR frame slot");
  }
  if (payloadBits > (capacityBytes + mainDataBegin) * 8) {
    throw new Error("npm MP3 perceptual reservoir detail helper reported an over-budget frame");
  }
  if (perceptualGranules !== 2 || calibratedGranules !== 0) {
    throw new Error("npm MP3 perceptual reservoir detail helper reported unexpected granule telemetry");
  }
  if (qualityGuardComparedGranules !== 0 || qualityGuardDistortionDelta !== 0) {
    throw new Error("npm MP3 perceptual reservoir detail helper unexpectedly reported quality guard telemetry");
  }
}
if (!perceptualReservoirBorrowed) {
  throw new Error("npm MP3 perceptual reservoir detail helper never reported main_data_begin borrowing");
}
const entropyTargetedReservoirDetails = Array.from(glue.mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(44100, 1, reservoirSamples, 128, false, 0));
const entropyTargetedReservoirDetailWidth = 14;
if (entropyTargetedReservoirDetails.length !== 8 * entropyTargetedReservoirDetailWidth ||
    entropyTargetedReservoirDetails[0] !== 0 ||
    entropyTargetedReservoirDetails[6] !== 0) {
  throw new Error(` + "`npm MP3 entropy-targeted perceptual reservoir details returned ${JSON.stringify(entropyTargetedReservoirDetails)}`" + `);
}
const entropyTargetedReservoirBits = entropyTargetedReservoirDetails
  .filter((_, index) => index % entropyTargetedReservoirDetailWidth === 12)
  .reduce((sum, value) => sum + value, 0);
const entropyTargetedReservoirCapacityBits = perceptualReservoirDetails
  .filter((_, index) => index % reservoirDetailWidth === 5)
  .reduce((sum, value) => sum + value * 8, 0);
if (entropyTargetedReservoirBits !== entropyTargetedReservoirCapacityBits ||
    !entropyTargetedReservoirDetails.some((value, index) => index % entropyTargetedReservoirDetailWidth === 13 && value === 1)) {
  throw new Error(` + "`npm MP3 entropy-targeted perceptual reservoir details failed target checks: ${JSON.stringify(entropyTargetedReservoirDetails)}`" + `);
}
const entropyTargetedReservoirMp3 = glue.encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(44100, 1, reservoirSamples, 128, false, 0);
const entropyTargetedReservoirMainDataBegins = mp3MainDataBegins(entropyTargetedReservoirMp3);
if (entropyTargetedReservoirMainDataBegins.length * entropyTargetedReservoirDetailWidth !== entropyTargetedReservoirDetails.length) {
  throw new Error("npm MP3 entropy-targeted perceptual reservoir frame count did not match selector details");
}
for (let frame = 0; frame < entropyTargetedReservoirMainDataBegins.length; frame += 1) {
  if (entropyTargetedReservoirMainDataBegins[frame] !== entropyTargetedReservoirDetails[frame * entropyTargetedReservoirDetailWidth + 6]) {
    throw new Error("npm MP3 entropy-targeted perceptual reservoir side-info did not match selector details");
  }
}
const perceptualReservoirMp3 = glue.encode_mp3_perceptual_reservoir_with_bitrate(44100, 1, reservoirSamples, 128, false);
const entropyTargetedReservoirProduction = glue.encode_audio_production("mp3", 44100, 1, reservoirSamples);
if (Buffer.compare(Buffer.from(entropyTargetedReservoirProduction), Buffer.from(entropyTargetedReservoirMp3)) !== 0) {
  throw new Error("npm MP3 mono production did not use the entropy-targeted perceptual reservoir path");
}
const perceptualReservoirMainDataBegins = mp3MainDataBegins(perceptualReservoirMp3);
if (perceptualReservoirMainDataBegins.length * reservoirDetailWidth !== perceptualReservoirDetails.length) {
  throw new Error("npm MP3 perceptual reservoir frame count did not match selector details");
}
for (let frame = 0; frame < perceptualReservoirMainDataBegins.length; frame += 1) {
  if (perceptualReservoirMainDataBegins[frame] !== perceptualReservoirDetails[frame * reservoirDetailWidth + 6]) {
    throw new Error("npm MP3 perceptual reservoir side-info did not match selector details");
  }
}
`;
  execFileSync(process.execPath, ["--input-type=module", "-e", smokeScript, path.join(tmp, "package")], {
    stdio: "inherit",
  });
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
  fs.unlinkSync(packagePath);
}
console.log(`checked ${packagePath}`);
"#;
    let mut command = Command::new("node");
    command
        .args(["-e", script])
        .env("npm_config_cache", cache)
        .current_dir(".");
    run_prepared_command(&mut command, "node -e <npm package output check>")
}

fn run_wasm_pack_build() -> Result<(), String> {
    let wasm_pack = env::var_os("SONARE_WASM_PACK").unwrap_or_else(|| OsString::from("wasm-pack"));
    let label = format!("{} build --target bundler", wasm_pack.to_string_lossy());
    let mut command = Command::new(wasm_pack);
    command
        .args(["build", "--target", "bundler"])
        .current_dir("bindings/wasm");
    run_prepared_command(&mut command, &label)?;
    match fs::remove_file("bindings/wasm/pkg/.gitignore") {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "failed to remove generated wasm-pack pkg/.gitignore before npm packing: {err}"
        )),
    }
}

fn run_wasm_pack_output_check() -> Result<(), String> {
    eprintln!("checking wasm-pack bundler output");
    let expected = [
        "bindings/wasm/pkg/sonare_codec_wasm.js",
        "bindings/wasm/pkg/sonare_codec_wasm.d.ts",
        "bindings/wasm/pkg/sonare_codec_wasm_bg.wasm",
    ];
    for path in expected {
        if !Path::new(path).is_file() {
            return Err(format!(
                "wasm-pack output is missing {path}; run `wasm-pack build --target bundler` from bindings/wasm before npm publish"
            ));
        }
    }
    let generated_types = fs::read_to_string("bindings/wasm/pkg/sonare_codec_wasm.d.ts")
        .map_err(|err| format!("failed to read generated wasm TypeScript definitions: {err}"))?;
    assert_contains(
        &generated_types,
        "class StreamDecoder",
        "generated wasm TypeScript definitions",
    )?;
    assert_contains(
        &generated_types,
        "decode_stream",
        "generated wasm TypeScript definitions",
    )?;
    for function in PUBLIC_BINDING_FUNCTIONS {
        assert_contains(
            &generated_types,
            function,
            "generated wasm TypeScript definitions",
        )?;
    }
    Ok(())
}

fn run_maturin_build() -> Result<(), String> {
    let python = env::var_os("SONARE_PYTHON").unwrap_or_else(|| OsString::from("python"));
    let label = format!(
        "{} -m maturin build --interpreter {}",
        python.to_string_lossy(),
        python.to_string_lossy()
    );
    let mut command = Command::new(&python);
    command
        .args(["-m", "maturin", "build", "--interpreter"])
        .arg(&python);
    command.current_dir("bindings/python");
    run_prepared_command(&mut command, &label)
}

fn run_python_wheel_output_check() -> Result<(), String> {
    let python = env::var_os("SONARE_PYTHON").unwrap_or_else(|| OsString::from("python"));
    let script = r#"
import glob
import math
import os
import subprocess
import sys
import tempfile
import zipfile

wheels = glob.glob("target/wheels/sonare_codec-0.1.0-*.whl")
if not wheels:
    sys.exit("missing Python wheel target/wheels/sonare_codec-0.1.0-*.whl")
wheel = max(wheels, key=os.path.getmtime)
with zipfile.ZipFile(wheel) as zf:
    names = set(zf.namelist())
    required = {
        "sonare_codec/__init__.pyi",
        "sonare_codec/py.typed",
        "sonare_codec-0.1.0.dist-info/METADATA",
        "sonare_codec-0.1.0.dist-info/licenses/LICENSE",
        "sonare_codec-0.1.0.dist-info/licenses/NOTICE",
    }
    missing = sorted(required - names)
    if missing:
        sys.exit("Python wheel is missing " + ", ".join(missing))
    metadata = zf.read("sonare_codec-0.1.0.dist-info/METADATA").decode("utf-8")
    for expected in [
        "Name: sonare-codec",
        "Version: 0.1.0",
        "License-Expression: Apache-2.0",
        "Project-URL: Repository, https://github.com/libraz/sonare-codec",
    ]:
        if expected not in metadata:
            sys.exit("Python wheel metadata is missing " + expected)
with tempfile.TemporaryDirectory(prefix="sonare-codec-wheel-") as target:
    subprocess.run(
        [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--quiet",
            "--no-deps",
            "--target",
            target,
            wheel,
        ],
        check=True,
    )
    sys.path.insert(0, target)
    import sonare_codec

    def max_adts_frame_len(stream):
        max_len = 0
        offset = 0
        while offset + 7 <= len(stream):
            frame_len = ((stream[offset + 3] & 0x03) << 11) | (stream[offset + 4] << 3) | (stream[offset + 5] >> 5)
            max_len = max(max_len, frame_len)
            offset += frame_len
        if offset != len(stream):
            sys.exit("Python wheel AAC bitrate helper returned malformed ADTS")
        return max_len

    def mp3_frame_info(stream):
        if len(stream) < 4 or stream[0] != 0xff or (stream[1] & 0xe0) != 0xe0:
            sys.exit("Python wheel MP3 helper returned malformed frame sync")
        version_bits = (stream[1] >> 3) & 0x03
        layer_bits = (stream[1] >> 1) & 0x03
        if version_bits != 0x03 or layer_bits != 0x01:
            sys.exit("Python wheel MP3 helper did not return MPEG-1 Layer III")
        bitrate_kbps = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320][stream[2] >> 4]
        sample_rate = [44100, 48000, 32000][(stream[2] >> 2) & 0x03]
        padding = 1 if stream[2] & 0x02 else 0
        channels = 1 if ((stream[3] >> 6) & 0x03) == 0x03 else 2
        frame_len = (144 * bitrate_kbps * 1000 // sample_rate) + padding
        return bitrate_kbps, sample_rate, channels, frame_len

    def mp3_main_data_begins(stream):
        begins = []
        offset = 0
        while offset < len(stream):
            _, _, _, frame_len = mp3_frame_info(stream[offset:])
            begins.append((stream[offset + 4] << 1) | (stream[offset + 5] >> 7))
            offset += frame_len
        if offset != len(stream):
            sys.exit("Python wheel MP3 helper returned non-tiling frames")
        return begins

    def has_approx(values, expected):
        return any(abs(value - expected) < 1e-6 for value in values)

    if sonare_codec.aac_lc_adts_max_frame_len_for_bitrate(44100, 10000) != 30:
        sys.exit("Python wheel AAC bitrate budget helper returned an unexpected frame length")
    if sonare_codec.aac_lc_default_production_bitrate_bps(1) != 128000 or sonare_codec.aac_lc_default_production_bitrate_bps(2) != 256000:
        sys.exit("Python wheel AAC default production bitrate helper returned unexpected values")
    production_steps = sonare_codec.aac_lc_pcm_step_candidates()
    standard_id_steps = sonare_codec.aac_standard_id_pcm_step_candidates()
    if not has_approx(production_steps, 0.2) or has_approx(production_steps, 0.15):
        sys.exit(f"Python wheel AAC production step candidates returned {production_steps}")
    if not has_approx(standard_id_steps, 0.075) or not has_approx(standard_id_steps, 0.15) or len(standard_id_steps) <= len(production_steps):
        sys.exit(f"Python wheel AAC standard-id step candidates returned {standard_id_steps}")
    if (
        sonare_codec.aac_standard_id_selected_scale_factor_global_gain(1) != 128
        or sonare_codec.aac_standard_id_selected_scale_factor_global_gain(2) != 126
        or sonare_codec.aac_standard_id_selected_scale_factor_magnitude_bias() != 16
    ):
        sys.exit("Python wheel AAC standard-id selected-scale-factor recommended parameters returned unexpected values")
    if (
        sonare_codec.aac_standard_id_selected_scale_factor_parameters(1) != [128.0, 16.0]
        or sonare_codec.aac_standard_id_selected_scale_factor_parameters(2) != [126.0, 16.0]
    ):
        sys.exit("Python wheel AAC standard-id selected-scale-factor parameter helper returned unexpected values")
    aac_10k = sonare_codec.encode_aac_with_bitrate(44100, 1, [0.0] * 2048, 10000)
    if not isinstance(aac_10k, bytes) or not aac_10k.startswith(b"\xff\xf1") or max_adts_frame_len(aac_10k) > 30:
        sys.exit("Python wheel AAC bitrate encode helper returned unexpected bytes")
    selected_aac_10k = sonare_codec.encode_aac_with_selected_scale_factors_and_bitrate(44100, 1, [0.0] * 2048, 10000)
    if not isinstance(selected_aac_10k, bytes) or not selected_aac_10k.startswith(b"\xff\xf1") or max_adts_frame_len(selected_aac_10k) > 30:
        sys.exit("Python wheel selected-scale-factor AAC bitrate encode helper returned unexpected bytes")
    m4a_10k = sonare_codec.encode_m4a_with_bitrate(44100, 1, [0.0] * 2048, 10000)
    if not isinstance(m4a_10k, bytes) or b"ftyp" not in m4a_10k[:16]:
        sys.exit("Python wheel M4A bitrate encode helper returned unexpected bytes")
    if sonare_codec.demux_m4a_as_aac_adts(m4a_10k) != aac_10k:
        sys.exit("Python wheel M4A bitrate encode helper did not mux the expected ADTS")
    selected_m4a_10k = sonare_codec.encode_m4a_with_selected_scale_factors_and_bitrate(44100, 1, [0.0] * 2048, 10000)
    if not isinstance(selected_m4a_10k, bytes) or b"ftyp" not in selected_m4a_10k[:16]:
        sys.exit("Python wheel selected-scale-factor M4A bitrate encode helper returned unexpected bytes")
    if sonare_codec.demux_m4a_as_aac_adts(selected_m4a_10k) != selected_aac_10k:
        sys.exit("Python wheel selected-scale-factor M4A bitrate encode helper did not mux the expected ADTS")
    if sonare_codec.aac_unsigned_pairs7_unit_magnitude_table() != [0, 0, 0, 1, 0, 1, 5, 3, 1, 0, 4, 3, 1, 1, 12, 4]:
        sys.exit("Python wheel AAC codebook 7 helper returned unexpected entries")
    pairs7_table = sonare_codec.aac_unsigned_pairs7_table()
    if len(pairs7_table) != 256 or pairs7_table[:4] != [0, 0, 0, 1] or pairs7_table[36:40] != [1, 1, 12, 4] or pairs7_table[-4:] != [7, 7, 4095, 12]:
        sys.exit("Python wheel AAC full codebook 7 helper returned unexpected entries")
    signed_pairs5 = sonare_codec.aac_signed_pairs5_table()
    if len(signed_pairs5) != 324 or signed_pairs5[:4] != [-4, -4, 8191, 13] or signed_pairs5[160:164] != [0, 0, 0, 1] or signed_pairs5[-4:] != [4, 4, 8190, 13]:
        sys.exit("Python wheel AAC signed-pairs codebook 5 helper returned unexpected entries")
    signed_pairs6 = sonare_codec.aac_signed_pairs6_table()
    if len(signed_pairs6) != 324 or signed_pairs6[:4] != [-4, -4, 2046, 11] or signed_pairs6[160:164] != [0, 0, 0, 4] or signed_pairs6[-4:] != [4, 4, 2044, 11]:
        sys.exit("Python wheel AAC signed-pairs codebook 6 helper returned unexpected entries")
    signed_quads1 = sonare_codec.aac_signed_quads1_table()
    if len(signed_quads1) != 486 or signed_quads1[:6] != [-1, -1, -1, -1, 2040, 11] or signed_quads1[240:246] != [0, 0, 0, 0, 0, 1] or signed_quads1[-6:] != [1, 1, 1, 1, 2036, 11]:
        sys.exit("Python wheel AAC signed-quad codebook 1 helper returned unexpected entries")
    signed_quads2 = sonare_codec.aac_signed_quads2_table()
    if len(signed_quads2) != 486 or signed_quads2[:6] != [-1, -1, -1, -1, 499, 9] or signed_quads2[240:246] != [0, 0, 0, 0, 0, 3] or signed_quads2[-6:] != [1, 1, 1, 1, 502, 9]:
        sys.exit("Python wheel AAC signed-quad codebook 2 helper returned unexpected entries")
    quads3 = sonare_codec.aac_unsigned_quads3_table()
    if len(quads3) != 486 or quads3[:6] != [0, 0, 0, 0, 0, 1] or quads3[240:246] != [1, 1, 1, 1, 116, 7] or quads3[-6:] != [2, 2, 2, 2, 32762, 15]:
        sys.exit("Python wheel AAC unsigned-quad codebook 3 helper returned unexpected entries")
    quads4 = sonare_codec.aac_unsigned_quads4_table()
    if len(quads4) != 486 or quads4[:6] != [0, 0, 0, 0, 7, 4] or quads4[240:246] != [1, 1, 1, 1, 0, 4] or quads4[-6:] != [2, 2, 2, 2, 2044, 11]:
        sys.exit("Python wheel AAC unsigned-quad codebook 4 helper returned unexpected entries")
    pairs8_table = sonare_codec.aac_unsigned_pairs8_table()
    if len(pairs8_table) != 256 or pairs8_table[:4] != [0, 0, 14, 5] or pairs8_table[36:40] != [1, 1, 0, 3] or pairs8_table[-4:] != [7, 7, 1023, 10]:
        sys.exit("Python wheel AAC full codebook 8 helper returned unexpected entries")
    pairs9_table = sonare_codec.aac_unsigned_pairs9_table()
    if len(pairs9_table) != 676 or pairs9_table[:4] != [0, 0, 0, 1] or pairs9_table[56:60] != [1, 1, 12, 4] or pairs9_table[-4:] != [12, 12, 32767, 15]:
        sys.exit("Python wheel AAC full codebook 9 helper returned unexpected entries")
    pairs10_table = sonare_codec.aac_unsigned_pairs10_table()
    if len(pairs10_table) != 676 or pairs10_table[:4] != [0, 0, 34, 6] or pairs10_table[56:60] != [1, 1, 0, 4] or pairs10_table[-4:] != [12, 12, 4095, 12]:
        sys.exit("Python wheel AAC full codebook 10 helper returned unexpected entries")
    escape_table = sonare_codec.aac_escape_table()
    if len(escape_table) != 1156 or escape_table[:4] != [0, 0, 0, 4] or escape_table[72:76] != [1, 1, 1, 4] or escape_table[-4:] != [16, 16, 4, 5]:
        sys.exit("Python wheel AAC escape codebook helper returned unexpected entries")
    scale_factor_table = sonare_codec.aac_scale_factor_delta_table()
    if len(scale_factor_table) != 363 or scale_factor_table[:3] != [-60, 262120, 18] or scale_factor_table[180:183] != [0, 0, 1] or scale_factor_table[-3:] != [60, 524275, 19]:
        sys.exit("Python wheel AAC scale-factor delta helper returned unexpected entries")
    if sonare_codec.aac_codebook6_unit_section_plan([1, -1, 0, 0], 2) != [0, 2, 6, 2, 4, 0]:
        sys.exit("Python wheel AAC codebook 6 section planner returned unexpected sections")
    if sonare_codec.aac_quad_unit_section_plan([1, -1, 0, 1, 0, 1, -1, 0, 0, 0, 0, 0], 4) != [0, 8, 3, 8, 12, 0]:
        sys.exit("Python wheel AAC quad section planner returned unexpected sections")
    if sonare_codec.aac_mixed_unit_section_plan([1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0], 4) != [0, 4, 3, 4, 8, 6, 8, 12, 0]:
        sys.exit("Python wheel AAC mixed section planner returned unexpected sections")
    if sonare_codec.aac_mixed_unit_payload_bit_lengths([1, -1, 0, 1, 1, -1, 1, -1, 0, 0, 0, 0], 4) != [27, 11, 38, 29, 11, 40]:
        sys.exit("Python wheel AAC mixed payload bit lengths returned unexpected values")
    if sonare_codec.aac_standard_unit_section_plan([1, -1, 17, 0], 2) != [0, 2, 6, 2, 4, 11]:
        sys.exit("Python wheel AAC standard section planner returned unexpected sections")
    if sonare_codec.aac_standard_unit_section_plan([0, 1], 2) != [0, 2, 5]:
        sys.exit("Python wheel AAC standard signed-pairs codebook 5 planner returned unexpected sections")
    if sonare_codec.aac_standard_unit_section_plan([1, -1, 0, 1, 17, 0, 0, 0], 4) != [0, 4, 4, 4, 8, 11]:
        sys.exit("Python wheel AAC standard mixed section planner returned unexpected sections")
    if sonare_codec.aac_standard_offsets_section_plan([1, -1, 0, 1, 17, 0, 0, 0], [0, 4, 8]) != [0, 4, 4, 4, 8, 11]:
        sys.exit("Python wheel AAC standard mixed offsets section planner returned unexpected sections")
    if sonare_codec.aac_standard_escape_payload_bit_lengths() != [9, 15, 24]:
        sys.exit("Python wheel AAC standard escape payload bit lengths returned unexpected values")
    if sonare_codec.aac_standard_mixed_payload_bit_lengths([1, -1, 0, 1, 17, 0, 0, 0], 4) != [18, 26, 44, 20, 26, 46]:
        sys.exit("Python wheel AAC standard mixed payload bit lengths returned unexpected values")
    if sonare_codec.aac_standard_mixed_offsets_payload_bit_lengths([1, -1, 0, 1, 17, 0, 0, 0], [0, 4, 8]) != [18, 26, 44, 20, 26, 46]:
        sys.exit("Python wheel AAC standard mixed offsets payload bit lengths returned unexpected values")
    standard_mono_adts = sonare_codec.encode_aac_standard_mono_offsets_with_step(44100, [0.0] * 2048, 20.0, 128)
    if not isinstance(standard_mono_adts, bytes) or not standard_mono_adts.startswith(b"\xff\xf1") or max_adts_frame_len(standard_mono_adts) > 16:
        sys.exit("Python wheel AAC standard mono offsets stream helper returned unexpected ADTS")
    standard_mono_bitrate_adts = sonare_codec.encode_aac_standard_mono_offsets_with_bitrate(44100, [0.0] * 2048, 128000, 128)
    if not isinstance(standard_mono_bitrate_adts, bytes) or not standard_mono_bitrate_adts.startswith(b"\xff\xf1") or max_adts_frame_len(standard_mono_bitrate_adts) > 372:
        sys.exit("Python wheel AAC standard mono offsets bitrate stream helper returned unexpected ADTS")
    standard_generic_adts = sonare_codec.encode_aac_with_standard_spectral_offsets_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128)
    if not isinstance(standard_generic_adts, bytes) or not standard_generic_adts.startswith(b"\xff\xf1") or max_adts_frame_len(standard_generic_adts) > 372:
        sys.exit("Python wheel AAC standard spectral-offset bitrate helper returned unexpected ADTS")
    standard_generic_m4a = sonare_codec.encode_m4a_with_standard_spectral_offsets_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128)
    if not isinstance(standard_generic_m4a, bytes) or standard_generic_m4a[4:8] != b"ftyp":
        sys.exit("Python wheel M4A standard spectral-offset bitrate helper returned unexpected container")
    standard_selected_generic_adts = sonare_codec.encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16)
    if not isinstance(standard_selected_generic_adts, bytes) or not standard_selected_generic_adts.startswith(b"\xff\xf1") or max_adts_frame_len(standard_selected_generic_adts) > 372:
        sys.exit("Python wheel AAC standard selected spectral-offset bitrate helper returned unexpected ADTS")
    recommended_standard_selected_generic_adts = sonare_codec.encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, [0.0] * 2048, 128000)
    if recommended_standard_selected_generic_adts != standard_selected_generic_adts:
        sys.exit("Python wheel AAC recommended standard selected helper did not match explicit mono parameters")
    standard_selected_generic_m4a = sonare_codec.encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16)
    if not isinstance(standard_selected_generic_m4a, bytes) or standard_selected_generic_m4a[4:8] != b"ftyp":
        sys.exit("Python wheel M4A standard selected spectral-offset bitrate helper returned unexpected container")
    recommended_standard_selected_generic_m4a = sonare_codec.encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, [0.0] * 2048, 128000)
    if recommended_standard_selected_generic_m4a != standard_selected_generic_m4a:
        sys.exit("Python wheel M4A recommended standard selected helper did not match explicit mono parameters")
    standard_selected_details = sonare_codec.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16)
    if len(standard_selected_details) != 8 or standard_selected_details[0] != 0 or standard_selected_details[4] != 1 or standard_selected_details[2] > 372 or standard_selected_details[6] > 372:
        sys.exit(f"Python wheel AAC standard selected bitrate details returned {standard_selected_details}")
    recommended_standard_selected_details = sonare_codec.aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if recommended_standard_selected_details != standard_selected_details:
        sys.exit(f"Python wheel AAC recommended standard selected bitrate details returned {recommended_standard_selected_details}")
    production_selected_details = sonare_codec.aac_selected_scale_factor_frame_details_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if len(production_selected_details) != 8 or production_selected_details[0] != 0 or production_selected_details[4] != 1 or production_selected_details[2] > 372 or production_selected_details[6] > 372:
        sys.exit(f"Python wheel AAC production selected bitrate details returned {production_selected_details}")
    standard_mono_bitrate_details = sonare_codec.aac_standard_mono_offsets_bitrate_frame_details(44100, [0.0] * 2048, 128000, 128)
    if len(standard_mono_bitrate_details) != 8 or standard_mono_bitrate_details[0] != 0 or standard_mono_bitrate_details[4] != 1 or standard_mono_bitrate_details[2] > 372 or standard_mono_bitrate_details[6] > 372:
        sys.exit(f"Python wheel AAC standard mono offsets bitrate details returned {standard_mono_bitrate_details}")
    standard_stereo_adts = sonare_codec.encode_aac_standard_stereo_offsets_with_step(44100, [0.0] * 4096, 20.0, 128)
    if not isinstance(standard_stereo_adts, bytes) or not standard_stereo_adts.startswith(b"\xff\xf1") or max_adts_frame_len(standard_stereo_adts) > 28:
        sys.exit("Python wheel AAC standard stereo offsets stream helper returned unexpected ADTS")
    standard_stereo_bitrate_adts = sonare_codec.encode_aac_standard_stereo_offsets_with_bitrate(44100, [0.0] * 4096, 256000, 128)
    if not isinstance(standard_stereo_bitrate_adts, bytes) or not standard_stereo_bitrate_adts.startswith(b"\xff\xf1") or max_adts_frame_len(standard_stereo_bitrate_adts) > 744:
        sys.exit("Python wheel AAC standard stereo offsets bitrate stream helper returned unexpected ADTS")
    standard_stereo_bitrate_details = sonare_codec.aac_standard_stereo_offsets_bitrate_frame_details(44100, [0.0] * 4096, 256000, 128)
    if len(standard_stereo_bitrate_details) != 8 or standard_stereo_bitrate_details[0] != 0 or standard_stereo_bitrate_details[4] != 1 or standard_stereo_bitrate_details[2] > 744 or standard_stereo_bitrate_details[6] > 744:
        sys.exit(f"Python wheel AAC standard stereo offsets bitrate details returned {standard_stereo_bitrate_details}")
    standard_selected_stereo_details = sonare_codec.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(44100, 2, [0.0] * 4096, 256000, 128, 16)
    if len(standard_selected_stereo_details) != 8 or standard_selected_stereo_details[0] != 0 or standard_selected_stereo_details[4] != 1 or standard_selected_stereo_details[2] > 744 or standard_selected_stereo_details[6] > 744:
        sys.exit(f"Python wheel AAC standard selected stereo bitrate details returned {standard_selected_stereo_details}")
    recommended_standard_selected_stereo_details = sonare_codec.aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(44100, 2, [0.0] * 4096, 256000)
    explicit_recommended_standard_selected_stereo_details = sonare_codec.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(44100, 2, [0.0] * 4096, 256000, 126, 16)
    if recommended_standard_selected_stereo_details != explicit_recommended_standard_selected_stereo_details:
        sys.exit(f"Python wheel AAC recommended standard selected stereo details returned {recommended_standard_selected_stereo_details}")
    production_selected_stereo_details = sonare_codec.aac_selected_scale_factor_frame_details_with_bitrate(44100, 2, [0.0] * 4096, 256000)
    if len(production_selected_stereo_details) != 8 or production_selected_stereo_details[0] != 0 or production_selected_stereo_details[4] != 1 or production_selected_stereo_details[2] > 744 or production_selected_stereo_details[6] > 744:
        sys.exit(f"Python wheel AAC production selected stereo bitrate details returned {production_selected_stereo_details}")
    if sonare_codec.mp3_layer3_main_data_capacity_bytes(44100, 1, 128, False, False) != 396:
        sys.exit("Python wheel MP3 capacity byte helper returned an unexpected value")
    if sonare_codec.mp3_layer3_main_data_capacity_bits(44100, 1, 128, False, False) != 3168:
        sys.exit("Python wheel MP3 capacity bit helper returned an unexpected value")
    if sonare_codec.mp3_layer3_main_data_capacity_bytes(44100, 1, 96, False, False) != 292:
        sys.exit("Python wheel MP3 96kbps capacity byte helper returned an unexpected value")
    if sonare_codec.mp3_layer3_main_data_capacity_bits(44100, 1, 96, False, False) != 2336:
        sys.exit("Python wheel MP3 96kbps capacity bit helper returned an unexpected value")
    mp3_steps = sonare_codec.mp3_pcm_step_candidates()
    if not has_approx(mp3_steps, 0.2) or has_approx(mp3_steps, 0.15):
        sys.exit(f"Python wheel MP3 step candidates returned {mp3_steps}")
    if sonare_codec.mp3_standard_big_value_table_selects() != [1,2,3,5,6,7,8,9,10,11,12,13,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31]:
        sys.exit("Python wheel MP3 standard table selector helper returned unexpected values")
    if sonare_codec.mp3_missing_standard_big_value_table_selects() != []:
        sys.exit("Python wheel MP3 missing standard table selector helper returned unexpected values")
    if sonare_codec.mp3_standard_count1_table_selects() != [0, 1]:
        sys.exit("Python wheel MP3 count1 selector helper returned unexpected values")
    mp3_96k = sonare_codec.encode_mp3_with_bitrate(44100, 1, [0.0] * 1152, 96, False, False)
    if not isinstance(mp3_96k, bytes) or mp3_frame_info(mp3_96k) != (96, 44100, 1, len(mp3_96k)):
        sys.exit("Python wheel MP3 bitrate encode helper returned an unexpected frame budget")
    mp3_cbr_128k = sonare_codec.encode_mp3_cbr_with_bitrate(44100, 1, [0.0] * (1152 * 3), 128, False)
    first_cbr = mp3_frame_info(mp3_cbr_128k)
    second_cbr = mp3_frame_info(mp3_cbr_128k[first_cbr[3]:])
    third_cbr = mp3_frame_info(mp3_cbr_128k[first_cbr[3] + second_cbr[3]:])
    if (
        not isinstance(mp3_cbr_128k, bytes)
        or first_cbr != (128, 44100, 1, 417)
        or second_cbr != (128, 44100, 1, 418)
        or third_cbr != (128, 44100, 1, 418)
        or len(mp3_cbr_128k) != 1253
    ):
        sys.exit("Python wheel MP3 CBR bitrate helper returned an unexpected padding schedule")
    perceptual_samples = [math.sin(index * 0.013) * 0.25 for index in range(1152 * 3)]
    mp3_candidate_profile = sonare_codec.mp3_first_frame_perceptual_candidate_profile_with_bitrate(44100, 1, perceptual_samples, 128, False)
    if (
        len(mp3_candidate_profile) < 6
        or len(mp3_candidate_profile) % 6 != 0
        or not has_approx([mp3_candidate_profile[0]], 0.0005)
        or mp3_candidate_profile[4] != 42.0
        or not any(value > 0 for index, value in enumerate(mp3_candidate_profile) if index % 6 == 3)
    ):
        sys.exit(f"Python wheel MP3 first-frame perceptual candidate profile returned {mp3_candidate_profile}")
    mp3_bit_allocation = sonare_codec.mp3_perceptual_bit_allocation_with_bitrate(44100, 1, perceptual_samples, 128, False, 0)
    mp3_target_bits = sum(value for index, value in enumerate(mp3_bit_allocation) if index % 5 == 4)
    if (
        len(mp3_bit_allocation) != 30
        or mp3_bit_allocation[0] != 0.0
        or mp3_bit_allocation[1] != 0.0
        or mp3_bit_allocation[2] != 0.0
        or not math.isfinite(mp3_bit_allocation[3])
        or mp3_target_bits != 9520.0
    ):
        sys.exit(f"Python wheel MP3 perceptual bit allocation returned {mp3_bit_allocation}")
    mp3_perceptual_cbr_128k = sonare_codec.encode_mp3_perceptual_active_cbr_with_bitrate(44100, 1, perceptual_samples, 128, False)
    first_perceptual = mp3_frame_info(mp3_perceptual_cbr_128k)
    second_perceptual = mp3_frame_info(mp3_perceptual_cbr_128k[first_perceptual[3]:])
    third_perceptual = mp3_frame_info(mp3_perceptual_cbr_128k[first_perceptual[3] + second_perceptual[3]:])
    if (
        not isinstance(mp3_perceptual_cbr_128k, bytes)
        or first_perceptual != (128, 44100, 1, 417)
        or second_perceptual != (128, 44100, 1, 418)
        or third_perceptual != (128, 44100, 1, 418)
        or len(mp3_perceptual_cbr_128k) != 1253
    ):
        sys.exit("Python wheel MP3 perceptual active CBR helper returned an unexpected padding schedule")
    reservoir_samples = []
    for index in range(1152 * 8):
        frame = index // 1152
        t = index % 1152
        if frame % 2 == 0:
            reservoir_samples.append(0.3 * (math.sin(t * 0.043) + math.sin(t * 0.131) + math.sin(t * 0.277) + math.sin(t * 0.611)))
        else:
            reservoir_samples.append(0.02 * math.sin(t * 0.05))
    reservoir_stereo_samples = []
    for frame in range(8):
        for t in range(1152):
            if frame % 2 == 0:
                reservoir_stereo_samples.append(0.28 * (math.sin(t * 0.037) + math.sin(t * 0.149) + math.sin(t * 0.419)))
                reservoir_stereo_samples.append(0.24 * (math.sin(t * 0.053) + math.sin(t * 0.173) + math.sin(t * 0.337)))
            else:
                reservoir_stereo_samples.append(0.02 * math.sin(t * 0.041))
                reservoir_stereo_samples.append(0.018 * math.sin(t * 0.047))

    def check_mp3_production_reservoir(label, channels, samples):
        detail_width = 14
        granules_per_frame = 2 if channels == 1 else 4
        detail_helper = sonare_codec.mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate
        reservoir_details = detail_helper(44100, channels, samples, 128, False, 0)
        if len(reservoir_details) != 8 * detail_width or reservoir_details[0] != 0 or reservoir_details[6] != 0:
            sys.exit(f"Python wheel {label} MP3 reservoir detail helper returned malformed frame details")
        reservoir_borrowed = False
        entropy_target_bits = 0
        capacity_bits = 0
        entropy_target_budget_frames = 0
        for offset in range(0, len(reservoir_details), detail_width):
            payload_bits = reservoir_details[offset + 2]
            frame_len = reservoir_details[offset + 3]
            padding = reservoir_details[offset + 4]
            capacity_bytes = reservoir_details[offset + 5]
            main_data_begin = reservoir_details[offset + 6]
            perceptual_granules = reservoir_details[offset + 8]
            calibrated_granules = reservoir_details[offset + 9]
            quality_guard_compared_granules = reservoir_details[offset + 10]
            quality_guard_distortion_delta = reservoir_details[offset + 11]
            frame_entropy_target_bits = reservoir_details[offset + 12]
            used_entropy_target_budget = reservoir_details[offset + 13]
            entropy_target_bits += frame_entropy_target_bits
            capacity_bits += capacity_bytes * 8
            if used_entropy_target_budget == 1:
                entropy_target_budget_frames += 1
            if main_data_begin > 0:
                reservoir_borrowed = True
            if frame_len not in (417, 418) or padding not in (0, 1):
                sys.exit(f"Python wheel {label} MP3 reservoir detail helper reported an unexpected CBR frame slot")
            if payload_bits > (capacity_bytes + main_data_begin) * 8:
                sys.exit(f"Python wheel {label} MP3 reservoir detail helper reported an over-budget frame")
            if perceptual_granules + calibrated_granules != granules_per_frame:
                sys.exit(f"Python wheel {label} MP3 reservoir detail helper reported inconsistent granule telemetry")
            if perceptual_granules != granules_per_frame or calibrated_granules != 0:
                sys.exit(f"Python wheel {label} MP3 production reservoir did not report perceptual granules")
            if quality_guard_compared_granules != 0 or quality_guard_distortion_delta != 0:
                sys.exit(f"Python wheel {label} MP3 production reservoir unexpectedly reported quality guard telemetry")
        if not reservoir_borrowed:
            sys.exit(f"Python wheel {label} MP3 reservoir detail helper never reported main_data_begin borrowing")
        if entropy_target_bits != capacity_bits or entropy_target_budget_frames == 0:
            sys.exit(f"Python wheel {label} MP3 entropy-targeted production reservoir failed target checks")
        production_reservoir_mp3 = sonare_codec.encode_audio_production("mp3", 44100, channels, samples)
        entropy_targeted_production_mp3 = sonare_codec.encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(44100, channels, samples, 128, False, 0)
        if production_reservoir_mp3 != entropy_targeted_production_mp3:
            sys.exit(f"Python wheel {label} MP3 production did not use the entropy-targeted perceptual reservoir path")
        production_main_data_begins = mp3_main_data_begins(production_reservoir_mp3)
        if len(production_main_data_begins) * detail_width != len(reservoir_details):
            sys.exit(f"Python wheel {label} MP3 production reservoir frame count did not match selector details")
        for frame, main_data_begin in enumerate(production_main_data_begins):
            if main_data_begin != reservoir_details[frame * detail_width + 6]:
                sys.exit(f"Python wheel {label} MP3 production reservoir side-info did not match selector details")

    check_mp3_production_reservoir("mono", 1, reservoir_samples)
    check_mp3_production_reservoir("stereo", 2, reservoir_stereo_samples)
    perceptual_reservoir_details = sonare_codec.mp3_perceptual_reservoir_frame_details_with_bitrate(44100, 1, reservoir_samples, 128, False)
    reservoir_detail_width = 12
    if len(perceptual_reservoir_details) != 8 * reservoir_detail_width or perceptual_reservoir_details[0] != 0 or perceptual_reservoir_details[6] != 0:
        sys.exit("Python wheel MP3 perceptual reservoir detail helper returned malformed frame details")
    perceptual_reservoir_borrowed = False
    for offset in range(0, len(perceptual_reservoir_details), reservoir_detail_width):
        payload_bits = perceptual_reservoir_details[offset + 2]
        frame_len = perceptual_reservoir_details[offset + 3]
        padding = perceptual_reservoir_details[offset + 4]
        capacity_bytes = perceptual_reservoir_details[offset + 5]
        main_data_begin = perceptual_reservoir_details[offset + 6]
        perceptual_granules = perceptual_reservoir_details[offset + 8]
        calibrated_granules = perceptual_reservoir_details[offset + 9]
        quality_guard_compared_granules = perceptual_reservoir_details[offset + 10]
        quality_guard_distortion_delta = perceptual_reservoir_details[offset + 11]
        if main_data_begin > 0:
            perceptual_reservoir_borrowed = True
        if frame_len not in (417, 418) or padding not in (0, 1):
            sys.exit("Python wheel MP3 perceptual reservoir detail helper reported an unexpected CBR frame slot")
        if payload_bits > (capacity_bytes + main_data_begin) * 8:
            sys.exit("Python wheel MP3 perceptual reservoir detail helper reported an over-budget frame")
        if perceptual_granules != 2 or calibrated_granules != 0:
            sys.exit("Python wheel MP3 perceptual reservoir detail helper reported unexpected granule telemetry")
        if quality_guard_compared_granules != 0 or quality_guard_distortion_delta != 0:
            sys.exit("Python wheel MP3 perceptual reservoir detail helper unexpectedly reported quality guard telemetry")
    if not perceptual_reservoir_borrowed:
        sys.exit("Python wheel MP3 perceptual reservoir detail helper never reported main_data_begin borrowing")
    entropy_targeted_reservoir_details = sonare_codec.mp3_entropy_targeted_perceptual_reservoir_frame_details_with_bitrate(44100, 1, reservoir_samples, 128, False, 0)
    entropy_targeted_reservoir_detail_width = 14
    if len(entropy_targeted_reservoir_details) != 8 * entropy_targeted_reservoir_detail_width or entropy_targeted_reservoir_details[0] != 0 or entropy_targeted_reservoir_details[6] != 0:
        sys.exit(f"Python wheel MP3 entropy-targeted perceptual reservoir details returned {entropy_targeted_reservoir_details}")
    entropy_targeted_reservoir_bits = sum(value for index, value in enumerate(entropy_targeted_reservoir_details) if index % entropy_targeted_reservoir_detail_width == 12)
    entropy_targeted_reservoir_capacity_bits = sum(value * 8 for index, value in enumerate(perceptual_reservoir_details) if index % reservoir_detail_width == 5)
    if entropy_targeted_reservoir_bits != entropy_targeted_reservoir_capacity_bits or not any(value == 1.0 for index, value in enumerate(entropy_targeted_reservoir_details) if index % entropy_targeted_reservoir_detail_width == 13):
        sys.exit(f"Python wheel MP3 entropy-targeted perceptual reservoir details failed target checks: {entropy_targeted_reservoir_details}")
    entropy_targeted_reservoir_mp3 = sonare_codec.encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(44100, 1, reservoir_samples, 128, False, 0)
    entropy_targeted_reservoir_main_data_begins = mp3_main_data_begins(entropy_targeted_reservoir_mp3)
    if len(entropy_targeted_reservoir_main_data_begins) * entropy_targeted_reservoir_detail_width != len(entropy_targeted_reservoir_details):
        sys.exit("Python wheel MP3 entropy-targeted perceptual reservoir frame count did not match selector details")
    for frame, main_data_begin in enumerate(entropy_targeted_reservoir_main_data_begins):
        if main_data_begin != entropy_targeted_reservoir_details[frame * entropy_targeted_reservoir_detail_width + 6]:
            sys.exit("Python wheel MP3 entropy-targeted perceptual reservoir side-info did not match selector details")
    perceptual_reservoir_mp3 = sonare_codec.encode_mp3_perceptual_reservoir_with_bitrate(44100, 1, reservoir_samples, 128, False)
    entropy_targeted_reservoir_production = sonare_codec.encode_audio_production("mp3", 44100, 1, reservoir_samples)
    if entropy_targeted_reservoir_production != entropy_targeted_reservoir_mp3:
        sys.exit("Python wheel MP3 mono production did not use the entropy-targeted perceptual reservoir path")
    perceptual_reservoir_main_data_begins = mp3_main_data_begins(perceptual_reservoir_mp3)
    if len(perceptual_reservoir_main_data_begins) * reservoir_detail_width != len(perceptual_reservoir_details):
        sys.exit("Python wheel MP3 perceptual reservoir frame count did not match selector details")
    for frame, main_data_begin in enumerate(perceptual_reservoir_main_data_begins):
        if main_data_begin != perceptual_reservoir_details[frame * reservoir_detail_width + 6]:
            sys.exit("Python wheel MP3 perceptual reservoir side-info did not match selector details")

    silent = sonare_codec.encode_audio_production("mp3", 44100, 1, [0.0] * 1152)
    if not isinstance(silent, bytes) or not silent:
        sys.exit("Python wheel encode_audio_production did not return MP3 bytes")
    try:
        production_mp3 = sonare_codec.encode_audio_production("mp3", 44100, 1, [0.25] + [0.0] * 1151)
    except ValueError as exc:
        sys.exit("Python wheel encode_audio_production rejected non-silent MP3: " + str(exc))
    else:
        if not isinstance(production_mp3, bytes) or mp3_frame_info(production_mp3) != (128, 44100, 1, len(production_mp3)):
            sys.exit("Python wheel encode_audio_production did not return default-budget non-silent MP3 bytes")
    try:
        production_mp3_stereo = sonare_codec.encode_audio_production("mp3", 44100, 2, [0.25, 0.0] + [0.0] * 2302)
    except ValueError as exc:
        sys.exit("Python wheel encode_audio_production rejected non-silent stereo MP3: " + str(exc))
    else:
        if not isinstance(production_mp3_stereo, bytes) or mp3_frame_info(production_mp3_stereo) != (128, 44100, 2, len(production_mp3_stereo)):
            sys.exit("Python wheel encode_audio_production did not return default-budget non-silent stereo MP3 bytes")
    try:
        production_m4a = sonare_codec.encode_audio_production("m4a", 44100, 1, [0.25] + [0.0] * 2047)
    except ValueError as exc:
        sys.exit("Python wheel encode_audio_production rejected non-silent M4A: " + str(exc))
    else:
        if not isinstance(production_m4a, bytes) or b"ftyp" not in production_m4a[:16]:
            sys.exit("Python wheel encode_audio_production did not return non-silent M4A bytes")
        if sonare_codec.detect_format(production_m4a) != "m4a":
            sys.exit("Python wheel detect_format did not identify production M4A")
    opus = sonare_codec.encode_audio("opus", 48000, 1, [0.0] * 4800)
    if not isinstance(opus, bytes) or not opus.startswith(b"OggS"):
        sys.exit("Python wheel Opus encode did not return Ogg bytes")
    production_opus = sonare_codec.encode_audio_production("opus", 48000, 1, [0.0] * 4800)
    if not isinstance(production_opus, bytes) or not production_opus.startswith(b"OggS"):
        sys.exit("Python wheel encode_audio_production did not return Opus Ogg bytes")
    if sonare_codec.detect_format(production_opus) != "opus":
        sys.exit("Python wheel detect_format did not identify production Opus")
    direct_opus = sonare_codec.encode_opus(48000, 1, [0.0] * 4800)
    if not isinstance(direct_opus, bytes) or not direct_opus.startswith(b"OggS"):
        sys.exit("Python wheel encode_opus did not return Ogg bytes")
    opus_pcm = sonare_codec.decode_opus(direct_opus)
    if opus_pcm[0] != 48000 or opus_pcm[1] != 1 or not opus_pcm[2]:
        sys.exit("Python wheel decode_opus returned unexpected PCM metadata")
    if sonare_codec.detect_format(direct_opus) != "opus":
        sys.exit("Python wheel detect_format did not identify encoded Opus")
    vorbis = sonare_codec.encode_audio("vorbis", 48000, 1, [0.0] * 4800)
    if not isinstance(vorbis, bytes) or not vorbis.startswith(b"OggS"):
        sys.exit("Python wheel Vorbis encode did not return Ogg bytes")
    production_vorbis = sonare_codec.encode_audio_production("vorbis", 48000, 1, [0.0] * 4800)
    if not isinstance(production_vorbis, bytes) or not production_vorbis.startswith(b"OggS"):
        sys.exit("Python wheel encode_audio_production did not return Vorbis Ogg bytes")
    if sonare_codec.detect_format(production_vorbis) != "vorbis":
        sys.exit("Python wheel detect_format did not identify production Vorbis")
    direct_vorbis = sonare_codec.encode_vorbis(48000, 1, [0.0] * 4800)
    if not isinstance(direct_vorbis, bytes) or not direct_vorbis.startswith(b"OggS"):
        sys.exit("Python wheel encode_vorbis did not return Ogg bytes")
    vorbis_pcm = sonare_codec.decode_vorbis(direct_vorbis)
    if vorbis_pcm[0] != 48000 or vorbis_pcm[1] != 1 or not vorbis_pcm[2]:
        sys.exit("Python wheel decode_vorbis returned unexpected PCM metadata")
    if sonare_codec.detect_format(direct_vorbis) != "vorbis":
        sys.exit("Python wheel detect_format did not identify encoded Vorbis")
print("checked " + wheel)
"#;
    let label = format!(
        "{} -c <python wheel output check>",
        python.to_string_lossy()
    );
    let mut command = Command::new(&python);
    command.args(["-c", script]);
    run_prepared_command(&mut command, &label)
}

struct SizeEntry {
    kind: &'static str,
    path: PathBuf,
    bytes: Option<u64>,
}

fn collect_size_report() -> Result<Vec<SizeEntry>, String> {
    let mut entries = Vec::new();
    for package in RUST_PUBLISH_PACKAGES {
        entries.push(size_entry(
            "rust crate",
            PathBuf::from(format!(
                "target/package/{}-{RELEASE_VERSION}.crate",
                package.package
            )),
        )?);
    }

    entries.extend(size_entries_from_dir(
        "npm tarball",
        Path::new("bindings/wasm"),
        ".tgz",
    )?);
    entries.push(size_entry(
        "wasm binary",
        PathBuf::from("bindings/wasm/pkg/sonare_codec_wasm_bg.wasm"),
    )?);
    entries.push(size_entry(
        "wasm js",
        PathBuf::from("bindings/wasm/pkg/sonare_codec_wasm.js"),
    )?);
    entries.push(size_entry(
        "wasm d.ts",
        PathBuf::from("bindings/wasm/pkg/sonare_codec_wasm.d.ts"),
    )?);
    entries.extend(size_entries_from_dir(
        "python wheel",
        Path::new("target/wheels"),
        ".whl",
    )?);
    entries.extend(size_entries_from_dir(
        "python wheel",
        Path::new("bindings/python/target/wheels"),
        ".whl",
    )?);
    Ok(entries)
}

fn size_entry(kind: &'static str, path: PathBuf) -> Result<SizeEntry, String> {
    let bytes = match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => Some(metadata.len()),
        Ok(_) => None,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("failed to inspect {}: {err}", path.display())),
    };
    Ok(SizeEntry { kind, path, bytes })
}

fn size_entries_from_dir(
    kind: &'static str,
    dir: &Path,
    suffix: &str,
) -> Result<Vec<SizeEntry>, String> {
    let mut entries = Vec::new();
    match fs::read_dir(dir) {
        Ok(read_dir) => {
            for entry in read_dir {
                let entry =
                    entry.map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
                let path = entry.path();
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(suffix))
                {
                    entries.push(size_entry(kind, path)?);
                }
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            entries.push(SizeEntry {
                kind,
                path: dir.join(format!("*{suffix}")),
                bytes: None,
            });
        }
        Err(err) => return Err(format!("failed to read {}: {err}", dir.display())),
    }
    if entries.is_empty() {
        entries.push(SizeEntry {
            kind,
            path: dir.join(format!("*{suffix}")),
            bytes: None,
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn run_command<I, S>(program: I, args: &[S], cwd: impl AsRef<Path>) -> Result<(), String>
where
    I: Into<OsString>,
    S: AsRef<std::ffi::OsStr>,
{
    let program = program.into();
    let label = command_label(&program, args);
    let mut command = Command::new(&program);
    command.args(args).current_dir(cwd);
    run_prepared_command(&mut command, &label)
}

fn run_prepared_command(command: &mut Command, label: &str) -> Result<(), String> {
    eprintln!("running {label}");
    let status = command
        .status()
        .map_err(|err| format!("failed to run {label}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with status {status}"))
    }
}

fn run_command_output<I, S>(program: I, args: &[S], cwd: impl AsRef<Path>) -> Result<String, String>
where
    I: Into<OsString>,
    S: AsRef<std::ffi::OsStr>,
{
    let program = program.into();
    let label = command_label(&program, args);
    eprintln!("running {label}");
    let output = Command::new(&program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|err| format!("failed to run {label}: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.is_empty() {
        eprint!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    if output.status.success() {
        Ok(stdout.into_owned())
    } else {
        Err(format!("{label} failed with status {}", output.status))
    }
}

fn command_label<S>(program: &std::ffi::OsStr, args: &[S]) -> String
where
    S: AsRef<std::ffi::OsStr>,
{
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(program.to_string_lossy().into_owned());
    parts.extend(
        args.iter()
            .map(|arg| arg.as_ref().to_string_lossy().into_owned()),
    );
    parts.join(" ")
}

fn toml_string_value<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    input.lines().find_map(|line| {
        let (line_key, value) = line.split_once('=')?;
        if line_key.trim() != key {
            return None;
        }
        quoted_value(value.trim())
    })
}

fn json_string_value<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let quoted_key = format!("\"{key}\"");
    input.lines().find_map(|line| {
        let (line_key, value) = line.split_once(':')?;
        if line_key.trim() != quoted_key {
            return None;
        }
        quoted_value(value.trim().trim_end_matches(','))
    })
}

fn quoted_value(input: &str) -> Option<&str> {
    input
        .strip_prefix('"')?
        .split_once('"')
        .map(|(value, _)| value)
}

fn assert_contains(input: &str, needle: &str, label: &str) -> Result<(), String> {
    if input.contains(needle) {
        Ok(())
    } else {
        Err(format!("{label} is missing expected entry {needle}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        aac_standard_candidate_is_at_least_as_good, best_normalized_correlation,
        compatibility_lossy_encode_diagnostics, production_lossy_min_correlation, readiness_pcm,
        required_qa_tool_in_list, run_ffmpeg_acceptance, run_ffmpeg_clean_acceptance,
        run_ffmpeg_decode_f32le, validate_aac_standard_id_mixed_workbench,
        validate_aac_standard_id_production_correlation_gap, validate_adts_frame_budget,
        validate_diagnostic_quality_floor, validate_lossy_oracle_pcm_quality,
        validate_mp3_perceptual_reservoir_production_correlation_gap,
        verify_aac_default_production_budget, verify_diagnostic_lossy_encode_readiness,
        verify_mp3_default_production_budget, verify_mp3_production_reservoir,
        verify_production_lossy_oracle_acceptance, AacStandardDiagnosticCandidate,
        LossyOraclePcmQuality, ProductionArtifactKind, AAC_PRODUCTION_MIN_CORRELATION,
        MP3_PRODUCTION_MONO_MIN_CORRELATION, MP3_PRODUCTION_STEREO_MIN_CORRELATION,
    };

    #[test]
    fn parses_required_qa_tool_list() {
        assert!(required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "nextest"
        ));
        assert!(required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "audit"
        ));
        assert!(required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "machete"
        ));
        assert!(required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "semver-checks"
        ));
        assert!(!required_qa_tool_in_list(
            "nextest,audit machete;semver-checks",
            "miri"
        ));
    }

    #[test]
    fn required_qa_tool_all_matches_every_tool() {
        assert!(required_qa_tool_in_list("all", "nextest"));
        assert!(required_qa_tool_in_list("nextest,all", "llvm-cov"));
    }

    #[test]
    fn lossy_oracle_quality_allows_delayed_correlated_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let mut decoded = vec![0.0; 31];
        decoded.extend(expected.iter().map(|sample| sample * 0.9));
        decoded.extend([0.0; 17]);

        let quality = validate_lossy_oracle_pcm_quality(&expected, &decoded).unwrap();
        assert!(quality.decoded_rms > 0.0);
        assert!(quality.best_correlation > 0.99);
    }

    #[test]
    fn production_lossy_min_correlation_matches_release_floors() {
        assert_eq!(
            production_lossy_min_correlation(ProductionArtifactKind::Mp3, 1).unwrap(),
            MP3_PRODUCTION_MONO_MIN_CORRELATION
        );
        assert_eq!(
            production_lossy_min_correlation(ProductionArtifactKind::Mp3, 2).unwrap(),
            MP3_PRODUCTION_STEREO_MIN_CORRELATION
        );
        assert_eq!(
            production_lossy_min_correlation(ProductionArtifactKind::Aac, 1).unwrap(),
            AAC_PRODUCTION_MIN_CORRELATION
        );
        assert_eq!(
            production_lossy_min_correlation(ProductionArtifactKind::M4a, 2).unwrap(),
            AAC_PRODUCTION_MIN_CORRELATION
        );

        let err = production_lossy_min_correlation(ProductionArtifactKind::Mp3, 3).unwrap_err();
        assert!(err.contains("mono/stereo only"));
    }

    #[test]
    fn aac_standard_id_production_gap_is_release_gated() {
        let standard_id = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.550,
        };
        let production = LossyOraclePcmQuality {
            decoded_rms: 0.7004,
            best_correlation: 0.762,
        };
        validate_aac_standard_id_production_correlation_gap(
            "AAC standard-id mono",
            standard_id,
            production,
        )
        .unwrap();

        let regressed = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.490,
        };
        let err = validate_aac_standard_id_production_correlation_gap(
            "AAC standard-id mono",
            regressed,
            production,
        )
        .unwrap_err();
        assert!(err.contains("correlation gap to production exceeded diagnostic limit"));
    }

    #[test]
    fn aac_standard_id_rms_control_advantage_is_release_gated() {
        let standard_id = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.550,
        };
        let production = LossyOraclePcmQuality {
            decoded_rms: 0.7004,
            best_correlation: 0.762,
        };
        super::validate_aac_standard_id_rms_control_advantage(
            "AAC standard-id mono",
            standard_id,
            production,
            0.1750,
        )
        .unwrap();

        let regressed = LossyOraclePcmQuality {
            decoded_rms: 0.9100,
            best_correlation: 0.570,
        };
        let err = super::validate_aac_standard_id_rms_control_advantage(
            "AAC standard-id mono",
            regressed,
            production,
            0.1750,
        )
        .unwrap_err();
        assert!(err.contains("RMS control regressed behind production"));
    }

    #[test]
    fn aac_standard_id_frame_selection_comparison_reports_budget_deltas() {
        let production = [
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.2,
                frame_len: 300,
                frame_capacity_bytes: 372,
            },
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.1,
                frame_len: 240,
                frame_capacity_bytes: 372,
            },
        ];
        let standard_id = [
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.15,
                frame_len: 280,
                frame_capacity_bytes: 372,
            },
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.075,
                frame_len: 260,
                frame_capacity_bytes: 372,
            },
        ];

        let comparison =
            super::compare_aac_frame_selection_details(&production, &standard_id).unwrap();

        assert_eq!(comparison.frames, 2);
        assert_eq!(comparison.production_max_frame_len, 300);
        assert_eq!(comparison.standard_id_max_frame_len, 280);
        assert_eq!(comparison.max_frame_len_delta, -20);
        assert_eq!(comparison.production_min_budget_slack, 72);
        assert_eq!(comparison.standard_id_min_budget_slack, 92);
        assert_eq!(comparison.min_budget_slack_delta, 20);
        assert!((comparison.max_step_delta + 0.05).abs() < 1.0e-6);
    }

    #[test]
    fn aac_standard_id_frame_selection_comparison_rejects_shape_mismatch() {
        let production = [sonare_codec::AacPcmFrameStepSelection {
            step: 0.2,
            frame_len: 300,
            frame_capacity_bytes: 372,
        }];
        let standard_id = [
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.15,
                frame_len: 280,
                frame_capacity_bytes: 372,
            },
            sonare_codec::AacPcmFrameStepSelection {
                step: 0.075,
                frame_len: 260,
                frame_capacity_bytes: 372,
            },
        ];

        let err =
            super::compare_aac_frame_selection_details(&production, &standard_id).unwrap_err();

        assert!(err.contains("frame count diverged"));
    }

    #[test]
    fn aac_standard_id_candidate_set_comparison_tracks_promotion_blocker() {
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();

        let mono_recommended =
            super::compare_aac_standard_id_to_production_frame_selection(&mono).unwrap();
        let mono_production_step =
            super::compare_aac_standard_id_candidate_set_to_production_frame_selection(
                &mono,
                sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            )
            .unwrap();
        let stereo_recommended =
            super::compare_aac_standard_id_to_production_frame_selection(&stereo).unwrap();
        let stereo_production_step =
            super::compare_aac_standard_id_candidate_set_to_production_frame_selection(
                &stereo,
                sonare_codec::AAC_LC_PCM_STEP_CANDIDATES,
            )
            .unwrap();

        eprintln!(
            "AAC standard-id candidate-set blocker: mono recommended={mono_recommended:?}, mono production-step={mono_production_step:?}, stereo recommended={stereo_recommended:?}, stereo production-step={stereo_production_step:?}"
        );
        assert!(mono_recommended.max_frame_len_delta > 0);
        assert!(stereo_recommended.max_frame_len_delta > 0);
        assert!(mono_production_step.max_frame_len_delta <= mono_recommended.max_frame_len_delta);
        assert!(
            stereo_production_step.max_frame_len_delta <= stereo_recommended.max_frame_len_delta
        );
    }

    #[test]
    fn aac_standard_id_payload_breakdown_identifies_spectral_cost() {
        let mono = readiness_pcm(44_100, 1).unwrap();
        let stereo = super::aac_standard_surface_stereo_pcm(&mono).unwrap();
        let mono_details =
            super::aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
                &mono,
                sonare_codec::aac_lc_default_production_bitrate_bps(1).unwrap(),
                sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            )
            .unwrap();
        let stereo_details =
            super::aac_standard_selected_scale_factor_frame_details_with_candidates_and_bitrate(
                &stereo,
                sonare_codec::aac_lc_default_production_bitrate_bps(2).unwrap(),
                sonare_codec::AAC_STANDARD_ID_PCM_STEP_CANDIDATES,
            )
            .unwrap();

        let mono_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_selection(&mono, &mono_details)
                .unwrap();
        let stereo_breakdown =
            super::aac_standard_id_payload_breakdown_for_frame_selection(&stereo, &stereo_details)
                .unwrap();

        eprintln!(
            "AAC standard-id payload breakdown: mono={mono_breakdown:?}, stereo={stereo_breakdown:?}"
        );
        assert_eq!(mono_breakdown.frames, mono_details.len());
        assert_eq!(stereo_breakdown.frames, stereo_details.len());
        assert_eq!(mono_breakdown.channels, 1);
        assert_eq!(stereo_breakdown.channels, 2);
        assert!(mono_breakdown.sections > 0);
        assert!(stereo_breakdown.sections > mono_breakdown.sections);
        assert!(mono_breakdown.spectral_bits > mono_breakdown.scale_factor_bits);
        assert!(stereo_breakdown.spectral_bits > stereo_breakdown.scale_factor_bits);
        assert!(mono_breakdown.escape_spectral_bits > 0);
        assert!(stereo_breakdown.escape_spectral_bits > mono_breakdown.escape_spectral_bits);
        let mono_dominant = mono_breakdown
            .dominant_spectral_section
            .expect("mono dominant spectral section");
        let stereo_dominant = stereo_breakdown
            .dominant_spectral_section
            .expect("stereo dominant spectral section");
        let mono_dominant_escape = mono_breakdown
            .dominant_escape_section
            .expect("mono dominant escape section");
        let stereo_dominant_escape = stereo_breakdown
            .dominant_escape_section
            .expect("stereo dominant escape section");
        assert_ne!(mono_dominant.codebook_id, 0);
        assert_ne!(stereo_dominant.codebook_id, 0);
        assert_eq!(mono_dominant_escape.codebook_id, 11);
        assert_eq!(stereo_dominant_escape.codebook_id, 11);
        assert!(mono_dominant.spectral_bits > mono_breakdown.scale_factor_bits);
        assert!(stereo_dominant.spectral_bits > stereo_breakdown.scale_factor_bits);
        assert!(mono_dominant_escape.max_abs >= 13);
        assert!(stereo_dominant_escape.max_abs >= 13);
        assert!(mono_dominant
            .best_alternative_spectral_bits
            .is_some_and(|bit_len| bit_len >= mono_dominant.spectral_bits));
        assert!(stereo_dominant
            .best_alternative_spectral_bits
            .is_some_and(|bit_len| bit_len >= stereo_dominant.spectral_bits));
        assert!(mono_dominant_escape
            .best_alternative_spectral_bits
            .is_none());
        assert!(stereo_dominant_escape
            .best_alternative_spectral_bits
            .is_none());
        assert!(mono_breakdown.total_bits() > 0);
        assert!(stereo_breakdown.total_bits() > mono_breakdown.total_bits());
    }

    #[test]
    fn mp3_perceptual_reservoir_production_gap_is_release_gated() {
        let reservoir = LossyOraclePcmQuality {
            decoded_rms: 0.9290,
            best_correlation: 0.572,
        };
        let production = LossyOraclePcmQuality {
            decoded_rms: 0.9290,
            best_correlation: 0.572,
        };
        validate_mp3_perceptual_reservoir_production_correlation_gap(
            "MP3 perceptual reservoir stereo",
            reservoir,
            production,
        )
        .unwrap();

        let regressed = LossyOraclePcmQuality {
            decoded_rms: 0.8403,
            best_correlation: 0.450,
        };
        let err = validate_mp3_perceptual_reservoir_production_correlation_gap(
            "MP3 perceptual reservoir stereo",
            regressed,
            production,
        )
        .unwrap_err();
        assert!(err.contains("correlation gap to production exceeded diagnostic limit"));
    }

    #[test]
    fn mp3_perceptual_diagnostic_reports_candidate_profile() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let summary = super::mp3_perceptual_diagnostic_summary(
            &pcm,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
        )
        .unwrap();

        assert!(summary.contains("first_frame_candidate_profile=["));
        assert!(summary.contains("0.0005:2552b,0/42,max0"));
        assert!(summary.contains("first_nonzero_scale_factor_step=1"));
        assert!(summary.contains("1:43b,1/42,max2"));
    }

    #[test]
    fn aac_standard_candidate_tiebreak_prefers_expected_rms() {
        let selected = sonare_codec::AacPcmFrameStepSelection {
            step: 0.005,
            frame_len: 171,
            frame_capacity_bytes: 372,
        };
        let quiet = AacStandardDiagnosticCandidate {
            global_gain: 112,
            selected,
            encoded: Vec::new(),
            quality: LossyOraclePcmQuality {
                decoded_rms: 0.0107,
                best_correlation: 0.550,
            },
        };
        let matched = AacStandardDiagnosticCandidate {
            global_gain: 128,
            selected,
            encoded: Vec::new(),
            quality: LossyOraclePcmQuality {
                decoded_rms: 0.1709,
                best_correlation: 0.550,
            },
        };

        assert!(!aac_standard_candidate_is_at_least_as_good(
            &quiet, &matched, 0.1750
        ));
        assert!(aac_standard_candidate_is_at_least_as_good(
            &matched, &quiet, 0.1750
        ));
    }

    #[test]
    fn aac_selected_scale_factor_gain_sweep_prefers_rms_controlled_candidate() {
        let controlled = LossyOraclePcmQuality {
            decoded_rms: 0.2014,
            best_correlation: 0.548,
        };
        let over_amplified = LossyOraclePcmQuality {
            decoded_rms: 1.6111,
            best_correlation: 0.548,
        };

        assert!(!super::lossy_oracle_quality_is_at_least_as_good(
            &over_amplified,
            &controlled,
            0.1750
        ));
        assert!(super::lossy_oracle_quality_is_at_least_as_good(
            &controlled,
            &over_amplified,
            0.1750
        ));

        let stereo_controlled = LossyOraclePcmQuality {
            decoded_rms: 0.1030,
            best_correlation: 0.601,
        };
        let stereo_over_amplified = LossyOraclePcmQuality {
            decoded_rms: 1.6473,
            best_correlation: 0.601,
        };

        assert!(!super::lossy_oracle_quality_is_at_least_as_good(
            &stereo_over_amplified,
            &stereo_controlled,
            0.1468
        ));
        assert!(super::lossy_oracle_quality_is_at_least_as_good(
            &stereo_controlled,
            &stereo_over_amplified,
            0.1468
        ));
    }

    #[test]
    fn aac_selected_scale_factor_bias_sweep_keeps_fixed_like_candidates() {
        assert!(super::AAC_STANDARD_DIAGNOSTIC_GLOBAL_GAIN_CANDIDATES
            .contains(&super::AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_GLOBAL_GAIN));
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES
                .contains(&super::AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_GLOBAL_GAIN)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES.contains(&126)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_GLOBAL_GAIN_CANDIDATES.contains(&130)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_FIXED_SURFACE_GLOBAL_GAIN_CANDIDATES
                .contains(&super::AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_GLOBAL_GAIN)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES
                .contains(&super::AAC_STANDARD_DIAGNOSTIC_SELECTED_SURFACE_MAGNITUDE_BIAS)
        );
        assert!(
            super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES.contains(&12)
        );
        assert!(
            !super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES.contains(&0)
        );
        assert!(
            !super::AAC_STANDARD_HIGH_LEVEL_SELECTED_SURFACE_MAGNITUDE_BIAS_CANDIDATES
                .contains(&20)
        );

        let low_bias_mono = LossyOraclePcmQuality {
            decoded_rms: 0.1693,
            best_correlation: 0.548,
        };
        let fixed_like_mono = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.550,
        };
        assert!(super::lossy_oracle_quality_is_at_least_as_good(
            &fixed_like_mono,
            &low_bias_mono,
            0.1750
        ));
        assert!(!super::lossy_oracle_quality_is_at_least_as_good(
            &low_bias_mono,
            &fixed_like_mono,
            0.1750
        ));

        let low_bias_stereo = LossyOraclePcmQuality {
            decoded_rms: 0.2059,
            best_correlation: 0.602,
        };
        let fixed_like_stereo = LossyOraclePcmQuality {
            decoded_rms: 0.1743,
            best_correlation: 0.607,
        };
        assert!(super::lossy_oracle_quality_is_at_least_as_good(
            &fixed_like_stereo,
            &low_bias_stereo,
            0.1468
        ));
        assert!(!super::lossy_oracle_quality_is_at_least_as_good(
            &low_bias_stereo,
            &fixed_like_stereo,
            0.1468
        ));
    }

    #[test]
    fn lossy_oracle_quality_rejects_silent_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let err = validate_lossy_oracle_pcm_quality(&expected, &[0.0; 256]).unwrap_err();
        assert!(err.contains("effectively silent"));
    }

    #[test]
    fn lossy_oracle_quality_rejects_excessively_amplified_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let decoded = expected
            .iter()
            .map(|sample| sample * 64.0)
            .collect::<Vec<_>>();

        let err = validate_lossy_oracle_pcm_quality(&expected, &decoded).unwrap_err();
        assert!(err.contains("excessively amplified"));
    }

    #[test]
    fn lossy_oracle_quality_rejects_uncorrelated_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let decoded = (0..256)
            .map(|sample| ((sample as f32) * 0.31).cos() * 0.25)
            .collect::<Vec<_>>();

        let err = validate_lossy_oracle_pcm_quality(&expected, &decoded).unwrap_err();
        assert!(err.contains("does not correlate"));
    }

    #[test]
    fn diagnostic_quality_floor_rejects_known_regressions() {
        let passing = LossyOraclePcmQuality {
            decoded_rms: 0.1460,
            best_correlation: 0.384,
        };
        validate_diagnostic_quality_floor("MP3 diagnostic", passing, 0.10, 0.30).unwrap();

        let quiet = LossyOraclePcmQuality {
            decoded_rms: 0.0107,
            best_correlation: 0.550,
        };
        let err =
            validate_diagnostic_quality_floor("AAC diagnostic", quiet, 0.10, 0.50).unwrap_err();
        assert!(err.contains("decoded RMS regressed"));

        let decorrelated = LossyOraclePcmQuality {
            decoded_rms: 0.1709,
            best_correlation: 0.016,
        };
        let err = validate_diagnostic_quality_floor("MP3 diagnostic", decorrelated, 0.10, 0.30)
            .unwrap_err();
        assert!(err.contains("correlation regressed"));
    }

    #[test]
    fn adts_frame_budget_rejects_oversized_diagnostic_frame() {
        validate_adts_frame_budget("AAC diagnostic", 171, 372, 128_000).unwrap();

        let err = validate_adts_frame_budget("AAC diagnostic", 373, 372, 128_000).unwrap_err();
        assert!(err.contains("ADTS frame budget failed"));
    }

    #[test]
    fn aac_standard_id_mixed_workbench_is_publish_readiness_gated() {
        validate_aac_standard_id_mixed_workbench().unwrap();
    }

    #[test]
    fn correlation_search_handles_decoder_delay() {
        let expected = (0..128)
            .map(|sample| ((sample as f32) * 0.1).sin())
            .collect::<Vec<_>>();
        let mut decoded = vec![0.0; 64];
        decoded.extend_from_slice(&expected);

        assert!(best_normalized_correlation(&expected, &decoded).unwrap() > 0.99);
    }

    #[test]
    fn compatibility_lossy_scaffolds_are_not_publish_ready_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let samples = (0..2304)
            .map(|sample| ((sample as f32) * 0.01).sin() * 0.25)
            .collect::<Vec<_>>();
        let pcm = sonare_codec::AudioBuffer::new(44_100, 1, samples).unwrap();

        let diagnostics = compatibility_lossy_encode_diagnostics(&ffmpeg, &pcm).unwrap();

        assert_eq!(diagnostics.len(), 7);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .contains("MP3 compatibility scaffold passes current oracle")
                || diagnostic.contains("MP3 compatibility scaffold cannot be promoted")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .contains("AAC-LC compatibility scaffold passes current oracle")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("MP3 standard-table scaffold")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("MP3 perceptual-scale-factor scaffold")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("MP3 perceptual reservoir scaffold")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic
                .contains("AAC-LC experimental nonzero scaffold is still not production-gated")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("AAC-LC standard-table scaffold")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("best_correlation")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn diagnostic_lossy_readiness_passes_when_ffmpeg_is_available() {
        if std::env::var_os("SONARE_FFMPEG").is_none() {
            return;
        }

        verify_diagnostic_lossy_encode_readiness().unwrap();
    }

    #[test]
    fn mp3_stereo_production_artifact_passes_oracle_when_ffmpeg_is_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(32_000, 2).unwrap();
        let encoded = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let artifacts = [(
            "MP3 32kHz stereo",
            ProductionArtifactKind::Mp3,
            pcm,
            encoded,
        )];

        verify_production_lossy_oracle_acceptance(ffmpeg, &artifacts).unwrap();
    }

    #[test]
    fn mp3_stereo_perceptual_reservoir_candidate_catches_up_with_production_when_ffmpeg_is_available(
    ) {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 2).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-stereo-perceptual-reservoir-diagnostic-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let calibrated_details =
            sonare_codec::select_mpeg1_layer3_reservoir_frame_details_with_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let perceptual_details =
            sonare_codec::select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider(
                &pcm,
                super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let guarded_details = sonare_codec::select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        assert!(calibrated_details
            .iter()
            .all(|detail| { detail.perceptual_granules == 0 && detail.calibrated_granules == 4 }));
        assert!(perceptual_details
            .iter()
            .all(|detail| { detail.perceptual_granules == 4 && detail.calibrated_granules == 0 }));
        assert!(guarded_details
            .iter()
            .all(|detail| { detail.perceptual_granules + detail.calibrated_granules == 4 }));
        assert!(guarded_details
            .iter()
            .all(|detail| { detail.quality_guard_compared_granules == 4 }));
        assert!(
            guarded_details
                .iter()
                .all(|detail| detail.quality_guard_distortion_delta.is_finite()),
            "quality guard reported a non-finite encoder-side distortion delta"
        );
        let calibrated_max_payload = calibrated_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);
        let perceptual_max_payload = perceptual_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);
        let perceptual_min_step = perceptual_details
            .iter()
            .map(|detail| detail.step)
            .fold(f32::INFINITY, f32::min);
        let guarded_max_payload = guarded_details
            .iter()
            .map(|detail| detail.payload_bit_len)
            .max()
            .unwrap_or(0);

        let candidate_quality =
            super::mp3_perceptual_reservoir_nonzero_encode_diagnostic(&ffmpeg, &pcm, &out_dir)
                .unwrap();
        let guarded =
            sonare_codec::encode_mpeg1_layer3_pcm_frames_with_quality_guarded_perceptual_reservoir_and_table_provider(
                &pcm,
                super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let guarded_path = out_dir.join("mp3-stereo-guarded-perceptual-reservoir.mp3");
        std::fs::write(&guarded_path, guarded).unwrap();
        super::run_ffmpeg_acceptance(&ffmpeg, &guarded_path).unwrap();
        let guarded_decoded =
            super::run_ffmpeg_decode_f32le(&ffmpeg, &guarded_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let guarded_quality =
            super::validate_lossy_oracle_pcm_quality(&pcm.samples, &guarded_decoded).unwrap();
        let production = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let production_path = out_dir.join("mp3-stereo-production.mp3");
        std::fs::write(&production_path, production).unwrap();
        super::run_ffmpeg_acceptance(&ffmpeg, &production_path).unwrap();
        let production_decoded = super::run_ffmpeg_decode_f32le(
            &ffmpeg,
            &production_path,
            pcm.sample_rate,
            pcm.channels,
        )
        .unwrap();
        let production_quality =
            super::validate_lossy_oracle_pcm_quality(&pcm.samples, &production_decoded).unwrap();
        std::fs::remove_dir_all(&out_dir).unwrap();

        assert!(
            candidate_quality.best_correlation >= 0.49,
            "stereo perceptual reservoir should pass the tightened basic oracle before production re-evaluation: {candidate_quality:?}"
        );
        assert!(
            perceptual_details
                .iter()
                .any(|detail| detail.main_data_begin > 0),
            "stereo perceptual reservoir should exercise reservoir borrowing"
        );
        assert!(
            perceptual_max_payload <= calibrated_max_payload,
            "stereo perceptual reservoir is not being held back by payload size: perceptual={perceptual_max_payload}, calibrated={calibrated_max_payload}"
        );
        assert!(
            guarded_details
                .iter()
                .any(|detail| detail.main_data_begin > 0),
            "quality-guarded stereo perceptual reservoir should exercise reservoir borrowing"
        );
        assert!(
            guarded_max_payload <= calibrated_max_payload,
            "quality-guarded stereo perceptual reservoir should stay within the calibrated payload envelope: guarded={guarded_max_payload}, calibrated={calibrated_max_payload}"
        );
        assert!(
            perceptual_min_step <= 1.0,
            "stereo perceptual reservoir did not select an active fine step: min_step={perceptual_min_step}"
        );
        assert!(
            guarded_quality.best_correlation + 0.01 >= production_quality.best_correlation,
            "quality-guarded stereo perceptual reservoir regressed production quality: guarded={guarded_quality:?}, production={production_quality:?}"
        );
        assert!(
            candidate_quality.best_correlation + 0.001 >= production_quality.best_correlation,
            "stereo perceptual reservoir should now match the production bridge: candidate={candidate_quality:?}, production={production_quality:?}"
        );
        assert!(
            production_quality.best_correlation + 0.001 >= candidate_quality.best_correlation,
            "stereo perceptual reservoir unexpectedly exceeded the production bridge enough to require floor re-tuning: candidate={candidate_quality:?}, production={production_quality:?}"
        );
    }

    #[test]
    fn mp3_entropy_targeted_perceptual_reservoir_candidate_passes_ffmpeg_oracle_when_available() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            return;
        };
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-entropy-targeted-reservoir-quality-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        let baseline =
            sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider(
                &pcm,
                super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let candidate = sonare_codec::encode_mpeg1_layer3_pcm_frames_with_entropy_targeted_perceptual_reservoir_and_table_provider(
            &pcm,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();
        let candidate_details = sonare_codec::select_mpeg1_layer3_entropy_targeted_perceptual_reservoir_frame_details_with_table_provider(
            &pcm,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_STEP_CANDIDATES,
            128,
            false,
            0,
            sonare_codec::mpeg1_layer3_standard_table_provider(),
        )
        .unwrap();

        let entropy_target_bits = candidate_details
            .iter()
            .map(|detail| detail.entropy_target_bits)
            .sum::<usize>();
        let capacity_bits = candidate_details
            .iter()
            .map(|detail| detail.frame_capacity_bytes * 8)
            .sum::<usize>();
        assert_eq!(
            entropy_target_bits, capacity_bits,
            "entropy-targeted reservoir should distribute the full frame capacity"
        );
        assert!(
            candidate_details
                .iter()
                .any(|detail| detail.used_entropy_target_budget),
            "entropy-targeted reservoir did not exercise its entropy budget path"
        );

        let baseline_path = out_dir.join("mp3-perceptual-reservoir-baseline.mp3");
        std::fs::write(&baseline_path, baseline).unwrap();
        run_ffmpeg_acceptance(&ffmpeg, &baseline_path).unwrap();
        let baseline_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &baseline_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let baseline_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &baseline_decoded).unwrap();

        let candidate_path = out_dir.join("mp3-entropy-targeted-perceptual-reservoir.mp3");
        std::fs::write(&candidate_path, candidate).unwrap();
        run_ffmpeg_acceptance(&ffmpeg, &candidate_path).unwrap();
        let candidate_decoded =
            run_ffmpeg_decode_f32le(&ffmpeg, &candidate_path, pcm.sample_rate, pcm.channels)
                .unwrap();
        let candidate_quality =
            validate_lossy_oracle_pcm_quality(&pcm.samples, &candidate_decoded).unwrap();
        std::fs::remove_dir_all(&out_dir).unwrap();

        validate_diagnostic_quality_floor(
            "MP3 entropy-targeted perceptual reservoir diagnostic",
            candidate_quality,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_MIN_DECODED_RMS,
            super::MP3_PERCEPTUAL_DIAGNOSTIC_MIN_CORRELATION,
        )
        .unwrap();
        assert!(
            candidate_quality.best_correlation + 0.05 >= baseline_quality.best_correlation,
            "entropy-targeted reservoir regressed below perceptual reservoir baseline: candidate={candidate_quality:?}, baseline={baseline_quality:?}"
        );
        eprintln!(
            "MP3 entropy-targeted reservoir quality: candidate={candidate_quality:?}, baseline={baseline_quality:?}"
        );
    }

    #[test]
    fn mp3_production_artifacts_respect_default_frame_budget() {
        for (sample_rate, channels) in [(44_100, 1), (44_100, 2)] {
            let pcm = readiness_pcm(sample_rate, channels).unwrap();
            let encoded = sonare_codec::encode_with_mode(
                sonare_codec::Format::Mp3,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let label = if channels == 1 {
                "MP3 44.1kHz mono"
            } else {
                "MP3 44.1kHz stereo"
            };

            verify_mp3_default_production_budget(
                label,
                ProductionArtifactKind::Mp3,
                &pcm,
                &encoded,
            )
            .unwrap();
        }
    }

    #[test]
    fn mp3_production_artifacts_pass_focused_ffmpeg_quality_gate() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!("skipping MP3 production quality gate: set SONARE_FFMPEG=/path/to/ffmpeg");
            return;
        };
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-mp3-production-quality-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (sample_rate, channels) in [(44_100, 1), (44_100, 2)] {
            let pcm = readiness_pcm(sample_rate, channels).unwrap();
            let encoded = sonare_codec::encode_with_mode(
                sonare_codec::Format::Mp3,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let label = if channels == 1 {
                "MP3 44.1kHz mono"
            } else {
                "MP3 44.1kHz stereo"
            };

            verify_mp3_default_production_budget(
                label,
                ProductionArtifactKind::Mp3,
                &pcm,
                &encoded,
            )
            .unwrap();
            let artifact_path = out_dir.join(format!("mp3-production-quality-{}ch.mp3", channels));
            std::fs::write(&artifact_path, &encoded).unwrap();
            run_ffmpeg_clean_acceptance(&ffmpeg, &artifact_path).unwrap();
            let decoded =
                run_ffmpeg_decode_f32le(&ffmpeg, &artifact_path, sample_rate, channels).unwrap();
            let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
            let min_correlation =
                production_lossy_min_correlation(ProductionArtifactKind::Mp3, channels).unwrap();
            assert!(
                quality.best_correlation >= min_correlation,
                "{label} production quality regressed below floor {min_correlation}: {quality:?}"
            );
            eprintln!(
                "{label} production quality: decoded_rms={:.4}, best_correlation={:.3}",
                quality.decoded_rms, quality.best_correlation
            );
        }
    }

    #[test]
    fn mp3_default_frame_budget_rejects_truncated_frame() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let encoded = sonare_codec::encode_with_mode(
            sonare_codec::Format::Mp3,
            &pcm,
            sonare_codec::EncodeMode::ProductionOnly,
        )
        .unwrap();
        let err = verify_mp3_default_production_budget(
            "MP3 truncated",
            ProductionArtifactKind::Mp3,
            &pcm,
            &encoded[..encoded.len() - 1],
        )
        .unwrap_err();

        assert!(err.contains("extends past stream length"));
    }

    #[test]
    fn mp3_production_reservoir_check_rejects_self_contained_perceptual_stream() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let perceptual =
            sonare_codec::encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider(
                &pcm,
                sonare_codec::MPEG1_LAYER3_PCM_STEP_CANDIDATES,
                128,
                false,
                sonare_codec::mpeg1_layer3_standard_table_provider(),
            )
            .unwrap();
        let err = verify_mp3_production_reservoir("MP3 perceptual diagnostic", &pcm, &perceptual)
            .unwrap_err();

        assert!(
            err.contains("never used main_data_begin")
                || err.contains("does not match selector detail")
        );
    }

    #[test]
    fn aac_production_artifacts_respect_default_bitrate_budget() {
        for (sample_rate, channels) in [(44_100, 1), (44_100, 2)] {
            let pcm = readiness_pcm(sample_rate, channels).unwrap();
            let adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let label = if channels == 1 {
                "AAC-LC 44.1kHz mono"
            } else {
                "AAC-LC 44.1kHz stereo"
            };

            verify_aac_default_production_budget(label, ProductionArtifactKind::Aac, &pcm, &adts)
                .unwrap();

            let m4a = sonare_codec::mux_aac_adts_as_m4a(&adts).unwrap();
            verify_aac_default_production_budget(label, ProductionArtifactKind::M4a, &pcm, &m4a)
                .unwrap();
        }
    }

    #[test]
    fn aac_production_artifacts_pass_focused_ffmpeg_quality_gate() {
        let Some(ffmpeg) = std::env::var_os("SONARE_FFMPEG") else {
            eprintln!("skipping AAC production quality gate: set SONARE_FFMPEG=/path/to/ffmpeg");
            return;
        };
        let out_dir = std::env::temp_dir().join(format!(
            "sonare-codec-aac-production-quality-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&out_dir).unwrap();

        for (sample_rate, channels) in [(44_100, 1), (44_100, 2)] {
            let pcm = readiness_pcm(sample_rate, channels).unwrap();
            let adts = sonare_codec::encode_with_mode(
                sonare_codec::Format::Aac,
                &pcm,
                sonare_codec::EncodeMode::ProductionOnly,
            )
            .unwrap();
            let m4a = sonare_codec::mux_aac_adts_as_m4a(&adts).unwrap();
            let label = if channels == 1 {
                "AAC-LC 44.1kHz mono"
            } else {
                "AAC-LC 44.1kHz stereo"
            };

            for (kind, bytes, extension) in [
                (ProductionArtifactKind::Aac, adts.as_slice(), "aac"),
                (ProductionArtifactKind::M4a, m4a.as_slice(), "m4a"),
            ] {
                verify_aac_default_production_budget(label, kind, &pcm, bytes).unwrap();
                let artifact_path =
                    out_dir.join(format!("aac-production-quality-{}ch.{extension}", channels));
                std::fs::write(&artifact_path, bytes).unwrap();
                run_ffmpeg_clean_acceptance(&ffmpeg, &artifact_path).unwrap();
                let decoded =
                    run_ffmpeg_decode_f32le(&ffmpeg, &artifact_path, sample_rate, channels)
                        .unwrap();
                let quality = validate_lossy_oracle_pcm_quality(&pcm.samples, &decoded).unwrap();
                let min_correlation = production_lossy_min_correlation(kind, channels).unwrap();
                assert!(
                    quality.best_correlation >= min_correlation,
                    "{label} {kind:?} production quality regressed below floor {min_correlation}: {quality:?}"
                );
                eprintln!(
                    "{label} {kind:?} production quality: decoded_rms={:.4}, best_correlation={:.3}",
                    quality.decoded_rms, quality.best_correlation
                );
            }
        }
    }

    #[test]
    fn aac_default_bitrate_budget_rejects_malformed_adts() {
        let pcm = readiness_pcm(44_100, 1).unwrap();
        let err = verify_aac_default_production_budget(
            "AAC-LC malformed",
            ProductionArtifactKind::Aac,
            &pcm,
            &[0xff, 0xf1, 0x50],
        )
        .unwrap_err();

        assert!(err.contains("ADTS stream has no complete frames"));
    }
}
