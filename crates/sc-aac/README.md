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
standard unsigned-pairs codebook 7/8/9/10 tables and escape codebook 11 table are
exposed as `aac_unsigned_pairs7_table`, `aac_unsigned_pairs8_table`,
`aac_unsigned_pairs9_table`, `aac_unsigned_pairs10_table`, and
`aac_escape_table`, with the older unit-magnitude subset helper retained for
diagnostics. The public bit-cost section planner can select codebook 7/8/9/10
by default, codebook 6 when a caller-supplied signed-pair table is provided,
and codebook 11 when an explicit escape table is supplied. The deterministic
planner now routes default magnitude-classified non-zero sections through the
available standard unsigned-pairs codebook 7 table for magnitudes up to 7 and
codebook 9 for magnitudes up to 12, falling back to escape-class sections above
that range. Low-level spectral quadruple symbols, caller-supplied quad-table
section metadata, and sign-bit section payload helpers are available as the
workbench for standard codebooks 1-4. A standard AAC-LC spectral table-set helper that includes escape
codebook 11 is also available for diagnostics and future rate-control work, but
production encode keeps the current FFmpeg-oracle-passing table set until
escape-coded output is accepted by the readiness gate. The diagnostic auto-step
path remains experimental until the remaining standard signed/quad codebook
syntax is filled in. Broader non-silent production AAC encoding is still under
active development.
