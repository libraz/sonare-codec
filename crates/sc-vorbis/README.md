# sc-vorbis

Vorbis codec crate for the Sonare codec workspace.

This crate provides Ogg Vorbis decode through the shared Symphonia decode
adapter and Ogg Vorbis encode through a first-party, pure-Rust encoder. The
encoder has no C dependency, so it also builds for the wasm target. It emits a
Symphonia-decodable Ogg Vorbis stream for mono and stereo PCM, with a floor1 +
multi-stage residue cascade, long/short block switching, and square-polar
stereo channel coupling for strongly correlated input.
