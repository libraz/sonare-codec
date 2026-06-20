use super::*;

pub(crate) fn run_npm_pack_output_check() -> Result<(), String> {
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
    glue.aac_standard_id_selected_scale_factor_magnitude_bias() !== 16 ||
    glue.aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(1) !== 2047 ||
    glue.aac_standard_id_selected_scale_factor_balanced_max_quantized_abs(2) !== 1535) {
  throw new Error("npm AAC standard-id selected-scale-factor recommended parameters returned unexpected values");
}
const aacRecommendedMonoParameters = Array.from(glue.aac_standard_id_selected_scale_factor_parameters(1));
const aacRecommendedStereoParameters = Array.from(glue.aac_standard_id_selected_scale_factor_parameters(2));
if (JSON.stringify(aacRecommendedMonoParameters) !== JSON.stringify([128, 16]) ||
    JSON.stringify(aacRecommendedStereoParameters) !== JSON.stringify([126, 16])) {
  throw new Error(` + "`npm AAC standard-id selected-scale-factor parameter helper returned ${JSON.stringify({aacRecommendedMonoParameters, aacRecommendedStereoParameters})}`" + `);
}
const aacBalancedMonoParameters = Array.from(glue.aac_standard_id_selected_scale_factor_balanced_parameters(1));
const aacBalancedStereoParameters = Array.from(glue.aac_standard_id_selected_scale_factor_balanced_parameters(2));
if (JSON.stringify(aacBalancedMonoParameters) !== JSON.stringify([136, 8, 2047]) ||
    JSON.stringify(aacBalancedStereoParameters) !== JSON.stringify([138, 4, 1535])) {
  throw new Error(` + "`npm AAC balanced standard-id selected-scale-factor parameter helper returned ${JSON.stringify({aacBalancedMonoParameters, aacBalancedStereoParameters})}`" + `);
}
const aacBalancedMonoGainDeltas = Array.from(glue.aac_standard_id_selected_scale_factor_balanced_gain_deltas(1));
const aacBalancedStereoGainDeltas = Array.from(glue.aac_standard_id_selected_scale_factor_balanced_gain_deltas(2));
const aacBalancedMonoBiases = Array.from(glue.aac_standard_id_selected_scale_factor_balanced_magnitude_biases(1));
const aacBalancedStereoBiases = Array.from(glue.aac_standard_id_selected_scale_factor_balanced_magnitude_biases(2));
if (JSON.stringify(aacBalancedMonoGainDeltas) !== JSON.stringify([0, 2, 4, 6, 8]) ||
    JSON.stringify(aacBalancedStereoGainDeltas) !== JSON.stringify([8, 12, 16]) ||
    JSON.stringify(aacBalancedMonoBiases) !== JSON.stringify([8, 12, 16, 20]) ||
    JSON.stringify(aacBalancedStereoBiases) !== JSON.stringify([4, 8, 12])) {
  throw new Error(` + "`npm AAC balanced standard-id selected-scale-factor profile helper returned ${JSON.stringify({aacBalancedMonoGainDeltas, aacBalancedStereoGainDeltas, aacBalancedMonoBiases, aacBalancedStereoBiases})}`" + `);
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
const standardSelectedMaxAbsAdts = glue.encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16, 2047);
if (!(standardSelectedMaxAbsAdts instanceof Uint8Array) || standardSelectedMaxAbsAdts[0] !== 0xff || standardSelectedMaxAbsAdts[1] !== 0xf1 || maxAdtsFrameLen(standardSelectedMaxAbsAdts) > 372) {
  throw new Error("npm AAC standard selected max-abs helper returned unexpected ADTS");
}
const recommendedStandardSelectedMaxAbsAdts = glue.encode_aac_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 2047);
if (Buffer.compare(Buffer.from(recommendedStandardSelectedMaxAbsAdts), Buffer.from(standardSelectedMaxAbsAdts)) !== 0) {
  throw new Error("npm AAC recommended standard selected max-abs helper did not match explicit mono parameters");
}
const balancedStandardSelectedAdts = glue.encode_aac_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, new Float32Array(2048), 128000);
const expectedBalancedStandardSelectedAdts = glue.encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 136, 8, 2047);
if (Buffer.compare(Buffer.from(balancedStandardSelectedAdts), Buffer.from(expectedBalancedStandardSelectedAdts)) !== 0) {
  throw new Error("npm AAC balanced standard selected helper did not match balanced mono parameters");
}
const standardSelectedGenericM4a = glue.encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16);
if (!(standardSelectedGenericM4a instanceof Uint8Array) || String.fromCharCode(...standardSelectedGenericM4a.slice(4, 8)) !== "ftyp") {
  throw new Error("npm M4A standard selected spectral-offset bitrate helper returned unexpected container");
}
const recommendedStandardSelectedGenericM4a = glue.encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, new Float32Array(2048), 128000);
if (Buffer.compare(Buffer.from(recommendedStandardSelectedGenericM4a), Buffer.from(standardSelectedGenericM4a)) !== 0) {
  throw new Error("npm M4A recommended standard selected helper did not match explicit mono parameters");
}
const standardSelectedMaxAbsM4a = glue.encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16, 2047);
if (!(standardSelectedMaxAbsM4a instanceof Uint8Array) || String.fromCharCode(...standardSelectedMaxAbsM4a.slice(4, 8)) !== "ftyp") {
  throw new Error("npm M4A standard selected max-abs helper returned unexpected container");
}
const recommendedStandardSelectedMaxAbsM4a = glue.encode_m4a_with_recommended_standard_spectral_offsets_and_selected_scale_factors_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 2047);
if (Buffer.compare(Buffer.from(recommendedStandardSelectedMaxAbsM4a), Buffer.from(standardSelectedMaxAbsM4a)) !== 0) {
  throw new Error("npm M4A recommended standard selected max-abs helper did not match explicit mono parameters");
}
const balancedStandardSelectedM4a = glue.encode_m4a_with_balanced_standard_spectral_offsets_and_selected_scale_factors_and_bitrate(44100, 1, new Float32Array(2048), 128000);
if (!(balancedStandardSelectedM4a instanceof Uint8Array) || String.fromCharCode(...balancedStandardSelectedM4a.slice(4, 8)) !== "ftyp") {
  throw new Error("npm M4A balanced standard selected helper returned unexpected container");
}
const standardSelectedMaxAbsM4aAdts = glue.demux_m4a_as_aac_adts(standardSelectedMaxAbsM4a);
if (standardSelectedMaxAbsM4aAdts.length !== standardSelectedMaxAbsAdts.length || !standardSelectedMaxAbsM4aAdts.every((byte, index) => byte === standardSelectedMaxAbsAdts[index])) {
  throw new Error("npm M4A standard selected max-abs helper did not mux the expected ADTS");
}
const balancedStandardSelectedM4aAdts = glue.demux_m4a_as_aac_adts(balancedStandardSelectedM4a);
if (Buffer.compare(Buffer.from(balancedStandardSelectedM4aAdts), Buffer.from(balancedStandardSelectedAdts)) !== 0) {
  throw new Error("npm M4A balanced standard selected helper did not mux the expected ADTS");
}
const standardSelectedDetails = Array.from(glue.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16));
if (standardSelectedDetails.length !== 8 || standardSelectedDetails[0] !== 0 || standardSelectedDetails[4] !== 1 || standardSelectedDetails[2] > 372 || standardSelectedDetails[6] > 372) {
  throw new Error(` + "`npm AAC standard selected bitrate details returned ${JSON.stringify(standardSelectedDetails)}`" + `);
}
const recommendedStandardSelectedDetails = Array.from(glue.aac_recommended_standard_selected_scale_factor_frame_details_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (JSON.stringify(recommendedStandardSelectedDetails) !== JSON.stringify(standardSelectedDetails)) {
  throw new Error(` + "`npm AAC recommended standard selected bitrate details returned ${JSON.stringify(recommendedStandardSelectedDetails)}`" + `);
}
const standardSelectedProfile = Array.from(glue.aac_standard_selected_scale_factor_profile_with_magnitude_bias_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16));
if (JSON.stringify(standardSelectedProfile) !== JSON.stringify([2, 1, 98, 0, 0, 0])) {
  throw new Error(` + "`npm AAC standard selected profile returned ${JSON.stringify(standardSelectedProfile)}`" + `);
}
const recommendedStandardSelectedProfile = Array.from(glue.aac_recommended_standard_selected_scale_factor_profile_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (JSON.stringify(recommendedStandardSelectedProfile) !== JSON.stringify(standardSelectedProfile)) {
  throw new Error(` + "`npm AAC recommended standard selected profile returned ${JSON.stringify(recommendedStandardSelectedProfile)}`" + `);
}
const balancedStandardSelectedProfile = Array.from(glue.aac_balanced_standard_selected_scale_factor_profile_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (JSON.stringify(balancedStandardSelectedProfile) !== JSON.stringify(standardSelectedProfile)) {
  throw new Error(` + "`npm AAC balanced standard selected profile returned ${JSON.stringify(balancedStandardSelectedProfile)}`" + `);
}
const standardPayloadBreakdown = Array.from(glue.aac_standard_id_payload_breakdown_with_magnitude_bias_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16));
if (standardPayloadBreakdown.length !== 11 || standardPayloadBreakdown[0] !== 2 || standardPayloadBreakdown[1] !== 1 || standardPayloadBreakdown[3] !== 0 || standardPayloadBreakdown[4] !== 0 || standardPayloadBreakdown[8] !== 0 || standardPayloadBreakdown[10] !== 0) {
  throw new Error(` + "`npm AAC standard-id payload breakdown returned ${JSON.stringify(standardPayloadBreakdown)}`" + `);
}
const recommendedPayloadBreakdown = Array.from(glue.aac_recommended_standard_id_payload_breakdown_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (JSON.stringify(recommendedPayloadBreakdown) !== JSON.stringify(standardPayloadBreakdown)) {
  throw new Error(` + "`npm AAC recommended standard-id payload breakdown returned ${JSON.stringify(recommendedPayloadBreakdown)}`" + `);
}
const balancedPayloadBreakdown = Array.from(glue.aac_balanced_standard_id_payload_breakdown_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (JSON.stringify(balancedPayloadBreakdown) !== JSON.stringify(standardPayloadBreakdown)) {
  throw new Error(` + "`npm AAC balanced standard-id payload breakdown returned ${JSON.stringify(balancedPayloadBreakdown)}`" + `);
}
const explicitBalancedQualityProfile = Array.from(glue.aac_standard_id_quality_control_profile_with_magnitude_bias_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 136, 8, 2047));
const balancedQualityProfile = Array.from(glue.aac_balanced_standard_id_quality_control_profile_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (balancedQualityProfile.length !== 16 ||
    JSON.stringify(balancedQualityProfile) !== JSON.stringify(explicitBalancedQualityProfile) ||
    balancedQualityProfile[0] !== 2 ||
    balancedQualityProfile[1] !== 1 ||
    balancedQualityProfile[3] < 0 ||
    balancedQualityProfile[4] !== 2047 ||
    balancedQualityProfile[5] !== 0 ||
    balancedQualityProfile[10] !== 0 ||
    balancedQualityProfile[13] !== 0) {
  throw new Error(` + "`npm AAC balanced quality-control profile returned ${JSON.stringify(balancedQualityProfile)}`" + `);
}
const balancedQualityCandidates = Array.from(glue.aac_standard_id_quality_control_candidates_for_balance_profile_with_bitrate(44100, 1, new Float32Array(2048), 128000));
if (balancedQualityCandidates.length === 0 ||
    balancedQualityCandidates.length % 19 !== 0 ||
    !balancedQualityCandidates.some((value, index) => index % 19 === 0 && value === 136) ||
    !balancedQualityCandidates.some((value, index) => index % 19 === 1 && value === 8) ||
    !balancedQualityCandidates.some((value, index) => index % 19 === 2 && value === 2047)) {
  throw new Error(` + "`npm AAC balanced quality-control candidates returned ${JSON.stringify(balancedQualityCandidates)}`" + `);
}
const standardSelectedMaxAbsDetails = Array.from(glue.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 128, 16, 2047));
if (standardSelectedMaxAbsDetails.length !== 8 || standardSelectedMaxAbsDetails[0] !== 0 || standardSelectedMaxAbsDetails[4] !== 1 || standardSelectedMaxAbsDetails[2] > 372 || standardSelectedMaxAbsDetails[6] > 372) {
  throw new Error(` + "`npm AAC standard selected max-abs bitrate details returned ${JSON.stringify(standardSelectedMaxAbsDetails)}`" + `);
}
const recommendedStandardSelectedMaxAbsDetails = Array.from(glue.aac_recommended_standard_selected_scale_factor_frame_details_with_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 2047));
if (JSON.stringify(recommendedStandardSelectedMaxAbsDetails) !== JSON.stringify(standardSelectedMaxAbsDetails)) {
  throw new Error(` + "`npm AAC recommended standard selected max-abs bitrate details returned ${JSON.stringify(recommendedStandardSelectedMaxAbsDetails)}`" + `);
}
const balancedStandardSelectedDetails = Array.from(glue.aac_balanced_standard_selected_scale_factor_frame_details_with_bitrate(44100, 1, new Float32Array(2048), 128000));
const expectedBalancedStandardSelectedDetails = Array.from(glue.aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_max_quantized_abs_and_bitrate(44100, 1, new Float32Array(2048), 128000, 136, 8, 2047));
if (JSON.stringify(balancedStandardSelectedDetails) !== JSON.stringify(expectedBalancedStandardSelectedDetails)) {
  throw new Error(` + "`npm AAC balanced standard selected details returned ${JSON.stringify(balancedStandardSelectedDetails)}`" + `);
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
const mp3MonoProductionSteps = Array.from(glue.mp3_production_pcm_step_candidates(1));
const mp3StereoProductionSteps = Array.from(glue.mp3_production_pcm_step_candidates(2));
if (mp3MonoProductionSteps[0] !== 2 ||
    hasApprox(mp3MonoProductionSteps, 0.2) ||
    JSON.stringify(mp3StereoProductionSteps) !== JSON.stringify(mp3Steps)) {
  throw new Error(` + "`npm MP3 production step candidates returned ${JSON.stringify({mp3MonoProductionSteps, mp3StereoProductionSteps})}`" + `);
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
const mp3BandBiased = glue.encode_mp3_perceptual_scale_factor_band_bias(44100, 1, new Float32Array(1152), 0.2, 0, 7, 2);
const mp3BandGain = glue.encode_mp3_perceptual_quantized_band_gain(44100, 1, new Float32Array(1152), 0.2, 0, 7, 1.5);
const mp3BandGainMatched = glue.encode_mp3_perceptual_quantized_band_gain_global_gain_bias(44100, 1, new Float32Array(1152), 2.0, 0, 7, 1.5, -4);
const mp3BandBiasedInfo = mp3FrameInfo(mp3BandBiased);
const mp3BandGainInfo = mp3FrameInfo(mp3BandGain);
const mp3BandGainMatchedInfo = mp3FrameInfo(mp3BandGainMatched);
if (!(mp3BandBiased instanceof Uint8Array) ||
    !(mp3BandGain instanceof Uint8Array) ||
    !(mp3BandGainMatched instanceof Uint8Array) ||
    mp3BandBiasedInfo.sampleRate !== 44100 ||
    mp3BandGainInfo.sampleRate !== 44100 ||
    mp3BandGainMatchedInfo.sampleRate !== 44100 ||
    mp3BandBiasedInfo.channels !== 1 ||
    mp3BandGainInfo.channels !== 1 ||
    mp3BandGainMatchedInfo.channels !== 1) {
  throw new Error("npm MP3 band-local diagnostic helpers returned unexpected frames");
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
const mp3LowBandShapeProfile = Array.from(glue.mp3_first_frame_low_band_spectral_shape_candidate_profile_with_bitrate(44100, 1, perceptualSamples, 128, false));
if (mp3LowBandShapeProfile.length < 7 ||
    mp3LowBandShapeProfile.length % 7 !== 0 ||
    !hasApprox([mp3LowBandShapeProfile[0]], 0.0005) ||
    !mp3LowBandShapeProfile.some((value, index) => index % 7 === 3 && value > 0) ||
    !mp3LowBandShapeProfile.every((value, index, profile) => {
      const slot = index % 7;
      if (slot === 3) return value <= profile[index + 1];
      if (slot === 5) return value <= profile[index + 1];
      return true;
    })) {
  throw new Error(` + "`npm MP3 first-frame low-band spectral shape profile returned ${JSON.stringify(mp3LowBandShapeProfile)}`" + `);
}
const mp3BandShapeProfile = Array.from(glue.mp3_first_frame_band_spectral_shape_candidate_profile_with_bitrate(44100, 1, perceptualSamples, 128, false));
if (mp3BandShapeProfile.length < 10 ||
    mp3BandShapeProfile.length % 10 !== 0 ||
    !hasApprox([mp3BandShapeProfile[0]], 0.0005) ||
    !mp3BandShapeProfile.some((value, index) => index % 10 === 6 && value > 0) ||
    !mp3BandShapeProfile.every((value, index, profile) => {
      const slot = index % 10;
      if (slot === 3) return value >= 0 && value < 21;
      if (slot === 4) return value <= profile[index + 1];
      if (slot === 6) return value <= profile[index + 2];
      if (slot === 7) return value <= profile[index + 2];
      return true;
    })) {
  throw new Error(` + "`npm MP3 first-frame band spectral shape profile returned ${JSON.stringify(mp3BandShapeProfile)}`" + `);
}
const mp3GuardedCandidateProfile = Array.from(glue.mp3_first_frame_quality_guarded_candidate_profile_with_bitrate(44100, 1, perceptualSamples, 128, false));
if (mp3GuardedCandidateProfile.length < 7 ||
    mp3GuardedCandidateProfile.length % 7 !== 0 ||
    !hasApprox([mp3GuardedCandidateProfile[0]], 0.0005) ||
    !mp3GuardedCandidateProfile.some((value, index) => index % 7 === 3 && value > 0) ||
    !mp3GuardedCandidateProfile.some((value, index) => index % 7 === 5 && value > 0)) {
  throw new Error(` + "`npm MP3 first-frame quality-guarded candidate profile returned ${JSON.stringify(mp3GuardedCandidateProfile)}`" + `);
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
  const profileHelper = glue.mp3_entropy_targeted_perceptual_reservoir_utilization_profile_with_bitrate;
  const reservoirDetails = Array.from(detailHelper(44100, channels, samples, 128, false, 0));
  const utilizationProfile = Array.from(profileHelper(44100, channels, samples, 128, false, 0));
  if (reservoirDetails.length !== 8 * detailWidth || reservoirDetails[0] !== 0 || reservoirDetails[6] !== 0) {
    throw new Error(` + "`npm ${label} MP3 reservoir detail helper returned malformed frame details`" + `);
  }
  let reservoirBorrowed = false;
  let entropyTargetBits = 0;
  let capacityBits = 0;
  let entropyTargetBudgetFrames = 0;
  let entropyPayloadBits = 0;
  let entropyBudgetBits = 0;
  let maxEntropySlackBits = 0;
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
      const roundedBudgetBits = Math.min(
        Math.max(1, Math.ceil(frameEntropyTargetBits / 8)),
        capacityBytes + mainDataBegin,
      ) * 8;
      entropyPayloadBits += payloadBits;
      entropyBudgetBits += roundedBudgetBits;
      maxEntropySlackBits = Math.max(maxEntropySlackBits, roundedBudgetBits - payloadBits);
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
  if (
    utilizationProfile.length !== 6 ||
    utilizationProfile[0] !== 8 ||
    utilizationProfile[1] !== entropyTargetBudgetFrames ||
    utilizationProfile[2] !== entropyPayloadBits ||
    utilizationProfile[3] !== entropyBudgetBits ||
    Math.abs(utilizationProfile[4] - entropyPayloadBits / entropyBudgetBits) > 1e-12 ||
    utilizationProfile[5] !== maxEntropySlackBits
  ) {
    throw new Error(` + "`npm ${label} MP3 entropy-target utilization profile did not match frame details`" + `);
  }
  const reservoirProduction = glue.encode_audio_production("mp3", 44100, channels, samples);
  const entropyTargetedProduction = glue.encode_mp3_entropy_targeted_perceptual_reservoir_with_bitrate(44100, channels, samples, 128, false, 0);
  const productionMainDataBegins = mp3MainDataBegins(reservoirProduction);
  if (channels === 1) {
    if (Buffer.compare(Buffer.from(reservoirProduction), Buffer.from(entropyTargetedProduction)) === 0) {
      throw new Error(` + "`npm ${label} MP3 production still used the older entropy-targeted perceptual reservoir payload`" + `);
    }
    if (productionMainDataBegins.length !== 8 || !productionMainDataBegins.some((value) => value > 0)) {
      throw new Error(` + "`npm ${label} MP3 production did not expose the mono low-band gain reservoir layout`" + `);
    }
  } else {
    if (Buffer.compare(Buffer.from(reservoirProduction), Buffer.from(entropyTargetedProduction)) !== 0) {
      throw new Error(` + "`npm ${label} MP3 production did not use the entropy-targeted perceptual reservoir path`" + `);
    }
    if (productionMainDataBegins.length * detailWidth !== reservoirDetails.length) {
      throw new Error(` + "`npm ${label} MP3 production reservoir frame count did not match selector details`" + `);
    }
    for (let frame = 0; frame < productionMainDataBegins.length; frame += 1) {
      if (productionMainDataBegins[frame] !== reservoirDetails[frame * detailWidth + 6]) {
        throw new Error(` + "`npm ${label} MP3 production reservoir side-info did not match selector details`" + `);
      }
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
if (Buffer.compare(Buffer.from(entropyTargetedReservoirProduction), Buffer.from(entropyTargetedReservoirMp3)) === 0) {
  throw new Error("npm MP3 mono production still used the older entropy-targeted perceptual reservoir path");
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
