# sc-opus

Opus codec crate for the Sonare codec workspace.

This crate decodes Ogg Opus streams with one or two channels using Opus mapping
family 0. Native builds also encode 48 kHz mono/stereo PCM into Ogg Opus through
libopus. Multistream channel mappings and WASM Opus encode are not implemented
yet.
