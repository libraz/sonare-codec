# sc-aac

AAC helper crate for the Sonare codec workspace.

The crate currently focuses on AAC-LC ADTS framing, silent PCM encode support,
limited non-silent long-block production candidate helpers, and M4A integration
through `sc-mp4`. Experimental helpers can select a quantizer step that fits
the available spectral tables and ADTS frame limit for mono/stereo long-block
payload scaffolds. Step search evaluates all candidates and can report ADTS
frame length against frame capacity for the future rate-control path. A
standard AAC scale-factor delta table provider is available for exercising the
DPCM path without local test-only tables. Non-zero ICS packing now keeps
section/scale-factor bits before the AAC pulse/TNS/gain flags and spectral bits
after them. Production candidate stream helpers can budget frames by target
bitrate and are checked by the FFmpeg-backed readiness oracle. The standard
unsigned-pairs codebook 7/8 tables are exposed as `aac_unsigned_pairs7_table`
and `aac_unsigned_pairs8_table`, with the older unit-magnitude subset helper
retained for diagnostics, and the public bit-cost section planner can now
select codebook 7/8 across their 0..=7 magnitude range. The deterministic
planner still uses magnitude classification until the remaining codebook syntax
is filled in. The diagnostic auto-step path remains experimental
until the remaining standard codebook syntax is filled in. Broader non-silent
production AAC encoding is still under active development.
