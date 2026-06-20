use super::*;

pub(crate) fn run_publish_rust_packages() -> Result<(), String> {
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

pub(crate) fn verify_rust_package_file_list(
    package: &str,
    package_list: &str,
) -> Result<(), String> {
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

pub(crate) fn run_package_metadata_check() -> Result<(), String> {
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

pub(crate) fn run_git_head_check() -> Result<(), String> {
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
