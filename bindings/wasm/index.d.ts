export type EncodedFormat =
  | "wav"
  | "flac"
  | "mp3"
  | "vorbis"
  | "opus"
  | "aac"
  | "m4a"
  | "mp4";

export class WavPcm {
  readonly sample_rate: number;
  readonly channels: number;
  samples(): Float32Array;
}

export class StreamDecoder {
  constructor();
  decode_stream(input: Uint8Array): WavPcm | undefined;
  reset(): void;
  buffered_len(): number;
}

export function detect_format(input: Uint8Array): string | undefined;

export function decode_audio(input: Uint8Array): WavPcm;

export function decode_wav(input: Uint8Array): WavPcm;

export function decode_flac(input: Uint8Array): WavPcm;

export function decode_mp3(input: Uint8Array): WavPcm;

export function decode_vorbis(input: Uint8Array): WavPcm;

export function decode_opus(input: Uint8Array): WavPcm;

export function decode_aac(input: Uint8Array): WavPcm;

export function decode_m4a(input: Uint8Array): WavPcm;

export function encode_audio(
  format: EncodedFormat,
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_audio_production(
  format: EncodedFormat,
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_wav(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_flac(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_mp3(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_aac(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_m4a(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function demux_m4a_as_aac_adts(input: Uint8Array): Uint8Array;
