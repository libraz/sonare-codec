# sc-mp3

MP3 helper crate for the Sonare codec workspace.

The crate contains clean-room MPEG audio header handling and experimental
MPEG-1 Layer III frame construction helpers. Public encode support includes a
silent compact path and a non-silent long-block scaffold while the full encoder
is being completed. Non-silent encode uses the standard table provider with
frame-level quantizer step search, so it can emit accepted non-zero payload
scaffolds instead of falling back to the all-zero scaffold. The standard table
provider includes the MPEG-1 Layer III big-values table 1 for unit-magnitude
pairs, big-values table 2 for magnitude-two pairs, tables 5/7/10/13, the
table-16 codeword tree used by escape-class tables 16..=23, and count1 tables
32/33.
The PCM analysis scaffold now emits 576 coefficients per granule by applying
long-block windows across the full granule span. The production-facing
quantizer uses the true polyphase + hybrid MDCT workbench for mono and keeps
stereo on the compatibility cosine-modulated subband scaffold while the stereo
true-polyphase path is brought up to the FFmpeg readiness oracle. Step search
evaluates all candidates and can report payload bit length against frame
capacity for the future rate-control path; the default non-silent auto-step
encode path also schedules CBR padding slots per frame, and the default
non-silent mono production path uses the CBR bit-reservoir packer. `layer3_header_for_capacity`,
`layer3_main_data_capacity_bytes`, and `layer3_main_data_capacity_bits` expose
the per-frame Layer III payload budget from a parsed or constructed header, and
the bitrate-selected stream helpers derive MPEG headers and per-frame capacity
from a caller-selected Layer III bitrate, including CBR and reservoir variants.
The clean-room psychoacoustic model is wired to a low-level long-block scale-factor selector and self-contained perceptual scale-factor frame/stream helpers with payload-budget step search, bitrate-derived frame capacity, and an allocation-active CBR selector that prefers fitting candidates with non-zero scale factors while analyzing zero-padded PCM and the sign-inverted hybrid MDCT spectrum for the scale-factor quantizer workbench; production encode keeps the calibrated global-gain path until rate-control integration is validated.
The remaining standard big-values tables, stereo true-polyphase readiness,
stereo reservoir promotion, production psychoacoustic bit allocation, and full
rate control are still pending.
