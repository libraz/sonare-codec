use super::*;

pub(crate) fn fuzz_smoke() -> ExitCode {
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

pub(crate) fn oracle_smoke() -> ExitCode {
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

pub(crate) fn generate_oracle_smoke_artifacts(
    out_dir: &Path,
) -> Result<Vec<std::path::PathBuf>, String> {
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

pub(crate) fn build_reference_manifest(
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

pub(crate) fn verify_refs() -> Result<(), String> {
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

pub(crate) fn compare_refs(
    ref_dir: &Path,
    manifest: &str,
    generated: &[PathBuf],
) -> Result<(), String> {
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

pub(crate) fn verify_manifest_artifact(
    manifest: &str,
    name: &str,
    bytes: &[u8],
) -> Result<(), String> {
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

pub(crate) fn manifest_artifact_block<'a>(
    manifest: &'a str,
    name: &str,
) -> Result<&'a str, String> {
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

pub(crate) fn run_ffmpeg_acceptance(ffmpeg: &OsStr, artifact: &Path) -> Result<(), String> {
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

pub(crate) fn run_ffmpeg_clean_acceptance(ffmpeg: &OsStr, artifact: &Path) -> Result<(), String> {
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

pub(crate) fn run_ffmpeg_decode_f32le(
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
pub(crate) struct LossyOraclePcmQuality {
    pub(crate) decoded_rms: f64,
    pub(crate) best_correlation: f64,
}

pub(crate) fn validate_lossy_oracle_pcm_quality(
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

pub(crate) fn rms(samples: &[f32]) -> f64 {
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

pub(crate) fn best_normalized_correlation(
    expected: &[f32],
    decoded: &[f32],
) -> Result<f64, String> {
    Ok(best_normalized_correlation_with_offset(expected, decoded)?.0)
}

pub(crate) fn best_normalized_correlation_with_offset(
    expected: &[f32],
    decoded: &[f32],
) -> Result<(f64, usize), String> {
    let window_len = expected.len().min(decoded.len());
    if window_len < 64 {
        return Err("not enough decoded PCM to validate correlation".to_owned());
    }

    let expected_window = &expected[..window_len];
    let mut best = -1.0_f64;
    let mut best_offset = 0_usize;
    for offset in 0..=decoded.len() - window_len {
        let correlation =
            normalized_correlation(expected_window, &decoded[offset..offset + window_len]);
        if correlation > best {
            best = correlation;
            best_offset = offset;
        }
    }
    Ok((best, best_offset))
}

pub(crate) fn normalized_correlation(left: &[f32], right: &[f32]) -> f64 {
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

pub(crate) fn format_name(format: sonare_codec::Format) -> &'static str {
    match format {
        sonare_codec::Format::Wav => "wav",
        sonare_codec::Format::Flac => "flac",
        sonare_codec::Format::Mp3 => "mp3",
        sonare_codec::Format::Vorbis => "vorbis",
        sonare_codec::Format::Opus => "opus",
        sonare_codec::Format::Aac => "aac",
    }
}

pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub(crate) fn json_escape(input: &str) -> String {
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

pub(crate) fn decode_hex(input: &str) -> Vec<u8> {
    let hex = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    assert_eq!(hex.len() % 2, 0);

    hex.chunks_exact(2)
        .map(|chunk| (hex_digit(chunk[0]) << 4) | hex_digit(chunk[1]))
        .collect()
}

pub(crate) fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => 0,
    }
}
