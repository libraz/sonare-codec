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
    "encode_aac",
    "encode_aac_with_bitrate",
    "encode_m4a",
    "encode_m4a_with_bitrate",
    "demux_m4a_as_aac_adts",
    "aac_lc_adts_max_frame_len_for_bitrate",
    "aac_unsigned_pairs7_unit_magnitude_table",
    "aac_unsigned_pairs7_table",
    "aac_unsigned_pairs8_table",
    "aac_scale_factor_delta_table",
    "mp3_layer3_main_data_capacity_bytes",
    "mp3_layer3_main_data_capacity_bits",
];
const PYTHON_ONLY_BINDING_FUNCTIONS: &[&str] = &["encode_vorbis", "encode_opus"];

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("artifact-check") => artifact_check(),
        Some("gen-refs") => gen_refs(),
        Some("fuzz-smoke") => fuzz_smoke(),
        Some("name-check") => name_check(),
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
                "usage: cargo xtask <artifact-check|gen-refs|fuzz-smoke|name-check|oracle-smoke|package-preflight|publish-plan|publish-preflight|publish-readiness|qa-check|ref-check|release-check|size-report|tool-check>"
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
    for check in [
        run_package_metadata_check,
        verify_production_lossy_encode_readiness,
    ] {
        if let Err(err) = check() {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
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
        Check::PublishReadiness => verify_production_lossy_encode_readiness(),
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
    let aac_experimental =
        experimental_aac_lc_nonzero_encode_diagnostic(ffmpeg, expected_pcm, &out_dir);
    diagnostics.push(match aac_experimental {
        Ok(quality) => format!(
            "AAC-LC experimental nonzero scaffold is still not production-gated: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        ),
        Err(err) => format!("AAC-LC experimental nonzero scaffold is not publish-ready: {err}"),
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
        eprintln!(
            "{label} production oracle PCM quality: decoded_rms={:.4}, best_correlation={:.3}",
            quality.decoded_rms, quality.best_correlation
        );
    }

    fs::remove_dir_all(&out_dir)
        .map_err(|err| format!("failed to remove {}: {err}", out_dir.display()))
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
    "encode_aac",
    "encode_aac_with_bitrate",
    "encode_m4a",
    "encode_m4a_with_bitrate",
    "demux_m4a_as_aac_adts",
    "aac_lc_adts_max_frame_len_for_bitrate",
    "aac_unsigned_pairs7_unit_magnitude_table",
    "aac_unsigned_pairs7_table",
    "aac_unsigned_pairs8_table",
    "aac_scale_factor_delta_table",
    "mp3_layer3_main_data_capacity_bytes",
    "mp3_layer3_main_data_capacity_bits",
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

    if sonare_codec.aac_lc_adts_max_frame_len_for_bitrate(44100, 10000) != 30:
        sys.exit("Python wheel AAC bitrate budget helper returned an unexpected frame length")
    aac_10k = sonare_codec.encode_aac_with_bitrate(44100, 1, [0.0] * 2048, 10000)
    if not isinstance(aac_10k, bytes) or not aac_10k.startswith(b"\xff\xf1") or max_adts_frame_len(aac_10k) > 30:
        sys.exit("Python wheel AAC bitrate encode helper returned unexpected bytes")
    m4a_10k = sonare_codec.encode_m4a_with_bitrate(44100, 1, [0.0] * 2048, 10000)
    if not isinstance(m4a_10k, bytes) or b"ftyp" not in m4a_10k[:16]:
        sys.exit("Python wheel M4A bitrate encode helper returned unexpected bytes")
    if sonare_codec.demux_m4a_as_aac_adts(m4a_10k) != aac_10k:
        sys.exit("Python wheel M4A bitrate encode helper did not mux the expected ADTS")
    if sonare_codec.aac_unsigned_pairs7_unit_magnitude_table() != [0, 0, 0, 1, 0, 1, 5, 3, 1, 0, 4, 3, 1, 1, 12, 4]:
        sys.exit("Python wheel AAC codebook 7 helper returned unexpected entries")
    pairs7_table = sonare_codec.aac_unsigned_pairs7_table()
    if len(pairs7_table) != 256 or pairs7_table[:4] != [0, 0, 0, 1] or pairs7_table[36:40] != [1, 1, 12, 4] or pairs7_table[-4:] != [7, 7, 4095, 12]:
        sys.exit("Python wheel AAC full codebook 7 helper returned unexpected entries")
    pairs8_table = sonare_codec.aac_unsigned_pairs8_table()
    if len(pairs8_table) != 256 or pairs8_table[:4] != [0, 0, 14, 5] or pairs8_table[36:40] != [1, 1, 0, 3] or pairs8_table[-4:] != [7, 7, 1023, 10]:
        sys.exit("Python wheel AAC full codebook 8 helper returned unexpected entries")
    scale_factor_table = sonare_codec.aac_scale_factor_delta_table()
    if len(scale_factor_table) != 363 or scale_factor_table[:3] != [-60, 262120, 18] or scale_factor_table[180:183] != [0, 0, 1] or scale_factor_table[-3:] != [60, 524275, 19]:
        sys.exit("Python wheel AAC scale-factor delta helper returned unexpected entries")
    if sonare_codec.mp3_layer3_main_data_capacity_bytes(44100, 1, 128, False, False) != 396:
        sys.exit("Python wheel MP3 capacity byte helper returned an unexpected value")
    if sonare_codec.mp3_layer3_main_data_capacity_bits(44100, 1, 128, False, False) != 3168:
        sys.exit("Python wheel MP3 capacity bit helper returned an unexpected value")
    mp3_96k = sonare_codec.encode_mp3_with_bitrate(44100, 1, [0.0] * 1152, 96, False, False)
    if not isinstance(mp3_96k, bytes) or not mp3_96k.startswith(b"\xff\xfb") or len(mp3_96k) != 313:
        sys.exit("Python wheel MP3 bitrate encode helper returned unexpected bytes")

    silent = sonare_codec.encode_audio_production("mp3", 44100, 1, [0.0] * 1152)
    if not isinstance(silent, bytes) or not silent:
        sys.exit("Python wheel encode_audio_production did not return MP3 bytes")
    try:
        production_mp3 = sonare_codec.encode_audio_production("mp3", 44100, 1, [0.25] + [0.0] * 1151)
    except ValueError as exc:
        sys.exit("Python wheel encode_audio_production rejected non-silent MP3: " + str(exc))
    else:
        if not isinstance(production_mp3, bytes) or not production_mp3.startswith(b"\xff\xfb"):
            sys.exit("Python wheel encode_audio_production did not return non-silent MP3 bytes")
    try:
        production_mp3_stereo = sonare_codec.encode_audio_production("mp3", 44100, 2, [0.25, 0.0] + [0.0] * 2302)
    except ValueError as exc:
        sys.exit("Python wheel encode_audio_production rejected non-silent stereo MP3: " + str(exc))
    else:
        if not isinstance(production_mp3_stereo, bytes) or not production_mp3_stereo.startswith(b"\xff\xfb"):
            sys.exit("Python wheel encode_audio_production did not return non-silent stereo MP3 bytes")
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
        best_normalized_correlation, compatibility_lossy_encode_diagnostics,
        required_qa_tool_in_list, validate_lossy_oracle_pcm_quality,
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
    fn lossy_oracle_quality_rejects_silent_pcm() {
        let expected = (0..256)
            .map(|sample| ((sample as f32) * 0.05).sin() * 0.25)
            .collect::<Vec<_>>();
        let err = validate_lossy_oracle_pcm_quality(&expected, &[0.0; 256]).unwrap_err();
        assert!(err.contains("effectively silent"));
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

        assert_eq!(diagnostics.len(), 4);
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
            diagnostics.iter().any(|diagnostic| diagnostic
                .contains("AAC-LC experimental nonzero scaffold is still not production-gated")),
            "{diagnostics:?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("best_correlation")),
            "{diagnostics:?}"
        );
    }
}
