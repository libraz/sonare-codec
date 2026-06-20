use super::*;

pub(crate) fn collect_size_report() -> Result<Vec<SizeEntry>, String> {
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

pub(crate) fn size_entry(kind: &'static str, path: PathBuf) -> Result<SizeEntry, String> {
    let bytes = match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => Some(metadata.len()),
        Ok(_) => None,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("failed to inspect {}: {err}", path.display())),
    };
    Ok(SizeEntry { kind, path, bytes })
}

pub(crate) fn size_entries_from_dir(
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

pub(crate) fn human_bytes(bytes: u64) -> String {
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

pub(crate) fn run_command<I, S>(program: I, args: &[S], cwd: impl AsRef<Path>) -> Result<(), String>
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

pub(crate) fn run_prepared_command(command: &mut Command, label: &str) -> Result<(), String> {
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

pub(crate) fn run_command_output<I, S>(
    program: I,
    args: &[S],
    cwd: impl AsRef<Path>,
) -> Result<String, String>
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

pub(crate) fn command_label<S>(program: &std::ffi::OsStr, args: &[S]) -> String
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

pub(crate) fn toml_string_value<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    input.lines().find_map(|line| {
        let (line_key, value) = line.split_once('=')?;
        if line_key.trim() != key {
            return None;
        }
        quoted_value(value.trim())
    })
}

pub(crate) fn json_string_value<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    let quoted_key = format!("\"{key}\"");
    input.lines().find_map(|line| {
        let (line_key, value) = line.split_once(':')?;
        if line_key.trim() != quoted_key {
            return None;
        }
        quoted_value(value.trim().trim_end_matches(','))
    })
}

pub(crate) fn quoted_value(input: &str) -> Option<&str> {
    input
        .strip_prefix('"')?
        .split_once('"')
        .map(|(value, _)| value)
}

pub(crate) fn assert_contains(input: &str, needle: &str, label: &str) -> Result<(), String> {
    if input.contains(needle) {
        Ok(())
    } else {
        Err(format!("{label} is missing expected entry {needle}"))
    }
}
