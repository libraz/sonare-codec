# sonare-codec

Unified audio decode and encode API for the Sonare codec workspace.

The default feature set provides Symphonia-backed decode plus WAV, FLAC, and
experimental silent MP3 encode support. The `aac` feature adds AAC ADTS and M4A
helpers, the `vorbis` feature adds Ogg Vorbis decode/encode for native builds,
and the `opus` feature adds Ogg Opus decode plus native encode for mono/stereo
mapping-family-0 streams. Unified `decode(input)` and format-specific decode
helpers share the same backend, with AAC/M4A fallback support for locally
generated silent streams. Some lossy encoder paths are intentionally limited
while the clean-room encoders are being completed. Use
`encode_with_mode(format, pcm, EncodeMode::ProductionOnly)` to reject
non-silent MP3/AAC experimental scaffold output.
