use super::*;

pub(crate) fn run_deny(args: &[&str]) -> Result<(), String> {
    if let Ok(path) = env::var("SONARE_CARGO_DENY") {
        return run_command(path, args, ".");
    }

    let mut cargo_args = Vec::with_capacity(args.len() + 1);
    cargo_args.push("deny");
    cargo_args.extend_from_slice(args);
    run_command("cargo", &cargo_args, ".")
}

pub(crate) fn run_optional_nextest() -> Result<(), String> {
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

pub(crate) fn run_optional_machete() -> Result<(), String> {
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

pub(crate) fn run_optional_audit() -> Result<(), String> {
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

pub(crate) fn run_optional_semver_checks() -> Result<(), String> {
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

pub(crate) fn run_optional_miri() -> Result<(), String> {
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

pub(crate) fn run_optional_coverage() -> Result<(), String> {
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

pub(crate) fn skip_optional_qa_tool(
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

pub(crate) fn required_qa_tool(tool: &str) -> bool {
    env::var_os(REQUIRED_QA_TOOLS_ENV)
        .and_then(|value| value.into_string().ok())
        .is_some_and(|value| required_qa_tool_in_list(&value, tool))
}

pub(crate) fn required_qa_tool_in_list(value: &str, tool: &str) -> bool {
    value
        .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
        .filter(|item| !item.is_empty())
        .any(|item| item == "all" || item == tool)
}

pub(crate) fn cargo_subcommand_available(subcommand: &str) -> bool {
    Command::new("cargo")
        .args([subcommand, "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

pub(crate) fn cargo_toolchain_subcommand_available(toolchain: &str, subcommand: &str) -> bool {
    Command::new("cargo")
        .args([toolchain, subcommand, "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

pub(crate) fn git_head_available() -> Result<bool, String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .map_err(|err| format!("failed to run git rev-parse --verify HEAD: {err}"))?;
    Ok(output.status.success())
}

pub(crate) fn run_wasm_check() -> Result<(), String> {
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

pub(crate) fn wasm_target_installed() -> Result<bool, String> {
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

pub(crate) fn run_npm_pack_dry_run() -> Result<(), String> {
    let cache = env::var_os("npm_config_cache")
        .unwrap_or_else(|| OsString::from("/private/tmp/sonare-codec-npm-cache"));
    let mut command = Command::new("npm");
    command
        .args(["pack", "--dry-run", "--ignore-scripts"])
        .current_dir("bindings/wasm")
        .env("npm_config_cache", cache);
    run_prepared_command(&mut command, "npm pack --dry-run --ignore-scripts")
}
