# sonare-codec

Unified audio decode and encode API for the Sonare codec workspace.

The default feature set provides Symphonia-backed decode plus WAV, FLAC, and
limited MP3 encode support. The `aac` feature adds AAC ADTS and M4A helpers,
the `vorbis` feature adds Ogg Vorbis decode/encode for native builds, and the
`opus` feature adds Ogg Opus decode plus native encode for mono/stereo
mapping-family-0 streams. Unified `decode(input)` and format-specific decode
helpers share the same backend, with AAC/M4A fallback support for locally
generated silent streams. Some lossy encoder paths are intentionally limited
while the clean-room encoders are being completed. Use
`encode_with_mode(format, pcm, EncodeMode::ProductionOnly)` to reject lossy
inputs outside the current mono/stereo MP3 MPEG-1 sample-rate and mono/stereo
AAC-LC ADTS/M4A 7.35/8/11.025/12/16/22.05/24/32/44.1/48/64/88.2/96 kHz
production candidate paths. The MP3 helper re-exports include
`layer3_header_for_capacity`, Layer III main-data capacity helpers, a
bitrate-selected stream helper plus a CBR padding-scheduled variant for callers
that need to budget frame payloads explicitly without duplicating header
calculations, the low-level psychoacoustic long-block scale-factor selector, and perceptual scale-factor frame/stream helpers with payload-budget and bitrate-derived capacity search used to validate future MP3 bit-allocation work. The AAC helper
re-exports include production-shaped ADTS and M4A bitrate encode helpers for
both fixed and internally selected scale-factor paths, the standard
scale-factor delta table, the standard unsigned-pairs codebook 7/8/9/10 tables
used by the current production-shaped AAC-LC path, and the escape codebook 11
table for explicit diagnostic packing. A standard AAC-LC spectral table-set
helper also exposes that escape table for diagnostics and future rate-control
work; production encode keeps the current FFmpeg-oracle-passing table set until
escape-coded output is accepted by the readiness gate. The bit-cost section
planner can select codebook 7/8/9/10 by default, codebook 6 when a
caller-supplied signed-pair table is provided, and codebook 11 when an explicit
escape table is supplied.
The default magnitude-classified AAC section planner also routes non-zero
sections through the available standard unsigned-pairs codebook 7/9 tables
before falling back to escape-class sections.
Mono/stereo offset-based selected-scale-factor stream helpers can also enforce
caller-provided ADTS max-frame or bitrate budgets, and the public non-silent
AAC-LC production candidate encode path uses that selected-scale-factor offsets
path with the default production bitrate budget.
