use super::*;

pub(crate) enum Check<'a> {
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
pub(crate) enum ToolCommand<'a> {
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
pub(crate) struct ToolCheck<'a> {
    pub(crate) label: &'a str,
    pub(crate) command: ToolCommand<'a>,
    pub(crate) required: bool,
}

pub(crate) enum ToolStatus {
    Present(String),
    Missing(String),
}

impl<'a> ToolCheck<'a> {
    pub(crate) fn command(label: &'a str, args: &'a [&'a str], required: bool) -> Self {
        Self {
            label,
            command: ToolCommand::Command {
                program: label,
                args,
            },
            required,
        }
    }

    pub(crate) fn env_command(
        label: &'a str,
        env_var: &'a str,
        args: &'a [&'a str],
        required: bool,
    ) -> Self {
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

    pub(crate) fn cargo_subcommand(
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

    pub(crate) fn cargo_subcommand_with_env(
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

    pub(crate) fn cargo_toolchain_subcommand(
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

    pub(crate) fn python_module(module: &'a str, required: bool) -> Self {
        Self {
            label: module,
            command: ToolCommand::PythonModule { module },
            required,
        }
    }

    pub(crate) fn run(self) -> ToolStatus {
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

pub(crate) fn run_check(check: Check<'_>) -> Result<(), String> {
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

pub(crate) fn run_registry_name_check_if_requested() -> Result<(), String> {
    if env::var_os("SONARE_CHECK_REGISTRY_NAMES").is_none() {
        eprintln!(
            "skipping registry name check: set SONARE_CHECK_REGISTRY_NAMES=1 before first publish"
        );
        return Ok(());
    }

    run_registry_name_check()
}

pub(crate) fn verify_production_lossy_encode_readiness() -> Result<(), String> {
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

pub(crate) fn verify_diagnostic_lossy_encode_readiness() -> Result<(), String> {
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
        let (aac_balanced_mono_quality, aac_balanced_mono_payload_breakdown) =
            validate_aac_standard_id_balanced_surface(AacStandardIdBalancedSurfaceCheck {
                ffmpeg: &ffmpeg,
                label: "AAC-LC standard-id balanced mono",
                expected_pcm: &pcm,
                bitrate: sonare_codec::aac_lc_default_production_bitrate_bps(1)
                    .map_err(|err| format!("AAC mono production bitrate lookup failed: {err}"))?,
                baseline_quality: aac_standard_surface_mono_quality,
                min_correlation: 0.45,
                out_dir: &out_dir,
                file_stem: "aaclc-standard-id-balanced-mono",
            })?;
        let aac_standard_stereo_pcm = aac_standard_surface_stereo_pcm(&pcm)?;
        let (aac_balanced_stereo_quality, aac_balanced_stereo_payload_breakdown) =
            validate_aac_standard_id_balanced_surface(AacStandardIdBalancedSurfaceCheck {
                ffmpeg: &ffmpeg,
                label: "AAC-LC standard-id balanced stereo",
                expected_pcm: &aac_standard_stereo_pcm,
                bitrate: sonare_codec::aac_lc_default_production_bitrate_bps(2)
                    .map_err(|err| format!("AAC stereo production bitrate lookup failed: {err}"))?,
                baseline_quality: aac_standard_surface_stereo_quality,
                min_correlation: 0.50,
                out_dir: &out_dir,
                file_stem: "aaclc-standard-id-balanced-stereo",
            })?;
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
        validate_aac_standard_id_production_correlation_gap(
            "AAC balanced standard-id mono",
            aac_balanced_mono_quality,
            aac_production_mono_quality,
        )?;
        validate_aac_standard_id_production_correlation_gap(
            "AAC balanced standard-id stereo",
            aac_balanced_stereo_quality,
            aac_production_stereo_quality,
        )?;
        validate_aac_standard_id_rms_control_advantage(
            "AAC standard-id mono",
            aac_standard_surface_mono_quality,
            aac_production_mono_quality,
            rms(&pcm.samples),
        )?;
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
            aac_balanced_mono_quality,
            aac_balanced_stereo_quality,
            aac_production_mono_quality,
            aac_production_stereo_quality,
            aac_standard_mono_frame_budget,
            aac_standard_stereo_frame_budget,
            aac_standard_mono_production_step_frame_budget,
            aac_standard_stereo_production_step_frame_budget,
            aac_standard_mono_payload_breakdown,
            aac_standard_stereo_payload_breakdown,
            aac_balanced_mono_payload_breakdown,
            aac_balanced_stereo_payload_breakdown,
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
                "diagnostic lossy encode readiness: MP3 decoded_rms={:.4}, MP3 best_correlation={:.3}, MP3 reservoir decoded_rms={:.4}, MP3 reservoir best_correlation={:.3}, MP3 stereo reservoir decoded_rms={:.4}, MP3 stereo reservoir best_correlation={:.3}, MP3 production mono best_correlation={:.3}, MP3 reservoir mono correlation_gap={:.3}, MP3 production stereo best_correlation={:.3}, MP3 reservoir stereo correlation_gap={:.3}, AAC decoded_rms={:.4}, AAC best_correlation={:.3}, AAC standard-id mono decoded_rms={:.4}, AAC standard-id mono rms_error={:.4}, AAC standard-id mono best_correlation={:.3}, AAC balanced mono decoded_rms={:.4}, AAC balanced mono best_correlation={:.3}, AAC balanced mono correlation_gap={:.3}, AAC balanced mono escape_spectral_bits={}, AAC balanced mono max_abs={}, AAC production mono decoded_rms={:.4}, AAC production mono rms_error={:.4}, AAC production mono best_correlation={:.3}, AAC standard-id mono correlation_gap={:.3}, AAC standard-id mono frame_len_delta={}, AAC standard-id mono min_slack_delta={}, AAC standard-id mono step_delta={:.6}, AAC standard-id mono production-step frame_len_delta={}, AAC standard-id mono production-step min_slack_delta={}, AAC standard-id mono production-step step_delta={:.6}, AAC standard-id mono section_bits={}, AAC standard-id mono scale_factor_bits={}, AAC standard-id mono spectral_bits={}, AAC standard-id mono total_bits={}, AAC standard-id mono escape_sections={}, AAC standard-id mono escape_spectral_bits={}, AAC standard-id mono max_abs={}, AAC standard-id mono dominant_section={:?}, AAC standard-id mono dominant_escape_section={:?}, AAC standard-id stereo decoded_rms={:.4}, AAC standard-id stereo rms_error={:.4}, AAC standard-id stereo best_correlation={:.3}, AAC balanced stereo decoded_rms={:.4}, AAC balanced stereo best_correlation={:.3}, AAC balanced stereo correlation_gap={:.3}, AAC balanced stereo escape_spectral_bits={}, AAC balanced stereo max_abs={}, AAC production stereo decoded_rms={:.4}, AAC production stereo rms_error={:.4}, AAC production stereo best_correlation={:.3}, AAC standard-id stereo correlation_gap={:.3}, AAC standard-id stereo frame_len_delta={}, AAC standard-id stereo min_slack_delta={}, AAC standard-id stereo step_delta={:.6}, AAC standard-id stereo production-step frame_len_delta={}, AAC standard-id stereo production-step min_slack_delta={}, AAC standard-id stereo production-step step_delta={:.6}, AAC standard-id stereo section_bits={}, AAC standard-id stereo scale_factor_bits={}, AAC standard-id stereo spectral_bits={}, AAC standard-id stereo total_bits={}, AAC standard-id stereo escape_sections={}, AAC standard-id stereo escape_spectral_bits={}, AAC standard-id stereo max_abs={}, AAC standard-id stereo dominant_section={:?}, AAC standard-id stereo dominant_escape_section={:?}",
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
                summary.aac_balanced_mono_quality.decoded_rms,
                summary.aac_balanced_mono_quality.best_correlation,
                summary.aac_production_mono_quality.best_correlation
                    - summary.aac_balanced_mono_quality.best_correlation,
                summary.aac_balanced_mono_payload_breakdown.escape_spectral_bits,
                summary.aac_balanced_mono_payload_breakdown.max_abs,
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
                summary.aac_balanced_stereo_quality.decoded_rms,
                summary.aac_balanced_stereo_quality.best_correlation,
                summary.aac_production_stereo_quality.best_correlation
                    - summary.aac_balanced_stereo_quality.best_correlation,
                summary
                    .aac_balanced_stereo_payload_breakdown
                    .escape_spectral_bits,
                summary.aac_balanced_stereo_payload_breakdown.max_abs,
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
pub(crate) struct DiagnosticLossyQualitySummary {
    pub(crate) mp3_quality: LossyOraclePcmQuality,
    pub(crate) mp3_reservoir_quality: LossyOraclePcmQuality,
    pub(crate) mp3_stereo_reservoir_quality: LossyOraclePcmQuality,
    pub(crate) mp3_production_mono_quality: LossyOraclePcmQuality,
    pub(crate) mp3_production_stereo_quality: LossyOraclePcmQuality,
    pub(crate) aac_quality: LossyOraclePcmQuality,
    pub(crate) aac_standard_surface_mono_quality: LossyOraclePcmQuality,
    pub(crate) aac_standard_surface_stereo_quality: LossyOraclePcmQuality,
    pub(crate) aac_balanced_mono_quality: LossyOraclePcmQuality,
    pub(crate) aac_balanced_stereo_quality: LossyOraclePcmQuality,
    pub(crate) aac_production_mono_quality: LossyOraclePcmQuality,
    pub(crate) aac_production_stereo_quality: LossyOraclePcmQuality,
    pub(crate) aac_standard_mono_frame_budget: AacFrameSelectionComparison,
    pub(crate) aac_standard_stereo_frame_budget: AacFrameSelectionComparison,
    pub(crate) aac_standard_mono_production_step_frame_budget: AacFrameSelectionComparison,
    pub(crate) aac_standard_stereo_production_step_frame_budget: AacFrameSelectionComparison,
    pub(crate) aac_standard_mono_payload_breakdown: AacStandardIdPayloadBreakdown,
    pub(crate) aac_standard_stereo_payload_breakdown: AacStandardIdPayloadBreakdown,
    pub(crate) aac_balanced_mono_payload_breakdown: AacStandardIdPayloadBreakdown,
    pub(crate) aac_balanced_stereo_payload_breakdown: AacStandardIdPayloadBreakdown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AacFrameSelectionComparison {
    pub(crate) frames: usize,
    pub(crate) production_max_frame_len: usize,
    pub(crate) standard_id_max_frame_len: usize,
    pub(crate) max_frame_len_delta: isize,
    pub(crate) production_min_budget_slack: usize,
    pub(crate) standard_id_min_budget_slack: usize,
    pub(crate) min_budget_slack_delta: isize,
    pub(crate) production_max_step: f32,
    pub(crate) standard_id_max_step: f32,
    pub(crate) max_step_delta: f32,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AacScaleFactorProfile {
    pub(crate) frames: usize,
    pub(crate) channels: usize,
    pub(crate) bands: usize,
    pub(crate) raised_bands: usize,
    pub(crate) max_delta: i16,
    pub(crate) mean_delta: f64,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AacScaleFactorPressureRecoveryCandidate {
    pub(crate) restored_bias: i16,
    pub(crate) restored_bands_per_channel: usize,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AacScaleFactorPressureRecovery {
    pub(crate) candidate: AacScaleFactorPressureRecoveryCandidate,
    pub(crate) profile: AacScaleFactorProfile,
    pub(crate) quality: LossyOraclePcmQuality,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AacQuantizerStepSweepResult {
    pub(crate) step_scale: f32,
    pub(crate) max_quantized_abs: i32,
    pub(crate) max_frame_len: usize,
    pub(crate) profile: AacScaleFactorProfile,
    pub(crate) quality: LossyOraclePcmQuality,
}

pub(crate) type AacStandardIdPayloadBreakdown = sonare_codec::AacStandardIdPayloadBreakdown;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ProductionArtifactKind {
    Mp3,
    Aac,
    M4a,
}

impl ProductionArtifactKind {
    pub(crate) fn from_format(format: sonare_codec::Format) -> Result<Self, String> {
        match format {
            sonare_codec::Format::Mp3 => Ok(Self::Mp3),
            sonare_codec::Format::Aac => Ok(Self::Aac),
            _ => Err(format!(
                "unexpected production lossy format for oracle: {format:?}"
            )),
        }
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "aac",
            Self::M4a => "m4a",
        }
    }
}
