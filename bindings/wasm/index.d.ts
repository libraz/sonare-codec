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

export function encode_mp3_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  bitrate_kbps: number,
  padding: boolean,
  crc_protected: boolean
): Uint8Array;

export function encode_aac(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_aac_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number
): Uint8Array;

export function encode_m4a(
  sample_rate: number,
  channels: number,
  samples: Float32Array
): Uint8Array;

export function encode_m4a_with_bitrate(
  sample_rate: number,
  channels: number,
  samples: Float32Array,
  target_bitrate_bps: number
): Uint8Array;

export function demux_m4a_as_aac_adts(input: Uint8Array): Uint8Array;

export function aac_lc_adts_max_frame_len_for_bitrate(
  sample_rate: number,
  target_bitrate_bps: number
): number;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_unsigned_pairs7_unit_magnitude_table(): Uint32Array;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_unsigned_pairs7_table(): Uint32Array;

/**
 * Returns flattened entries as [x, y, bits, len, ...].
 */
export function aac_unsigned_pairs8_table(): Uint32Array;

/**
 * Returns flattened entries as [delta, bits, len, ...].
 */
export function aac_scale_factor_delta_table(): Int32Array;

export function mp3_layer3_main_data_capacity_bytes(
  sample_rate: number,
  channels: number,
  bitrate_kbps: number,
  padding: boolean,
  crc_protected: boolean
): number;

export function mp3_layer3_main_data_capacity_bits(
  sample_rate: number,
  channels: number,
  bitrate_kbps: number,
  padding: boolean,
  crc_protected: boolean
): number;
