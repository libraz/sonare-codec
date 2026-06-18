# sc-opus

Opus codec crate for the Sonare codec workspace.

This crate decodes Ogg Opus streams with one or two channels using Opus mapping
family 0. Native builds also encode 48 kHz mono/stereo PCM into Ogg Opus through
libopus. The Rust CELT workbench now includes range coding, Laplace residuals,
band normalisation, coarse/fine energy quantisation, CWRS pulse coding, PVQ band
quantisation, and bit-exact theta split-angle primitives. Those pieces are not
yet wired into the live encoder, which still uses libopus on native builds.
Multistream channel mappings and WASM Opus encode are not implemented yet.
