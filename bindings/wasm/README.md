# @libraz/sonare-codec

Rust/WASM bindings for `sonare-codec`.

```ts
import init, {
  decode_audio,
  decode_m4a,
  decode_mp3,
  demux_m4a_as_aac_adts,
  detect_format,
  aac_lc_adts_max_frame_len_for_bitrate,
  aac_unsigned_pairs7_unit_magnitude_table,
  encode_audio,
  encode_audio_production,
  encode_aac_with_bitrate,
  encode_m4a_with_bitrate,
  encode_mp3_with_bitrate,
  mp3_layer3_main_data_capacity_bytes,
  StreamDecoder,
} from "@libraz/sonare-codec";

await init();

const pcm = decode_audio(bytes);
const flac = encode_audio("flac", pcm.sample_rate, pcm.channels, pcm.samples());
const mp3 = encode_audio("mp3", 44100, 1, new Float32Array(1152));
const productionMp3 = encode_audio_production("mp3", 44100, 1, new Float32Array(1152));
const mp3_96k = encode_mp3_with_bitrate(44100, 1, new Float32Array(1152), 96, false, false);
const mp3Pcm = decode_mp3(mp3);
const aac = encode_audio("aac", 44100, 1, new Float32Array(1024));
const aac_10k = encode_aac_with_bitrate(44100, 1, new Float32Array(2048), 10000);
const m4a = encode_audio("m4a", 44100, 1, new Float32Array(1024));
const m4a_10k = encode_m4a_with_bitrate(44100, 1, new Float32Array(2048), 10000);
const kind = detect_format(m4a); // "m4a"
const adts = demux_m4a_as_aac_adts(m4a);
const m4aPcm = decode_m4a(m4a);
const aacBudget = aac_lc_adts_max_frame_len_for_bitrate(44100, 10000);
const aacCodebook7 = aac_unsigned_pairs7_unit_magnitude_table();
const mp3Capacity = mp3_layer3_main_data_capacity_bytes(44100, 1, 128, false, false);

const decoder = new StreamDecoder();
const partial = flac.slice(0, flac.length - 2);
console.assert(decoder.decode_stream(partial) === undefined);
const streamed = decoder.decode_stream(flac.slice(flac.length - 2));
```

Current WASM encoder support is WAV, FLAC, MP3 Layer III, and AAC-LC ADTS/M4A.
`encode_audio_production` accepts the current non-silent lossy production
candidates: mono/stereo MP3 at 32/44.1/48 kHz and mono/stereo AAC-LC ADTS/M4A
at 7.35/8/11.025/12/16/22.05/24/32/44.1/48/64/88.2/96 kHz. Other non-silent
MP3/AAC shapes are rejected, and Vorbis/Opus encode is still incomplete on the
WASM surface. The package also exposes small lossy diagnostics for AAC ADTS
bitrate budgets, AAC scale-factor/codebook 7/8 tables, MP3 Layer III main-data
capacity, and caller-selected AAC/MP3 bitrate encoding.
