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
`layer3_header_for_capacity`, Layer III main-data capacity helpers, and a
bitrate-selected stream helper for callers that need to budget frame payloads
explicitly without duplicating header calculations. The AAC helper
re-exports include a production-shaped ADTS bitrate encode helper, the standard
scale-factor delta table, the standard unsigned-pairs codebook 7/8 tables used
by the current production-shaped AAC-LC path, and the bit-cost section planner
can select codebook 7/8 across their 0..=7 magnitude range.
