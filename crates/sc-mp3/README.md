# sc-mp3

MP3 helper crate for the Sonare codec workspace.

The crate contains clean-room MPEG audio header handling and experimental
MPEG-1 Layer III frame construction helpers. Public encode support includes a
silent compact path and a non-silent long-block scaffold while the full encoder
is being completed. Non-silent encode uses the standard table provider with
frame-level quantizer step search, so it can emit accepted non-zero payload
scaffolds instead of falling back to the all-zero scaffold. The standard table
provider includes the MPEG-1 Layer III big-values table 1 for unit-magnitude
pairs, big-values table 2 for magnitude-two pairs, and count1 tables 32/33.
The PCM analysis scaffold now emits 576 coefficients per granule by applying
long-block windows across the full granule span. Step search evaluates all
candidates and can report payload bit length against frame capacity for the
future rate-control path. `layer3_header_for_capacity`,
`layer3_main_data_capacity_bytes`, and `layer3_main_data_capacity_bits` expose
the per-frame Layer III payload budget from a parsed or constructed header, and
the bitrate-selected stream helper derives the MPEG header and per-frame
capacity from a caller-selected Layer III bitrate. The remaining larger
big-values tables, true polyphase/hybrid analysis, psychoacoustic scale-factor
selection, and bit reservoir integration are still pending.
