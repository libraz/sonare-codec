from typing import Literal, Optional, Sequence, Tuple, Union

BytesLike = Union[bytes, bytearray, memoryview]
EncodedFormat = Literal["wav", "flac", "mp3", "vorbis", "opus", "aac", "m4a", "mp4"]
PcmTuple = Tuple[int, int, list[float]]

class StreamDecoder:
    def __init__(self) -> None: ...
    def decode_stream(self, chunk: BytesLike) -> Optional[PcmTuple]: ...
    def reset(self) -> None: ...
    def buffered_len(self) -> int: ...

def detect_format(input: BytesLike) -> Optional[str]: ...

def decode_audio(input: BytesLike) -> PcmTuple: ...

def decode_wav(input: BytesLike) -> PcmTuple: ...

def decode_flac(input: BytesLike) -> PcmTuple: ...

def decode_mp3(input: BytesLike) -> PcmTuple: ...

def decode_vorbis(input: BytesLike) -> PcmTuple: ...

def decode_opus(input: BytesLike) -> PcmTuple: ...

def decode_aac(input: BytesLike) -> PcmTuple: ...

def decode_m4a(input: BytesLike) -> PcmTuple: ...

def encode_audio(
    format: EncodedFormat,
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
) -> bytes: ...

def encode_audio_production(
    format: EncodedFormat,
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
) -> bytes: ...

def encode_wav(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_flac(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_mp3(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_mp3_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    bitrate_kbps: int,
    padding: bool,
    crc_protected: bool,
) -> bytes: ...

def encode_mp3_cbr_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    bitrate_kbps: int,
    crc_protected: bool,
) -> bytes: ...

def encode_mp3_perceptual_active_cbr_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    bitrate_kbps: int,
    crc_protected: bool,
) -> bytes: ...

def encode_mp3_perceptual_reservoir_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    bitrate_kbps: int,
    crc_protected: bool,
) -> bytes: ...

def encode_mp3_quality_guarded_perceptual_reservoir_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    bitrate_kbps: int,
    crc_protected: bool,
) -> bytes: ...

def mp3_reservoir_frame_details_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    bitrate_kbps: int,
    crc_protected: bool,
) -> list[float]: ...

def mp3_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    bitrate_kbps: int,
    crc_protected: bool,
) -> list[float]: ...

def mp3_quality_guarded_perceptual_reservoir_frame_details_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    bitrate_kbps: int,
    crc_protected: bool,
) -> list[float]: ...

def encode_vorbis(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_opus(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_aac(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_aac_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
) -> bytes: ...

def encode_aac_with_selected_scale_factors_and_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
) -> bytes: ...

def encode_aac_with_standard_spectral_offsets_and_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
    global_gain: int,
) -> bytes: ...

def encode_aac_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
    global_gain: int,
    scale_factor_magnitude_bias: int,
) -> bytes: ...

def encode_m4a(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_m4a_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
) -> bytes: ...

def encode_m4a_with_selected_scale_factors_and_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
) -> bytes: ...

def encode_m4a_with_standard_spectral_offsets_and_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
    global_gain: int,
) -> bytes: ...

def encode_m4a_with_standard_spectral_offsets_and_selected_scale_factors_with_magnitude_bias_and_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
    global_gain: int,
    scale_factor_magnitude_bias: int,
) -> bytes: ...

def demux_m4a_as_aac_adts(input: BytesLike) -> bytes: ...

def aac_lc_adts_max_frame_len_for_bitrate(
    sample_rate: int,
    target_bitrate_bps: int,
) -> int: ...

def aac_lc_default_production_bitrate_bps(channels: int) -> int: ...

def aac_unsigned_pairs7_unit_magnitude_table() -> list[int]: ...

def aac_unsigned_pairs7_table() -> list[int]: ...

def aac_signed_pairs5_table() -> list[int]: ...

def aac_signed_pairs6_table() -> list[int]: ...

def aac_signed_quads1_table() -> list[int]: ...

def aac_signed_quads2_table() -> list[int]: ...

def aac_unsigned_pairs8_table() -> list[int]: ...

def aac_unsigned_pairs9_table() -> list[int]: ...

def aac_unsigned_pairs10_table() -> list[int]: ...

def aac_unsigned_quads3_table() -> list[int]: ...

def aac_unsigned_quads4_table() -> list[int]: ...

def aac_escape_table() -> list[int]: ...

def aac_scale_factor_delta_table() -> list[int]: ...

def aac_codebook6_unit_section_plan(
    quantized: list[int],
    band_width: int,
) -> list[int]: ...

def aac_quad_unit_section_plan(
    quantized: list[int],
    band_width: int,
) -> list[int]: ...

def aac_mixed_unit_section_plan(
    quantized: list[int],
    band_width: int,
) -> list[int]: ...

def aac_mixed_unit_payload_bit_lengths(
    quantized: list[int],
    band_width: int,
) -> list[int]: ...

def aac_standard_unit_section_plan(
    quantized: list[int],
    band_width: int,
) -> list[int]: ...

def aac_standard_offsets_section_plan(
    quantized: list[int],
    offsets: list[int],
) -> list[int]: ...

def aac_standard_escape_payload_bit_lengths() -> list[int]: ...

def aac_standard_mixed_payload_bit_lengths(
    quantized: list[int],
    band_width: int,
) -> list[int]: ...

def aac_standard_mixed_offsets_payload_bit_lengths(
    quantized: list[int],
    offsets: list[int],
) -> list[int]: ...

def encode_aac_standard_mono_offsets_with_step(
    sample_rate: int,
    samples: list[float],
    step: float,
    global_gain: int,
) -> bytes: ...

def encode_aac_standard_mono_offsets_with_bitrate(
    sample_rate: int,
    samples: list[float],
    target_bitrate_bps: int,
    global_gain: int,
) -> bytes: ...

def aac_standard_mono_offsets_bitrate_frame_details(
    sample_rate: int,
    samples: list[float],
    target_bitrate_bps: int,
    global_gain: int,
) -> list[float]: ...

def encode_aac_standard_stereo_offsets_with_step(
    sample_rate: int,
    samples: list[float],
    step: float,
    global_gain: int,
) -> bytes: ...

def encode_aac_standard_stereo_offsets_with_bitrate(
    sample_rate: int,
    samples: list[float],
    target_bitrate_bps: int,
    global_gain: int,
) -> bytes: ...

def aac_standard_stereo_offsets_bitrate_frame_details(
    sample_rate: int,
    samples: list[float],
    target_bitrate_bps: int,
    global_gain: int,
) -> list[float]: ...

def aac_standard_selected_scale_factor_frame_details_with_magnitude_bias_and_bitrate(
    sample_rate: int,
    channels: int,
    samples: list[float],
    target_bitrate_bps: int,
    global_gain: int,
    scale_factor_magnitude_bias: int,
) -> list[float]: ...

def aac_selected_scale_factor_frame_details_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: list[float],
    target_bitrate_bps: int,
) -> list[float]: ...

def mp3_layer3_main_data_capacity_bytes(
    sample_rate: int,
    channels: int,
    bitrate_kbps: int,
    padding: bool,
    crc_protected: bool,
) -> int: ...

def mp3_layer3_main_data_capacity_bits(
    sample_rate: int,
    channels: int,
    bitrate_kbps: int,
    padding: bool,
    crc_protected: bool,
) -> int: ...
