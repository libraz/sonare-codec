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
bitrate and are checked by the FFmpeg-backed readiness oracle. Offset-based
mono/stereo selected-scale-factor stream helpers can also select steps under
caller-provided max-frame or bitrate budgets, keeping scale-factor selection,
bit-cost section planning, and ADTS frame budgeting on one helper path; the
public non-silent production `encode()` scaffold now uses that selected
scale-factor offsets path with a conservative default bitrate budget as well. The
standard signed-pairs codebook 5/6 tables, unsigned-quad codebook 3/4 tables,
signed-quad codebook 1/2 tables, unsigned-pairs codebook 7/8/9/10 tables, and
escape codebook 11 table are exposed as `aac_signed_quads1_table`,
`aac_signed_quads2_table`, `aac_signed_pairs5_table`, `aac_signed_pairs6_table`,
`aac_unsigned_quads3_table`, `aac_unsigned_quads4_table`, `aac_unsigned_pairs7_table`,
`aac_unsigned_pairs8_table`, `aac_unsigned_pairs9_table`,
`aac_unsigned_pairs10_table`, and `aac_escape_table`, with the older unit-magnitude subset helper retained for
diagnostics. Core-owned unit fixtures for codebook 6 and quad codebooks 1/3
keep binding/package workbenches off duplicated ad hoc tables. The public bit-cost section planner can select codebook 7/8/9/10
by default, direct signed quad codebook 1/2 and direct signed pair codebook 5/6
in the standard-id workbench, and codebook 11 when an explicit escape table is
supplied. Quad codebook 1-4 section planning is available as a caller-table
workbench and remains separate from production encode while full standard
table-set integration is still pending. A standard
codebook-id section planner can also combine the standard signed-quad,
unsigned-quad, signed-pair, unsigned-pair, and escape codebooks, or
caller-supplied quad codebooks 1-4 with pair/escape codebooks 5-11, for future full-codebook integration, including
split helpers that hand section plus scale-factor bits and spectral bits to the
individual-channel payload packer separately, plus bit-cost planning entry
points for those scale-factor-bearing split payloads. A dedicated standard
AAC-LC workbench helper plans the currently implemented standard pair/escape
tables with the unit quad fixture so bindings and readiness checks share the
same full-codebook integration point. The deterministic
planner now routes default magnitude-classified non-zero sections through the
available standard unsigned-pairs codebook 7 table for magnitudes up to 7 and
codebook 9 for magnitudes up to 12, falling back to escape-class sections above
that range. The standard-id offsets ADTS stream workbench also has mono/stereo
fixed-step, max-frame, and bitrate-derived step search wrappers for diagnostic
framing and budget checks. Low-level spectral quadruple symbols, caller-supplied quad-table
section metadata, and sign-bit section payload helpers are available as the
workbench for standard codebooks 1-4. A standard AAC-LC spectral table-set helper that includes escape
codebook 11 is also available for diagnostics and future rate-control work, but
production encode keeps the current FFmpeg-oracle-passing table set until
escape-coded output is accepted by the readiness gate. The diagnostic auto-step
path remains experimental until the remaining standard signed/quad codebook
syntax is filled in. Broader non-silent production AAC encoding is still under
active development.
