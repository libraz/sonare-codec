# sc-aac

AAC helper crate for the Sonare codec workspace.

The crate currently focuses on AAC-LC ADTS framing, silent PCM encode support,
an experimental non-silent long-block scaffold, and M4A integration through
`sc-mp4`. Experimental helpers can select a quantizer step that fits the
available spectral tables and ADTS frame limit for mono/stereo long-block
payload scaffolds. Step search evaluates all candidates and can report ADTS
frame length against frame capacity for the future rate-control path. A minimal
experimental scale-factor delta table provider is available for exercising the
DPCM path without local test-only tables. Non-zero ICS packing now keeps
section/scale-factor bits before the AAC pulse/TNS/gain flags and spectral bits
after them. The public non-silent compatibility scaffold remains zero-spectral,
while the internal non-zero experimental path is kept behind diagnostics
because FFmpeg still rejects it as an invalid standard AAC-LC bitstream until
the remaining standard codebook syntax is filled in. Non-silent production AAC
encoding is still under active development.
