use super::*;

pub(crate) fn run_wasm_pack_build() -> Result<(), String> {
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

pub(crate) fn run_wasm_pack_output_check() -> Result<(), String> {
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

pub(crate) fn run_maturin_build() -> Result<(), String> {
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

pub(crate) fn run_python_wheel_output_check() -> Result<(), String> {
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
        or sonare_codec.aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(1) != 2047
        or sonare_codec.aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(2) != 1535
    ):
        sys.exit("Python wheel AAC standard-id selected-scale-factor recommended parameters returned unexpected values")
    if (
        sonare_codec.aac_standard_id_selected_scale_factor_parameters(1) != [128.0, 16.0]
        or sonare_codec.aac_standard_id_selected_scale_factor_parameters(2) != [126.0, 16.0]
    ):
        sys.exit("Python wheel AAC standard-id selected-scale-factor parameter helper returned unexpected values")
    if (
        sonare_codec.aac_standard_id_selected_scale_factor_balanced_parameters(1) != [136.0, 8.0, 2047.0]
        or sonare_codec.aac_standard_id_selected_scale_factor_balanced_parameters(2) != [138.0, 4.0, 1535.0]
    ):
        sys.exit("Python wheel AAC balanced standard-id selected-scale-factor parameter helper returned unexpected values")
    if (
        sonare_codec.aac_standard_id_selected_scale_factor_balanced_gain_deltas(1) != [0.0, 2.0, 4.0, 6.0, 8.0]
        or sonare_codec.aac_standard_id_selected_scale_factor_balanced_gain_deltas(2) != [8.0, 12.0, 16.0]
        or sonare_codec.aac_standard_id_selected_scale_factor_balanced_magnitude_biases(1) != [8.0, 12.0, 16.0, 20.0]
        or sonare_codec.aac_standard_id_selected_scale_factor_balanced_magnitude_biases(2) != [4.0, 8.0, 12.0]
    ):
        sys.exit("Python wheel AAC balanced standard-id selected-scale-factor profile helper returned unexpected values")
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
    standard_selected_max_abs_adts = sonare_codec.encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16, 2047)
    if not isinstance(standard_selected_max_abs_adts, bytes) or not standard_selected_max_abs_adts.startswith(b"\xff\xf1") or max_adts_frame_len(standard_selected_max_abs_adts) > 372:
        sys.exit("Python wheel AAC standard selected max-abs helper returned unexpected ADTS")
    recommended_standard_selected_max_abs_adts = sonare_codec.encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 2047)
    if recommended_standard_selected_max_abs_adts != standard_selected_max_abs_adts:
        sys.exit("Python wheel AAC recommended standard selected max-abs helper did not match explicit mono parameters")
    balanced_standard_selected_adts = sonare_codec.encode_aac_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, [0.0] * 2048, 128000)
    expected_balanced_standard_selected_adts = sonare_codec.encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 136, 8, 2047)
    if balanced_standard_selected_adts != expected_balanced_standard_selected_adts:
        sys.exit("Python wheel AAC balanced standard selected helper did not match balanced mono parameters")
    standard_selected_generic_m4a = sonare_codec.encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16)
    if not isinstance(standard_selected_generic_m4a, bytes) or standard_selected_generic_m4a[4:8] != b"ftyp":
        sys.exit("Python wheel M4A standard selected spectral-offset bitrate helper returned unexpected container")
    recommended_standard_selected_generic_m4a = sonare_codec.encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, [0.0] * 2048, 128000)
    if recommended_standard_selected_generic_m4a != standard_selected_generic_m4a:
        sys.exit("Python wheel M4A recommended standard selected helper did not match explicit mono parameters")
    standard_selected_max_abs_m4a = sonare_codec.encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16, 2047)
    if not isinstance(standard_selected_max_abs_m4a, bytes) or standard_selected_max_abs_m4a[4:8] != b"ftyp":
        sys.exit("Python wheel M4A standard selected max-abs helper returned unexpected container")
    recommended_standard_selected_max_abs_m4a = sonare_codec.encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 2047)
    if recommended_standard_selected_max_abs_m4a != standard_selected_max_abs_m4a:
        sys.exit("Python wheel M4A recommended standard selected max-abs helper did not match explicit mono parameters")
    balanced_standard_selected_m4a = sonare_codec.encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, [0.0] * 2048, 128000)
    if not isinstance(balanced_standard_selected_m4a, bytes) or balanced_standard_selected_m4a[4:8] != b"ftyp":
        sys.exit("Python wheel M4A balanced standard selected helper returned unexpected container")
    if sonare_codec.demux_m4a_as_aac_adts(standard_selected_max_abs_m4a) != standard_selected_max_abs_adts:
        sys.exit("Python wheel M4A standard selected max-abs helper did not mux the expected ADTS")
    if sonare_codec.demux_m4a_as_aac_adts(balanced_standard_selected_m4a) != balanced_standard_selected_adts:
        sys.exit("Python wheel M4A balanced standard selected helper did not mux the expected ADTS")
    standard_selected_details = sonare_codec.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16)
    if len(standard_selected_details) != 8 or standard_selected_details[0] != 0 or standard_selected_details[4] != 1 or standard_selected_details[2] > 372 or standard_selected_details[6] > 372:
        sys.exit(f"Python wheel AAC standard selected bitrate details returned {standard_selected_details}")
    recommended_standard_selected_details = sonare_codec.aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if recommended_standard_selected_details != standard_selected_details:
        sys.exit(f"Python wheel AAC recommended standard selected bitrate details returned {recommended_standard_selected_details}")
    standard_selected_profile = sonare_codec.aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16)
    if standard_selected_profile != [2.0, 1.0, 98.0, 0.0, 0.0, 0.0]:
        sys.exit(f"Python wheel AAC standard selected profile returned {standard_selected_profile}")
    recommended_standard_selected_profile = sonare_codec.aac_recommended_standard_selected_scale_factor_profile_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if recommended_standard_selected_profile != standard_selected_profile:
        sys.exit(f"Python wheel AAC recommended standard selected profile returned {recommended_standard_selected_profile}")
    balanced_standard_selected_profile = sonare_codec.aac_balanced_standard_selected_scale_factor_profile_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if balanced_standard_selected_profile != standard_selected_profile:
        sys.exit(f"Python wheel AAC balanced standard selected profile returned {balanced_standard_selected_profile}")
    standard_payload_breakdown = sonare_codec.aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16)
    if len(standard_payload_breakdown) != 11 or standard_payload_breakdown[0] != 2.0 or standard_payload_breakdown[1] != 1.0 or standard_payload_breakdown[3] != 0.0 or standard_payload_breakdown[4] != 0.0 or standard_payload_breakdown[8] != 0.0 or standard_payload_breakdown[10] != 0.0:
        sys.exit(f"Python wheel AAC standard-id payload breakdown returned {standard_payload_breakdown}")
    recommended_payload_breakdown = sonare_codec.aac_recommended_standard_id_payload_breakdown_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if recommended_payload_breakdown != standard_payload_breakdown:
        sys.exit(f"Python wheel AAC recommended standard-id payload breakdown returned {recommended_payload_breakdown}")
    balanced_payload_breakdown = sonare_codec.aac_balanced_standard_id_payload_breakdown_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if balanced_payload_breakdown != standard_payload_breakdown:
        sys.exit(f"Python wheel AAC balanced standard-id payload breakdown returned {balanced_payload_breakdown}")
    explicit_balanced_quality_profile = sonare_codec.aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 136, 8, 2047)
    balanced_quality_profile = sonare_codec.aac_balanced_standard_id_quality_control_profile_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if (
        len(balanced_quality_profile) != 16
        or balanced_quality_profile != explicit_balanced_quality_profile
        or balanced_quality_profile[0] != 2.0
        or balanced_quality_profile[1] != 1.0
        or balanced_quality_profile[3] < 0.0
        or balanced_quality_profile[4] != 2047.0
        or balanced_quality_profile[5] != 0.0
        or balanced_quality_profile[10] != 0.0
        or balanced_quality_profile[13] != 0.0
    ):
        sys.exit(f"Python wheel AAC balanced quality-control profile returned {balanced_quality_profile}")
    balanced_quality_candidates = sonare_codec.aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    if (
        not balanced_quality_candidates
        or len(balanced_quality_candidates) % 19 != 0
        or not any(value == 136.0 for index, value in enumerate(balanced_quality_candidates) if index % 19 == 0)
        or not any(value == 8.0 for index, value in enumerate(balanced_quality_candidates) if index % 19 == 1)
        or not any(value == 2047.0 for index, value in enumerate(balanced_quality_candidates) if index % 19 == 2)
    ):
        sys.exit(f"Python wheel AAC balanced quality-control candidates returned {balanced_quality_candidates}")
    standard_selected_max_abs_details = sonare_codec.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 128, 16, 2047)
    if len(standard_selected_max_abs_details) != 8 or standard_selected_max_abs_details[0] != 0 or standard_selected_max_abs_details[4] != 1 or standard_selected_max_abs_details[2] > 372 or standard_selected_max_abs_details[6] > 372:
        sys.exit(f"Python wheel AAC standard selected max-abs bitrate details returned {standard_selected_max_abs_details}")
    recommended_standard_selected_max_abs_details = sonare_codec.aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 2047)
    if recommended_standard_selected_max_abs_details != standard_selected_max_abs_details:
        sys.exit(f"Python wheel AAC recommended standard selected max-abs bitrate details returned {recommended_standard_selected_max_abs_details}")
    balanced_standard_selected_details = sonare_codec.aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(44100, 1, [0.0] * 2048, 128000)
    expected_balanced_standard_selected_details = sonare_codec.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(44100, 1, [0.0] * 2048, 128000, 136, 8, 2047)
    if balanced_standard_selected_details != expected_balanced_standard_selected_details:
        sys.exit(f"Python wheel AAC balanced standard selected details returned {balanced_standard_selected_details}")
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
    mp3_mono_production_steps = sonare_codec.mp3_production_pcm_step_candidates(1)
    mp3_stereo_production_steps = sonare_codec.mp3_production_pcm_step_candidates(2)
    if (
        mp3_mono_production_steps[0] != 2.0
        or has_approx(mp3_mono_production_steps, 0.2)
        or mp3_stereo_production_steps != mp3_steps
    ):
        sys.exit(
            f"Python wheel MP3 production step candidates returned {mp3_mono_production_steps=} {mp3_stereo_production_steps=}"
        )
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
    mp3_band_biased = sonare_codec.encode_mp3_perceptual_scale_factor_band_bias(44100, 1, [0.0] * 1152, 0.2, 0, 7, 2)
    mp3_band_gain = sonare_codec.encode_mp3_perceptual_quantized_band_gain(44100, 1, [0.0] * 1152, 0.2, 0, 7, 1.5)
    mp3_band_gain_matched = sonare_codec.encode_mp3_perceptual_quantized_band_gain_global_gain_bias(44100, 1, [0.0] * 1152, 2.0, 0, 7, 1.5, -4)
    if (
        not isinstance(mp3_band_biased, bytes)
        or not isinstance(mp3_band_gain, bytes)
        or not isinstance(mp3_band_gain_matched, bytes)
        or mp3_frame_info(mp3_band_biased)[:3] != (128, 44100, 1)
        or mp3_frame_info(mp3_band_gain)[:3] != (128, 44100, 1)
        or mp3_frame_info(mp3_band_gain_matched)[:3] != (128, 44100, 1)
    ):
        sys.exit("Python wheel MP3 band-local diagnostic helpers returned unexpected frames")
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
    mp3_low_band_shape_profile = sonare_codec.mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate(44100, 1, perceptual_samples, 128, False)
    if (
        len(mp3_low_band_shape_profile) < 7
        or len(mp3_low_band_shape_profile) % 7 != 0
        or not has_approx([mp3_low_band_shape_profile[0]], 0.0005)
        or not any(value > 0 for index, value in enumerate(mp3_low_band_shape_profile) if index % 7 == 3)
        or any(
            value > mp3_low_band_shape_profile[index + 1]
            for index, value in enumerate(mp3_low_band_shape_profile)
            if index % 7 in (3, 5)
        )
    ):
        sys.exit(f"Python wheel MP3 first-frame low-band spectral shape profile returned {mp3_low_band_shape_profile}")
    mp3_band_shape_profile = sonare_codec.mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate(44100, 1, perceptual_samples, 128, False)
    if (
        len(mp3_band_shape_profile) < 10
        or len(mp3_band_shape_profile) % 10 != 0
        or not has_approx([mp3_band_shape_profile[0]], 0.0005)
        or not any(value > 0 for index, value in enumerate(mp3_band_shape_profile) if index % 10 == 6)
        or any(
            value < 0.0 or value >= 21.0
            for index, value in enumerate(mp3_band_shape_profile)
            if index % 10 == 3
        )
        or any(
            value > mp3_band_shape_profile[index + 1]
            for index, value in enumerate(mp3_band_shape_profile)
            if index % 10 == 4
        )
        or any(
            value > mp3_band_shape_profile[index + 2]
            for index, value in enumerate(mp3_band_shape_profile)
            if index % 10 in (6, 7)
        )
    ):
        sys.exit(f"Python wheel MP3 first-frame band spectral shape profile returned {mp3_band_shape_profile}")
    mp3_guarded_candidate_profile = sonare_codec.mp3_first_frame_quality_guarded_candidate_profile_with_bitrate(44100, 1, perceptual_samples, 128, False)
    if (
        len(mp3_guarded_candidate_profile) < 7
        or len(mp3_guarded_candidate_profile) % 7 != 0
        or not has_approx([mp3_guarded_candidate_profile[0]], 0.0005)
        or not any(value > 0 for index, value in enumerate(mp3_guarded_candidate_profile) if index % 7 == 3)
        or not any(value > 0 for index, value in enumerate(mp3_guarded_candidate_profile) if index % 7 == 5)
    ):
        sys.exit(f"Python wheel MP3 first-frame quality-guarded candidate profile returned {mp3_guarded_candidate_profile}")
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
        production_main_data_begins = mp3_main_data_begins(production_reservoir_mp3)
        if channels == 1:
            if production_reservoir_mp3 == entropy_targeted_production_mp3:
                sys.exit(f"Python wheel {label} MP3 production still used the older entropy-targeted perceptual reservoir payload")
            if len(production_main_data_begins) != 8 or not any(value > 0 for value in production_main_data_begins):
                sys.exit(f"Python wheel {label} MP3 production did not expose the mono low-band gain reservoir layout")
        else:
            if production_reservoir_mp3 != entropy_targeted_production_mp3:
                sys.exit(f"Python wheel {label} MP3 production did not use the entropy-targeted perceptual reservoir path")
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
    entropy_profile = sonare_codec.mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate(44100, 1, reservoir_samples, 128, False, 0)
    entropy_payload_bits = 0
    entropy_budget_bits = 0
    entropy_budget_frames = 0
    entropy_max_slack_bits = 0
    for offset in range(0, len(entropy_targeted_reservoir_details), entropy_targeted_reservoir_detail_width):
        payload_bits = entropy_targeted_reservoir_details[offset + 2]
        capacity_bytes = entropy_targeted_reservoir_details[offset + 5]
        main_data_begin = entropy_targeted_reservoir_details[offset + 6]
        target_bits = entropy_targeted_reservoir_details[offset + 12]
        used_target = entropy_targeted_reservoir_details[offset + 13]
        if used_target == 1.0:
            budget_bits = min(max(1, math.ceil(target_bits / 8)), capacity_bytes + main_data_begin) * 8
            entropy_budget_frames += 1
            entropy_payload_bits += payload_bits
            entropy_budget_bits += budget_bits
            entropy_max_slack_bits = max(entropy_max_slack_bits, budget_bits - payload_bits)
    if len(entropy_profile) != 6 or entropy_profile[0] != 8 or entropy_profile[1] != entropy_budget_frames or entropy_profile[2] != entropy_payload_bits or entropy_profile[3] != entropy_budget_bits or abs(entropy_profile[4] - entropy_payload_bits / entropy_budget_bits) > 1e-12 or entropy_profile[5] != entropy_max_slack_bits:
        sys.exit(f"Python wheel MP3 entropy-target utilization profile did not match frame details: {entropy_profile}")
    entropy_targeted_reservoir_mp3 = sonare_codec.encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(44100, 1, reservoir_samples, 128, False, 0)
    entropy_targeted_reservoir_main_data_begins = mp3_main_data_begins(entropy_targeted_reservoir_mp3)
    if len(entropy_targeted_reservoir_main_data_begins) * entropy_targeted_reservoir_detail_width != len(entropy_targeted_reservoir_details):
        sys.exit("Python wheel MP3 entropy-targeted perceptual reservoir frame count did not match selector details")
    for frame, main_data_begin in enumerate(entropy_targeted_reservoir_main_data_begins):
        if main_data_begin != entropy_targeted_reservoir_details[frame * entropy_targeted_reservoir_detail_width + 6]:
            sys.exit("Python wheel MP3 entropy-targeted perceptual reservoir side-info did not match selector details")
    perceptual_reservoir_mp3 = sonare_codec.encode_mp3_perceptual_reservoir_with_bitrate(44100, 1, reservoir_samples, 128, False)
    entropy_targeted_reservoir_production = sonare_codec.encode_audio_production("mp3", 44100, 1, reservoir_samples)
    if entropy_targeted_reservoir_production == entropy_targeted_reservoir_mp3:
        sys.exit("Python wheel MP3 mono production still used the older entropy-targeted perceptual reservoir path")
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

pub(crate) struct SizeEntry {
    pub(crate) kind: &'static str,
    pub(crate) path: PathBuf,
    pub(crate) bytes: Option<u64>,
}
