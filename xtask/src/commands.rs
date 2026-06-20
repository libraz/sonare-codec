use super::*;

pub(crate) fn artifact_check() -> ExitCode {
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

pub(crate) fn release_check() -> ExitCode {
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

pub(crate) fn name_check() -> ExitCode {
    match run_registry_name_check() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn run_registry_name_check() -> Result<(), String> {
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

pub(crate) fn package_preflight() -> ExitCode {
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

pub(crate) fn publish_preflight() -> ExitCode {
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

pub(crate) fn qa_check() -> ExitCode {
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

pub(crate) fn publish_readiness() -> ExitCode {
    match run_publish_readiness_check() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn run_publish_readiness_check() -> Result<(), String> {
    for check in [
        run_package_metadata_check,
        verify_production_lossy_encode_readiness,
        verify_diagnostic_lossy_encode_readiness,
    ] {
        check()?;
    }
    Ok(())
}

pub(crate) fn aac_standard_diagnostic() -> ExitCode {
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

pub(crate) fn mp3_perceptual_diagnostic() -> ExitCode {
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

pub(crate) fn publish_plan() -> ExitCode {
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

pub(crate) fn gen_refs() -> ExitCode {
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

pub(crate) fn ref_check() -> ExitCode {
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

pub(crate) fn size_report() -> ExitCode {
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

pub(crate) fn tool_check() -> ExitCode {
    match run_tool_readiness_check() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}

pub(crate) fn run_tool_readiness_check() -> Result<(), ()> {
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

pub(crate) fn check_registry_name(label: &str, url: &str) -> Result<(), String> {
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

pub(crate) fn http_status(url: &str) -> Result<u16, String> {
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
