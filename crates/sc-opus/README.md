# sc-opus

Opus codec crate for the Sonare codec workspace.

This crate decodes Ogg Opus streams with one or two channels using Opus mapping
family 0. It also encodes 48 kHz mono/stereo PCM into an Ogg Opus stream through
a first-party, pure-Rust CELT encoder with no C dependency, so encode also
builds for the wasm target. The CELT path covers range coding, Laplace
residuals, band normalisation, coarse/fine energy quantisation, CWRS pulse
coding, PVQ band quantisation, bit-exact theta split-angle coding, the comb
prefilter/postfilter, transient analysis, and VBR rate control, and is wired
into the live encoder; its self-encoded packets round-trip through the
independent pure-Rust Opus decoder. Encoding is CELT-only fullband at 20 ms per
frame; SILK/hybrid modes and multistream channel mappings are not implemented
yet.
