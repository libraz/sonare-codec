# @libraz/sonare-codec

Rust/WASM bindings for `sonare-codec`.

```ts
import init, {
  decode_audio,
  decode_m4a,
  decode_mp3,
  demux_m4a_as_aac_adts,
  detect_format,
  encode_audio,
  encode_audio_production,
  StreamDecoder,
} from "@libraz/sonare-codec";

await init();

const pcm = decode_audio(bytes);
const flac = encode_audio("flac", pcm.sample_rate, pcm.channels, pcm.samples());
const mp3 = encode_audio("mp3", 44100, 1, new Float32Array(1152));
const productionMp3 = encode_audio_production("mp3", 44100, 1, new Float32Array(1152));
const mp3Pcm = decode_mp3(mp3);
const aac = encode_audio("aac", 44100, 1, new Float32Array(1024));
const m4a = encode_audio("m4a", 44100, 1, new Float32Array(1024));
const kind = detect_format(m4a); // "m4a"
const adts = demux_m4a_as_aac_adts(m4a);
const m4aPcm = decode_m4a(m4a);

const decoder = new StreamDecoder();
const partial = flac.slice(0, flac.length - 2);
console.assert(decoder.decode_stream(partial) === undefined);
const streamed = decoder.decode_stream(flac.slice(flac.length - 2));
```

Current WASM encoder support is WAV, FLAC, MP3 Layer III, and AAC-LC ADTS/M4A.
MP3/AAC non-silent output still uses experimental long-block scaffolds, and
Vorbis/Opus encode is still incomplete on the WASM surface. Use
`encode_audio_production` to reject non-silent AAC/MP3 experimental scaffold
output.
