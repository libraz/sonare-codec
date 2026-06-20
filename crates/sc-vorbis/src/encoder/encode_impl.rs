use super::*;

/// A pure-Rust Vorbis encoder for one channel layout / sample rate.
pub struct VorbisEncoder {
    channels: u16,
    psy: PsyAnalysis,
    fitter: Floor1Fitter,
    floor: Floor1Encoding,
    short_psy: PsyAnalysis,
    short_fitter: Floor1Fitter,
    short_floor: Floor1Encoding,
    id_bytes: Vec<u8>,
    comment_bytes: Vec<u8>,
}

impl VorbisEncoder {
    /// Builds the encoder for `channels` at `sample_rate` Hz.
    #[must_use]
    pub fn new(channels: u16, sample_rate: u32) -> Self {
        let n = BLOCK_N;
        let m = (2 * n) as u32;
        let psy = PsyAnalysis::new(n, sample_rate);
        let fitter = Floor1Fitter::new(POSTLIST.to_vec(), Floor1FitInfo::standard());
        // Short-block analysis state, used when a transient switches a long grid
        // slot to short blocks. The floor books are shared with the long floor;
        // only the partition count and postlist differ.
        let short_psy = PsyAnalysis::new(SHORT_N, sample_rate);
        let short_fitter = Floor1Fitter::new(SHORT_POSTLIST.to_vec(), Floor1FitInfo::standard());

        // Packet-side floor coders: books indexed by global codebook number.
        // Index 0 is unused by the floor (it only references 1 and 2); a small
        // valid book keeps the indices aligned with the global list.
        let floor = floor_encoding(
            FLOOR_PARTITIONS,
            vec![complete_book(1), complete_book(4), complete_book(6)],
        );
        let short_floor = floor_encoding(
            FLOOR_PARTITIONS_SHORT,
            vec![complete_book(1), complete_book(4), complete_book(6)],
        );

        // Two block sizes: short (256) for transients, long (2048) for steady
        // content. Declared in the identification header so a blockflag-1 (long)
        // mode can carry the short/long window-overlap flags.
        let short_m = (2 * SHORT_N) as u32;
        let id_bytes = pack_identification_header(channels as u8, sample_rate, 0, 0, 0, short_m, m);
        let comment_bytes = pack_comment_header(b"sonare-codec", &[]);

        Self {
            channels,
            psy,
            fitter,
            floor,
            short_psy,
            short_fitter,
            short_floor,
            id_bytes,
            comment_bytes,
        }
    }

    /// Builds a packet-side floor coder over `partitions` partitions from
    /// length-fitted class-phrase and post-value books (the cascade structure is
    /// fixed). Mirrors the floor built in [`new`](Self::new) but with the
    /// per-stream fitted books; the long and short floors share these books and
    /// differ only in `partitions`.
    pub(crate) fn build_floor(
        partitions: usize,
        class_lengths: &[u8],
        value_lengths: &[u8],
    ) -> Floor1Encoding {
        floor_encoding(
            partitions,
            vec![
                complete_book(1),
                Codebook::new(class_lengths.to_vec()).unwrap_or_else(|| complete_book(4)),
                Codebook::new(value_lengths.to_vec()).unwrap_or_else(|| complete_book(6)),
            ],
        )
    }

    /// Builds the packet-side residue coder from the fitted value-book lengths.
    /// Three partition types classified by peak magnitude: type 0 codes nothing
    /// (empty), type 1 codes through the fine book alone (quiet), type 2 codes a
    /// coarse stage then refines with the fine book (loud tonal peaks). The
    /// coarse stage gives the cascade a wide dynamic range while the fine stage
    /// keeps full resolution near zero.
    pub(crate) fn build_residue(
        end: usize,
        coarse_lengths: &[u8],
        fine_lengths: &[u8],
    ) -> ResidueConfig {
        let fine = || {
            residue_value_book(fine_lengths, FINE_MIN, FINE_DELTA).unwrap_or_else(|| {
                VqBook::new(
                    complete_book(7),
                    1,
                    FINE_MIN,
                    FINE_DELTA,
                    false,
                    residue_quantlist(),
                )
            })
        };
        let coarse =
            residue_value_book(coarse_lengths, COARSE_MIN, COARSE_DELTA).unwrap_or_else(|| {
                VqBook::new(
                    complete_book(7),
                    1,
                    COARSE_MIN,
                    COARSE_DELTA,
                    false,
                    residue_quantlist(),
                )
            });
        ResidueConfig {
            begin: 0,
            end,
            grouping: GROUPING,
            partitions: 3,
            partitions_per_word: 1,
            stages: 2,
            // Type 0: no stages (skip). Type 1: fine stage only (bit 1). Type 2:
            // coarse stage (bit 0) then fine stage (bit 1).
            secondstages: vec![0b00, 0b10, 0b11],
            partbooks: vec![
                vec![None, None],
                vec![None, Some(fine())],
                vec![Some(coarse), Some(fine())],
            ],
            // max == 0 -> type 0 (skip); max <= FINE_ONLY_MAX -> type 1 (fine
            // only); louder -> type 2 (coarse + fine).
            classmetric1: vec![0, FINE_ONLY_MAX as i32, 0],
            classmetric2: vec![-1, -1, -1],
            // 3 partition types: type 0 (skip, the common case) gets the 1-bit
            // codeword, types 1 and 2 get 2 bits.
            phrasebook: Codebook::new(vec![1, 2, 2]).unwrap_or_else(|| complete_book(2)),
        }
    }

    /// Assembles the spec setup configuration (the serialized counterpart of the
    /// packet-side floor/residue coders), with the residue value book carrying
    /// the per-stream fitted codeword lengths.
    pub(crate) fn build_setup(
        channels: u16,
        coupled: bool,
        coarse_lengths: &[u8],
        fine_lengths: &[u8],
        floor_class_lengths: &[u8],
        floor_value_lengths: &[u8],
    ) -> SetupConfig {
        let codebooks = vec![
            // 0: residue classification book (3 partition types; type 0 = 1 bit).
            StaticCodebook {
                dim: 1,
                entries: 3,
                lengthlist: vec![1, 2, 2],
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 1: floor1 class-phrase book (16 entries), lengths fitted per stream.
            StaticCodebook {
                dim: 1,
                entries: 16,
                lengthlist: floor_class_lengths.to_vec(),
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 2: floor1 post-value book (64 entries), lengths fitted per stream.
            StaticCodebook {
                dim: 1,
                entries: 64,
                lengthlist: floor_value_lengths.to_vec(),
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            },
            // 3: residue fine-stage VQ book (128 entries, maptype 1, step 0.25).
            // The codeword lengths are fitted to the stream's residue.
            StaticCodebook {
                dim: 1,
                entries: RES_LEVELS as u32,
                lengthlist: fine_lengths.to_vec(),
                maptype: 1,
                q_min: float32_pack(FINE_MIN),
                q_delta: float32_pack(FINE_DELTA),
                q_quant: 7,
                q_sequencep: false,
                quantlist: (0..RES_LEVELS as i32).collect(),
            },
            // 4: residue coarse-stage VQ book (128 entries, maptype 1, step 4.0)
            // — the wide first stage for loud tonal partitions.
            StaticCodebook {
                dim: 1,
                entries: RES_LEVELS as u32,
                lengthlist: coarse_lengths.to_vec(),
                maptype: 1,
                q_min: float32_pack(COARSE_MIN),
                q_delta: float32_pack(COARSE_DELTA),
                q_quant: 7,
                q_sequencep: false,
                quantlist: (0..RES_LEVELS as i32).collect(),
            },
        ];

        // A type-1 floor differs between block sizes only in its partition count
        // and postlist; the class table and books are shared.
        let floor_setup = |partitions: usize, postlist: &[i32]| Floor1Setup {
            partition_class: vec![0; partitions],
            class_dim: vec![4],
            class_subs: vec![1],
            class_book: vec![BOOK_FLOOR_CLASS as u8],
            class_subbook: vec![vec![-1, BOOK_FLOOR_VALUE as i32]],
            mult: FLOOR_MULT as u8,
            postlist: postlist.iter().map(|&p| p as u32).collect(),
        };
        // The residue cascade is identical for both block sizes apart from its
        // spectral extent (`end`).
        let residue_setup = |end: u32| ResidueSetup {
            residue_type: 1,
            begin: 0,
            end,
            grouping: GROUPING as u32,
            groupbook: BOOK_GROUP as u8,
            // Per partition type: type 0 nothing, type 1 the fine stage, type 2
            // the coarse then fine stages. The booklist names the set-bit stages
            // in type-major, stage-minor order: [fine(t1), coarse(t2), fine(t2)].
            secondstages: vec![0b00, 0b10, 0b11],
            booklist: vec![
                BOOK_RES_FINE as u8,
                BOOK_RES_COARSE as u8,
                BOOK_RES_FINE as u8,
            ],
        };

        // Square-polar couple a stereo pair (channel 0 = magnitude, 1 = angle):
        // correlated content sends the angle residue to zero, which then skips.
        // Other channel counts stay independent (no coupling). Both mappings
        // (long submap 0, short submap 1) carry the same coupling.
        let mapping_setup = |floor_idx: u8, residue_idx: u8| {
            let (coupling_mag, coupling_ang) = if coupled {
                (vec![0u32], vec![1u32])
            } else {
                (vec![], vec![])
            };
            Mapping0Setup {
                submaps: 1,
                coupling_mag,
                coupling_ang,
                chmuxlist: vec![0; channels as usize],
                floorsubmap: vec![floor_idx],
                residuesubmap: vec![residue_idx],
            }
        };

        SetupConfig {
            channels,
            codebooks,
            // Floor 0 / residue 0 / mapping 0 = long block; 1 = short block.
            floors: vec![
                (1, floor_setup(FLOOR_PARTITIONS, &POSTLIST)),
                (1, floor_setup(FLOOR_PARTITIONS_SHORT, &SHORT_POSTLIST)),
            ],
            residues: vec![residue_setup(BLOCK_N as u32), residue_setup(SHORT_N as u32)],
            mappings: vec![(0, mapping_setup(0, 0)), (0, mapping_setup(1, 1))],
            // Mode 0 = long (blockflag 1, carries window-overlap flags), mode 1 =
            // short (blockflag 0). The mode number is one bit per audio packet.
            modes: vec![
                ModeSetup {
                    blockflag: true,
                    windowtype: 0,
                    transformtype: 0,
                    mapping: 0,
                },
                ModeSetup {
                    blockflag: false,
                    windowtype: 0,
                    transformtype: 0,
                    mapping: 1,
                },
            ],
        }
    }

    /// Encodes interleaved PCM into a complete Ogg Vorbis byte stream.
    #[must_use]
    pub fn encode(&self, pcm: &AudioBuffer) -> Vec<u8> {
        let channel_count = usize::from(self.channels);
        let n = BLOCK_N;
        let m = 2 * n;
        let half = n;
        let frames = pcm.frames();

        // De-interleave, with a half-block of priming pad in front and a tail pad
        // out to a whole number of hops (so every sample gets an overlap partner).
        let needed = frames + 2 * half;
        let padded_len = needed.div_ceil(half) * half;
        let mut planar = vec![vec![0.0f32; padded_len]; channel_count];
        for (f, frame) in pcm.samples.chunks_exact(channel_count).enumerate() {
            for (ch, &sample) in frame.iter().enumerate() {
                planar[ch][half + f] = sample;
            }
        }

        // The long grid: `block_count` long blocks centred at `(k+1)*half` with
        // 50% overlap. Detect a transient in each interior grid slot (the first
        // and last slot stay long so every transient run keeps both bracketing
        // long blocks), then schedule short blocks over the transient runs.
        let mut block_count = 0usize;
        while (block_count * half) + m <= padded_len {
            block_count += 1;
        }
        let mut transient = vec![false; block_count];
        if block_count >= 3 {
            for (k, flag) in transient
                .iter_mut()
                .enumerate()
                .take(block_count - 1)
                .skip(1)
            {
                let pos = k * half;
                *flag = planar
                    .iter()
                    .filter_map(|ch| ch.get(pos..pos + m))
                    .any(block_is_transient);
            }
        }
        let schedule = build_schedule(block_count, &transient);

        // Pass 1: analyze every scheduled block, building the per-channel plans
        // and the per-stage histograms the residue (and floor) books are fitted
        // to. Long and short blocks share the residue and floor codebooks, so
        // their entries histogram into the same counts.
        let (lo, hi) = low_high_neighbors(&POSTLIST);
        let (short_lo, short_hi) = low_high_neighbors(&SHORT_POSTLIST);
        // Couple a stereo pair only when the channels are correlated enough that
        // coupling improves (never regresses) fidelity; otherwise code them
        // independently. The choice is per stream, matching the setup header.
        let coupled = channel_count == 2 && channels_are_correlated(&planar[0], &planar[1]);
        let mut plans: Vec<Vec<Option<ChannelPlan>>> = Vec::with_capacity(schedule.len());
        let mut coarse_counts = [0u64; RES_LEVELS];
        let mut fine_counts = [0u64; RES_LEVELS];
        // Per-floor-book histograms of the entries each block's floor would code,
        // keyed by the floor coder's book index (the class-phrase book and the
        // post-value book), so both floor books can be length-fitted too.
        let mut floor_counts: Vec<Vec<u64>> = self
            .floor
            .books
            .iter()
            .map(|b| vec![0u64; b.entries()])
            .collect();

        for spec in &schedule {
            // Select the long or short analysis chain. A long block spans
            // `2*BLOCK_N` samples through a left/right transition window (the
            // symmetric long window when both neighbours are long); a short block
            // spans `2*SHORT_N` through the symmetric short window.
            let (psy, fitter, postlist, floor_hist, neigh_lo, neigh_hi, bins) = if spec.long {
                (
                    &self.psy,
                    &self.fitter,
                    &POSTLIST[..],
                    &self.floor,
                    &lo,
                    &hi,
                    BLOCK_N,
                )
            } else {
                (
                    &self.short_psy,
                    &self.short_fitter,
                    &SHORT_POSTLIST[..],
                    &self.short_floor,
                    &short_lo,
                    &short_hi,
                    SHORT_N,
                )
            };
            let window = if !spec.long {
                vorbis_window(2 * SHORT_N)
            } else if spec.lw && spec.nw {
                // Both neighbours long: the transition window reduces exactly to
                // the symmetric long window, so use it directly.
                vorbis_window(2 * BLOCK_N)
            } else {
                vorbis_window_lr(
                    2 * BLOCK_N,
                    if spec.lw { BLOCK_N } else { SHORT_N },
                    if spec.nw { BLOCK_N } else { SHORT_N },
                )
            };
            let start = spec.center - bins;

            let mut raw: Vec<Option<(Vec<i32>, Vec<f32>)>> = Vec::with_capacity(channel_count);
            for ch in &planar {
                let block = ch
                    .get(start..start + 2 * bins)
                    .and_then(|seg| analyze_block_windowed(psy, fitter, postlist, seg, &window));
                match block {
                    Some(b) => {
                        let mut posts = b.posts.clone();
                        let dev = encode_post_deviations(
                            postlist, &mut posts, neigh_lo, neigh_hi, QUANT_Q,
                        );
                        let mut residue = b.residue;
                        // AoTuV M1 companding: relatively compensate the residue
                        // against the noise floor on steady (long) blocks, where
                        // sustained near-floor energy is what costs bits. A short
                        // block codes a transient attack, so it is left untouched
                        // to preserve that energy.
                        if spec.long {
                            let gains = psy.m1_companding_gains(&b.logmdct);
                            if gains.len() == residue.len() {
                                for (r, g) in residue.iter_mut().zip(&gains) {
                                    *r *= g;
                                }
                            }
                        }
                        raw.push(Some((dev, residue)));
                    }
                    None => raw.push(None),
                }
            }

            // Forward-couple the stereo pair before the residue is snapped and
            // histogrammed, so the books are fitted to what is actually coded.
            if coupled {
                couple_stereo_block(&mut raw);
            }

            let mut plan: Vec<Option<ChannelPlan>> = Vec::with_capacity(channel_count);
            for entry in raw {
                match entry {
                    Some((dev, mut residue)) => {
                        floor_hist.histogram(&dev, &mut floor_counts);
                        snap_residue(&mut residue);
                        histogram_cascade(&residue, &mut coarse_counts, &mut fine_counts);
                        plan.push(Some(ChannelPlan { dev, residue }));
                    }
                    None => plan.push(None),
                }
            }
            plans.push(plan);
        }

        // Fit each residue book to its stage's histogram, then serialize the
        // setup header and build the matching packet-side residue coders.
        let coarse_lengths = fit_book_lengths(&coarse_counts, 7);
        let fine_lengths = fit_book_lengths(&fine_counts, 7);
        // Fit the two floor books to their histograms the same way, but only when
        // enough blocks amortize the setup-header cost; otherwise keep the compact
        // flat books (class 4-bit, post-value 6-bit). Histogramming used the
        // construction-time books, whose entry counts match the fitted ones, so
        // the classification — and thus the reconstruction — is unchanged.
        let (floor_class_lengths, floor_value_lengths) = if plans.len() >= FLOOR_FIT_MIN_BLOCKS {
            (
                fit_book_lengths(&floor_counts[BOOK_FLOOR_CLASS], 4),
                fit_book_lengths(&floor_counts[BOOK_FLOOR_VALUE], 6),
            )
        } else {
            (vec![4u8; 16], vec![6u8; 64])
        };
        // Long and short coders share the fitted books, differing only in the
        // floor partition count and the residue spectral extent.
        let long_floor =
            Self::build_floor(FLOOR_PARTITIONS, &floor_class_lengths, &floor_value_lengths);
        let short_floor = Self::build_floor(
            FLOOR_PARTITIONS_SHORT,
            &floor_class_lengths,
            &floor_value_lengths,
        );
        let long_residue = Self::build_residue(BLOCK_N, &coarse_lengths, &fine_lengths);
        let short_residue = Self::build_residue(SHORT_N, &coarse_lengths, &fine_lengths);
        let setup_bytes = Self::build_setup(
            self.channels,
            coupled,
            &coarse_lengths,
            &fine_lengths,
            &floor_class_lengths,
            &floor_value_lengths,
        )
        .pack();

        // Pass 2: emit each block's audio packet with the fitted coder, tagging
        // each with its end granule (sample) position. With 50% overlap the first
        // block only primes the overlap, so block `k` finalizes the samples up to
        // its centre (`centre - half`); the final block's granule is clamped to
        // the true input length so the decoder trims the tail padding to an exact
        // length. Centres advance monotonically across long and short blocks, so
        // the granules stay monotonic and the final clamp stays strictly above
        // the previous block's granule.
        let scheduled = plans.len();
        let mut audio: Vec<(Vec<u8>, u64)> = Vec::with_capacity(scheduled);
        for (k, (spec, plan)) in schedule.iter().zip(&plans).enumerate() {
            let granule = if k + 1 == scheduled {
                frames as u64
            } else {
                (spec.center - half) as u64
            };
            let (floor, residue) = if spec.long {
                (&long_floor, &long_residue)
            } else {
                (&short_floor, &short_residue)
            };
            audio.push((Self::write_packet(floor, residue, spec, plan), granule));
        }

        mux_vorbis(
            STREAM_SERIAL,
            &self.id_bytes,
            &self.comment_bytes,
            &setup_bytes,
            &audio,
        )
    }

    /// Writes one block's audio packet from its per-channel plans: the
    /// audio-packet bit, the mode (and, for a long block, its left/right
    /// window-overlap flags), then per-channel floor1, then the residue of the
    /// channels whose floor is present. `floor`/`residue` are the per-stream
    /// length-fitted coders for this block's size.
    pub(crate) fn write_packet(
        floor: &Floor1Encoding,
        residue: &ResidueConfig,
        spec: &BlockSpec,
        plan: &[Option<ChannelPlan>],
    ) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.write(0, 1); // audio packet (not a header)
                       // Two modes, so the mode number is one bit. Mode 0 is the
                       // long block (blockflag 1) and carries the left/right
                       // window-overlap flags; mode 1 is the short block
                       // (blockflag 0), which has no window flags.
        if spec.long {
            w.write(0, 1); // mode number 0 (long)
            w.write(u32::from(spec.lw), 1); // previous-window flag
            w.write(u32::from(spec.nw), 1); // next-window flag
        } else {
            w.write(1, 1); // mode number 1 (short)
        }

        let mut active: Vec<Vec<f32>> = Vec::new();
        for channel in plan {
            match channel {
                Some(cp) => {
                    floor.pack(&cp.dev, &mut w);
                    active.push(cp.residue.clone());
                }
                None => {
                    // Floor unused for this channel: clear the floor's present
                    // flag; this channel's residue is not coded.
                    w.write(0, 1);
                }
            }
        }
        // The submap codes the residue of exactly the present-floor channels.
        if !active.is_empty() {
            residue.encode(&active, &mut w);
        }

        w.into_bytes()
    }
}
