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

def encode_vorbis(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_opus(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_aac(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_aac_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
) -> bytes: ...

def encode_m4a(sample_rate: int, channels: int, samples: Sequence[float]) -> bytes: ...

def encode_m4a_with_bitrate(
    sample_rate: int,
    channels: int,
    samples: Sequence[float],
    target_bitrate_bps: int,
) -> bytes: ...

def demux_m4a_as_aac_adts(input: BytesLike) -> bytes: ...

def aac_lc_adts_max_frame_len_for_bitrate(
    sample_rate: int,
    target_bitrate_bps: int,
) -> int: ...

def aac_unsigned_pairs7_unit_magnitude_table() -> list[int]: ...

def aac_unsigned_pairs7_table() -> list[int]: ...

def aac_unsigned_pairs8_table() -> list[int]: ...

def aac_scale_factor_delta_table() -> list[int]: ...

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
