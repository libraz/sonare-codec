    use super::filterbank;
    use super::{
        apply_alias_reduction, apply_big_value_table_to_granule, apply_count1_table_to_granule,
        apply_frequency_inversion, apply_part2_3_length_to_granule,
        apply_scale_factor_compress_to_granule, apply_spectral_regions_to_granule,
        assemble_layer3_frame, assemble_layer3_frame_from_payloads,
        assemble_mpeg1_layer3_pcm_frame_with_perceptual_scale_factors_and_table_provider,
        assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors,
        assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider,
        big_value_pairs, count1_quads, crc16_mpeg_audio, decode, encode,
        encode_mpeg1_layer3_pcm_frames_with_auto_step_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_bitrate_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_cbr_bitrate_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_auto_step_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_max_payload_bits_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_auto_step_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_max_payload_bits_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_perceptual_scale_factors_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors,
        encode_mpeg1_layer3_pcm_frames_with_header_and_selected_scale_factors_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_max_payload_bits_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_perceptual_active_cbr_bitrate_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_perceptual_auto_step_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_perceptual_bitrate_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_perceptual_cbr_bitrate_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_perceptual_max_payload_bits_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_perceptual_reservoir_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_perceptual_scale_factors_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_reservoir_and_table_provider,
        encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors,
        encode_mpeg1_layer3_pcm_frames_with_selected_scale_factors_and_table_provider,
        experimental_unit_magnitude_table_provider, layer3_analysis_subband_block,
        layer3_header_for_capacity, layer3_long_block_spectrum, layer3_long_scalefactor_band_index,
        layer3_long_scalefactor_band_range, layer3_main_data_capacity_bits,
        layer3_short_reorder_map, layer3_short_scalefactor_band_index,
        layer3_short_scalefactor_band_range,
        build_layer3_block_schedule, layer3_granule_is_transient, layer3_main_data_capacity_bytes,
        mdct_long_block, mdct_short_block, mpeg1_layer3_global_gain_for_step, Layer3BlockType,
        mpeg1_layer3_long_scalefactor_band_index, mpeg1_layer3_long_scalefactor_band_range,
        mpeg1_layer3_quality_guard_candidate_is_better,
        mpeg1_layer3_standard_big_value_table_provider, mpeg1_layer3_standard_table_provider,
        pack_big_value_pairs_with_linbits, pack_big_value_pairs_with_region_tables_and_provider,
        pack_big_value_pairs_with_sign_bits, pack_big_value_pairs_with_table,
        pack_count1_quads_with_sign_bits, pack_count1_quads_with_table,
        pack_layer3_main_data_payloads, pack_main_data_codewords,
        pack_main_data_codewords_for_granule, pack_main_data_codewords_with_len,
        pack_main_data_parts, pack_main_data_parts_for_granule, pack_main_data_regions,
        pack_main_data_regions_for_granule, pack_mpeg1_layer3_long_quantized_spectrum_for_granule,
        pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_and_table_provider,
        pack_mpeg1_layer3_long_quantized_spectrum_with_selected_scale_factors_for_granule,
        pack_mpeg1_layer3_long_quantized_spectrum_with_table_provider,
        pack_mpeg1_layer3_long_scale_factors, pack_mpeg1_layer3_long_scale_factors_for_granule,
        pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_and_table_provider,
        pack_mpeg1_layer3_pcm_long_block_with_calibrated_gain_for_granule,
        pack_mpeg1_layer3_pcm_long_block_with_perceptual_scale_factors_and_table_provider,
        pack_mpeg1_layer3_pcm_long_block_with_perceptual_scalefac_scale_and_table_provider,
        pack_mpeg1_layer3_short_quantized_spectrum_with_table_provider,
        pack_mpeg1_layer3_short_scale_factors, pack_mpeg2_layer3_lsf_long_scale_factors,
        pack_mpeg2_layer3_lsf_long_scale_factors_for_granule, pack_quantized_spectrum_for_granule,
        pack_quantized_spectrum_with_scale_factors_and_table_provider,
        pack_quantized_spectrum_with_scale_factors_for_granule,
        pack_quantized_spectrum_with_table_provider, plan_spectral_regions, quantize_long_block,
        quantize_mpeg1_layer3_long_spectrum_with_scalefactors,
        quantize_mpeg1_layer3_short_spectrum_with_scalefactors, quantize_pcm_long_block,
        select_big_value_region_tables, select_big_value_region_tables_by_bit_cost,
        select_big_value_table, select_big_value_table_by_bit_cost, select_count1_table,
        select_count1_table_by_bit_cost,
        select_mpeg1_layer3_first_frame_quality_guarded_candidate_profile_with_table_provider,
        select_mpeg1_layer3_long_scale_factor_compress,
        select_mpeg1_layer3_long_scale_factors_for_quantized_spectrum,
        select_mpeg1_layer3_short_scale_factor_compress,
        select_mpeg1_layer3_pcm_frame_perceptual_active_step_details_with_table_provider,
        select_mpeg1_layer3_pcm_frame_perceptual_active_step_with_table_provider,
        select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_max_payload_bits_and_table_provider,
        select_mpeg1_layer3_pcm_frame_perceptual_step_details_with_table_provider,
        select_mpeg1_layer3_pcm_frame_perceptual_step_with_max_payload_bits_and_table_provider,
        select_mpeg1_layer3_pcm_frame_perceptual_step_with_table_provider,
        select_mpeg1_layer3_pcm_frame_step_details_with_max_payload_bits_and_table_provider,
        select_mpeg1_layer3_pcm_frame_step_details_with_table_provider,
        select_mpeg1_layer3_pcm_frame_step_with_max_payload_bits_and_table_provider,
        select_mpeg1_layer3_pcm_frame_step_with_table_provider,
        select_mpeg1_layer3_perceptual_reservoir_frame_details_with_table_provider,
        select_mpeg1_layer3_quality_guarded_perceptual_reservoir_frame_details_with_table_provider,
        select_mpeg1_layer3_reservoir_frame_details_with_table_provider,
        select_mpeg2_layer3_lsf_long_scale_factor_compress, BitWriter, ChannelMode, FrameHeader,
        Layer, Layer3BigValueMagnitude, Layer3BigValuePair, Layer3BigValueRegionTableSelection,
        Layer3BigValueTableSelection, Layer3Count1MagnitudeQuad, Layer3Count1Quad,
        Layer3Count1TableSelection, Layer3EntropyTableProvider, Layer3EntropyTables,
        Layer3GranuleChannelInfo, Layer3PcmFrameStepSelection,
        Layer3QualityGuardPerceptualCandidate, Layer3ScaleFactorCompress, Layer3SideInfo,
        Layer3SpectralRegions, Layer3WindowSwitching, MpegVersion, ALIAS_REDUCTION_C,
        LONG_BLOCK_GRANULE_SAMPLES, MPEG1_LAYER3_LONG_SCALEFACTOR_BAND_BOUNDARIES,
        MPEG1_LAYER3_LONG_SCALE_FACTOR_COUNT,
        MPEG1_LAYER3_MISSING_STANDARD_BIG_VALUE_TABLE_SELECTS, MPEG1_LAYER3_PCM_STEP_CANDIDATES,
        MPEG1_LAYER3_STANDARD_BIG_VALUE_TABLE_SELECTS, MPEG1_LAYER3_STANDARD_COUNT1_TABLE_SELECTS,
    };
    use sc_core::{
        detect, quantize_spectrum, AudioBuffer, BitReader, Error, Format, HuffmanCode,
        HuffmanEntry, PackedBits,
    };

    /// Inverse of the `sc-core` (unnormalized) MDCT used by `mdct_long_block`:
    /// `x[m] = (2/N) sum_k X[k] cos[(pi/N)(m + 0.5 + N/2)(k + 0.5)]`, N = 18.
    fn ctrl_imdct_36(lines: &[f32]) -> [f32; 36] {
        let n = LONG_BLOCK_GRANULE_SAMPLES;
        let mut out = [0.0_f32; 36];
        for (m, o) in out.iter_mut().enumerate() {
            let mut acc = 0.0_f64;
            for (k, &x) in lines.iter().enumerate() {
                let angle = std::f64::consts::PI / n as f64
                    * (m as f64 + 0.5 + n as f64 / 2.0)
                    * (k as f64 + 0.5);
                acc += f64::from(x) * angle.cos();
            }
            *o = (2.0 / n as f64 * acc) as f32;
        }
        out
    }

    fn ctrl_sine_window_36() -> [f32; 36] {
        let mut w = [0.0_f32; 36];
        for (i, wi) in w.iter_mut().enumerate() {
            *wi = (std::f32::consts::PI / 36.0 * (i as f32 + 0.5)).sin();
        }
        w
    }

    /// Decoder-side alias reduction: the exact inverse of `apply_alias_reduction`.
    fn ctrl_alias_reduce(spectrum: &mut [f32]) {
        for boundary in 0..(filterbank::SUBBANDS - 1) {
            let upper_base =
                boundary * LONG_BLOCK_GRANULE_SAMPLES + (LONG_BLOCK_GRANULE_SAMPLES - 1);
            let lower_base = (boundary + 1) * LONG_BLOCK_GRANULE_SAMPLES;
            for (i, &c) in ALIAS_REDUCTION_C.iter().enumerate() {
                let cs = 1.0 / (1.0 + c * c).sqrt();
                let ca = c / (1.0 + c * c).sqrt();
                let upper = upper_base - i;
                let lower = lower_base + i;
                let a = spectrum[upper];
                let b = spectrum[lower];
                spectrum[upper] = a * cs - b * ca;
                spectrum[lower] = b * cs + a * ca;
            }
        }
    }

    /// ISO/IEC 11172-3 polyphase synthesis filterbank (decoder), used only as a
    /// controlled oracle so the full encoder chain can be inverted in-process.
    struct CtrlSynth {
        v: Vec<f32>,
    }

    impl CtrlSynth {
        fn new() -> Self {
            Self { v: vec![0.0; 1024] }
        }

        fn step(&mut self, s: &[f32; filterbank::SUBBANDS]) -> [f32; filterbank::SUBBANDS] {
            self.v.rotate_right(64);
            for i in 0..64 {
                let mut acc = 0.0_f32;
                for (k, sk) in s.iter().enumerate() {
                    let angle =
                        (16.0 + i as f64) * (2.0 * k as f64 + 1.0) * std::f64::consts::PI / 64.0;
                    acc += angle.cos() as f32 * *sk;
                }
                self.v[i] = acc;
            }
            let mut u = [0.0_f32; filterbank::WINDOW_LEN];
            for i in 0..8 {
                for j in 0..32 {
                    u[i * 64 + j] = self.v[i * 128 + j];
                    u[i * 64 + 32 + j] = self.v[i * 128 + 96 + j];
                }
            }
            let mut out = [0.0_f32; filterbank::SUBBANDS];
            for (j, oj) in out.iter_mut().enumerate() {
                let mut acc = 0.0_f32;
                for i in 0..16 {
                    acc += u[j + 32 * i] * filterbank::SYNTHESIS_WINDOW_D[j + 32 * i];
                }
                *oj = acc;
            }
            out
        }
    }

    fn ctrl_corr(a: &[f32], b: &[f32]) -> f64 {
        let n = a.len().min(b.len());
        if n == 0 {
            return 0.0;
        }
        let ma = a[..n].iter().map(|x| f64::from(*x)).sum::<f64>() / n as f64;
        let mb = b[..n].iter().map(|x| f64::from(*x)).sum::<f64>() / n as f64;
        let (mut num, mut da, mut db) = (0.0_f64, 0.0_f64, 0.0_f64);
        for i in 0..n {
            let x = f64::from(a[i]) - ma;
            let y = f64::from(b[i]) - mb;
            num += x * y;
            da += x * x;
            db += y * y;
        }
        if da == 0.0 || db == 0.0 {
            0.0
        } else {
            num / (da.sqrt() * db.sqrt())
        }
    }

    /// Runs the full Layer III long-block encoder chain
    /// (`layer3_long_block_spectrum`) and inverts it with a controlled
    /// spec-complete decoder. If the encoder is the exact inverse of the standard
    /// decoder, this reconstructs the input sweep; a low correlation localizes the
    /// bug inside our encoder chain rather than in Symphonia's conventions.
    #[test]
    fn controlled_full_chain_reconstructs_sweep() {
        let sample_rate = 44_100_u32;
        let total = 22_050_usize;
        let input: Vec<f32> = (0..total)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let f = 300.0 + 5_700.0 * (i as f32 / total as f32);
                0.5 * (std::f32::consts::TAU * f * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(sample_rate, 1, input.clone()).unwrap();

        let granules = total / 576;
        let win = ctrl_sine_window_36();
        let mut prev_tail = vec![[0.0_f32; LONG_BLOCK_GRANULE_SAMPLES]; filterbank::SUBBANDS];
        let mut synth = CtrlSynth::new();
        let mut out = Vec::<f32>::with_capacity(granules * 576);

        for g in 0..granules {
            let mut spectrum = layer3_long_block_spectrum(&pcm, 0, g * 576).unwrap();
            ctrl_alias_reduce(&mut spectrum);

            let mut hops = [[0.0_f32; filterbank::SUBBANDS]; LONG_BLOCK_GRANULE_SAMPLES];
            for sb in 0..filterbank::SUBBANDS {
                let lines = &spectrum[sb * 18..sb * 18 + 18];
                let im = ctrl_imdct_36(lines);
                let mut cur = [0.0_f32; LONG_BLOCK_GRANULE_SAMPLES];
                for i in 0..LONG_BLOCK_GRANULE_SAMPLES {
                    cur[i] = im[i] * win[i] + prev_tail[sb][i];
                    prev_tail[sb][i] = im[i + 18] * win[i + 18];
                }
                apply_frequency_inversion(sb, &mut cur);
                for h in 0..LONG_BLOCK_GRANULE_SAMPLES {
                    hops[h][sb] = cur[h];
                }
            }
            for hop in &hops {
                out.extend_from_slice(&synth.step(hop));
            }
        }

        // Lag-scan to absorb the filterbank + overlap reconstruction delay.
        let seg = 8_192_usize;
        let ref_start = 6_000_usize;
        let reference = &input[ref_start..ref_start + seg];
        let mut best = (0_usize, f64::NEG_INFINITY);
        for d in 0..2_000_usize {
            let start = ref_start + d;
            if start + seg > out.len() {
                break;
            }
            let c = ctrl_corr(reference, &out[start..start + seg]);
            if c > best.1 {
                best = (d, c);
            }
        }
        let aligned = &out[ref_start + best.0..ref_start + best.0 + seg];
        let in_rms = (reference.iter().map(|x| x * x).sum::<f32>() / seg as f32).sqrt();
        let out_rms = (aligned.iter().map(|x| x * x).sum::<f32>() / seg as f32).sqrt();
        println!(
            "controlled full chain: delay={} corr={:.4} ratio={:.4}",
            best.0,
            best.1,
            out_rms / in_rms
        );
        assert!(
            best.1 > 0.9,
            "encoder chain is not the inverse of the standard decoder: corr={:.4}",
            best.1
        );
    }

    /// Like `controlled_full_chain_reconstructs_sweep`, but routes the spectrum
    /// through the real quantizer and the calibrated `global_gain` requantization
    /// (`xr = sign * |is|^(4/3) * 2^((gg-210)/4)`, zero scalefactors) before
    /// decoding. If this still reconstructs, the spectral *values* the decoder
    /// should receive are correct, so any end-to-end failure is in the bitstream
    /// packing/side-info, not the DSP or the gain calibration.
    #[test]
    fn controlled_chain_survives_quantization() {
        let sample_rate = 44_100_u32;
        let total = 22_050_usize;
        let input: Vec<f32> = (0..total)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let f = 300.0 + 5_700.0 * (i as f32 / total as f32);
                0.5 * (std::f32::consts::TAU * f * t).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(sample_rate, 1, input.clone()).unwrap();

        let step = 0.05_f32;
        let gg = mpeg1_layer3_global_gain_for_step(step);
        let gain = 2.0_f32.powf((f32::from(gg) - 210.0) / 4.0);

        let granules = total / 576;
        let win = ctrl_sine_window_36();
        let mut prev_tail = vec![[0.0_f32; LONG_BLOCK_GRANULE_SAMPLES]; filterbank::SUBBANDS];
        let mut synth = CtrlSynth::new();
        let mut out = Vec::<f32>::with_capacity(granules * 576);

        for g in 0..granules {
            let spectrum = layer3_long_block_spectrum(&pcm, 0, g * 576).unwrap();
            let is = quantize_spectrum(&spectrum, step, 8191).unwrap();
            // Calibrated requantization, exactly as a spec decoder would apply it.
            let mut xr: Vec<f32> = is
                .iter()
                .map(|&q| {
                    let mag = (q.unsigned_abs() as f32).powf(4.0 / 3.0) * gain;
                    if q < 0 {
                        -mag
                    } else {
                        mag
                    }
                })
                .collect();
            ctrl_alias_reduce(&mut xr);

            let mut hops = [[0.0_f32; filterbank::SUBBANDS]; LONG_BLOCK_GRANULE_SAMPLES];
            for sb in 0..filterbank::SUBBANDS {
                let lines = &xr[sb * 18..sb * 18 + 18];
                let im = ctrl_imdct_36(lines);
                let mut cur = [0.0_f32; LONG_BLOCK_GRANULE_SAMPLES];
                for i in 0..LONG_BLOCK_GRANULE_SAMPLES {
                    cur[i] = im[i] * win[i] + prev_tail[sb][i];
                    prev_tail[sb][i] = im[i + 18] * win[i + 18];
                }
                apply_frequency_inversion(sb, &mut cur);
                for h in 0..LONG_BLOCK_GRANULE_SAMPLES {
                    hops[h][sb] = cur[h];
                }
            }
            for hop in &hops {
                out.extend_from_slice(&synth.step(hop));
            }
        }

        let seg = 8_192_usize;
        let ref_start = 6_000_usize;
        let reference = &input[ref_start..ref_start + seg];
        let mut best = (0_usize, f64::NEG_INFINITY);
        for d in 0..2_000_usize {
            let start = ref_start + d;
            if start + seg > out.len() {
                break;
            }
            let c = ctrl_corr(reference, &out[start..start + seg]);
            if c > best.1 {
                best = (d, c);
            }
        }
        let aligned = &out[ref_start + best.0..ref_start + best.0 + seg];
        let in_rms = (reference.iter().map(|x| x * x).sum::<f32>() / seg as f32).sqrt();
        let out_rms = (aligned.iter().map(|x| x * x).sum::<f32>() / seg as f32).sqrt();
        println!(
            "controlled chain (quantized): delay={} corr={:.4} ratio={:.4}",
            best.0,
            best.1,
            out_rms / in_rms
        );
        assert!(
            best.1 > 0.9,
            "quantize+requant path lost the signal: corr={:.4}",
            best.1
        );
    }

    #[ignore = "diagnostic: report auto-selected quantizer step and bit usage"]
    #[test]
    fn diagnostic_reports_step_and_bit_usage() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0xc0]).unwrap();
        let sample_rate = 44_100_u32;
        let samples: Vec<f32> = (0..22_050)
            .map(|i| {
                0.5 * (std::f32::consts::TAU * 2_000.0 * (i as f32 / sample_rate as f32)).sin()
            })
            .collect();
        let pcm = AudioBuffer::new(sample_rate, 1, samples).unwrap();
        let provider = mpeg1_layer3_standard_table_provider();
        let start = usize::from(header.samples_per_frame());
        for &step in MPEG1_LAYER3_PCM_STEP_CANDIDATES {
            let q = quantize_pcm_long_block(&pcm, 0, start, step);
            let max_is = q
                .as_ref()
                .map(|v| v.iter().map(|x| x.unsigned_abs()).max().unwrap_or(0))
                .unwrap_or(0);
            let pack = q.as_ref().ok().map(|quantized| {
                let mut g = Layer3GranuleChannelInfo::default();
                pack_quantized_spectrum_with_table_provider(&mut g, quantized, provider)
                    .map(|p| p.bit_len)
                    .map_err(|e| format!("{e:?}"))
            });
            println!(
                "  step={step:>9} quant_ok={} max_is={max_is} pack={pack:?}",
                q.is_ok(),
            );
        }
        let sel = select_mpeg1_layer3_pcm_frame_step_details_with_table_provider(
            header,
            &pcm,
            start,
            MPEG1_LAYER3_PCM_STEP_CANDIDATES,
            provider,
        )
        .unwrap();
        println!(
            "selected step={} payload_bits={}",
            sel.step, sel.payload_bit_len
        );
    }

    #[test]
    fn parses_mpeg1_layer3_header() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x64]).unwrap();

        assert_eq!(header.version, MpegVersion::Mpeg1);
        assert_eq!(header.layer, Layer::Layer3);
        assert!(header.protection_absent);
        assert_eq!(header.bitrate_kbps, 128);
        assert_eq!(header.sample_rate, 44_100);
        assert!(!header.padding);
        assert_eq!(header.channel_mode, ChannelMode::JointStereo);
        assert_eq!(header.samples_per_frame(), 1152);
        assert_eq!(header.frame_len(), 417);
        assert_eq!(header.channel_count(), 2);
        assert_eq!(header.layer3_granule_count(), 2);
        assert_eq!(header.layer3_side_info_len(), Some(32));
        assert_eq!(layer3_main_data_capacity_bytes(header).unwrap(), 381);
        assert_eq!(layer3_main_data_capacity_bits(header).unwrap(), 3048);
    }

    #[test]
    fn parses_mpeg2_layer3_padded_header() {
        let header = FrameHeader::parse(&[0xff, 0xf3, 0x82, 0xc0]).unwrap();

        assert_eq!(header.version, MpegVersion::Mpeg2);
        assert_eq!(header.layer, Layer::Layer3);
        assert_eq!(header.bitrate_kbps, 64);
        assert_eq!(header.sample_rate, 22_050);
        assert!(header.padding);
        assert_eq!(header.channel_mode, ChannelMode::SingleChannel);
        assert_eq!(header.samples_per_frame(), 576);
        assert_eq!(header.frame_len(), 209);
        assert_eq!(header.channel_count(), 1);
        assert_eq!(header.layer3_granule_count(), 1);
        assert_eq!(header.layer3_side_info_len(), Some(9));
        assert_eq!(layer3_main_data_capacity_bytes(header).unwrap(), 196);
        assert_eq!(layer3_main_data_capacity_bits(header).unwrap(), 1568);
    }

    #[test]
    fn builds_layer3_capacity_headers() {
        let mono = layer3_header_for_capacity(44_100, 1, 128, false, false).unwrap();
        let stereo = layer3_header_for_capacity(44_100, 2, 128, false, false).unwrap();
        let mpeg2 = layer3_header_for_capacity(22_050, 1, 64, true, false).unwrap();

        assert_eq!(mono.version, MpegVersion::Mpeg1);
        assert_eq!(mono.channel_mode, ChannelMode::SingleChannel);
        assert_eq!(layer3_main_data_capacity_bytes(mono).unwrap(), 396);
        assert_eq!(layer3_main_data_capacity_bits(mono).unwrap(), 3168);
        assert_eq!(stereo.version, MpegVersion::Mpeg1);
        assert_eq!(stereo.channel_mode, ChannelMode::Stereo);
        assert_eq!(layer3_main_data_capacity_bytes(stereo).unwrap(), 381);
        assert_eq!(mpeg2.version, MpegVersion::Mpeg2);
        assert_eq!(layer3_main_data_capacity_bytes(mpeg2).unwrap(), 196);
        assert!(layer3_header_for_capacity(44_100, 3, 128, false, false).is_err());
        assert!(layer3_header_for_capacity(44_100, 1, 123, false, false).is_err());
    }

    #[test]
    fn serializes_header_roundtrip() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::JointStereo,
        };

        let bytes = header.to_bytes().unwrap();

        assert_eq!(FrameHeader::parse(&bytes).unwrap(), header);
    }

    #[test]
    fn rejects_reserved_header_fields() {
        let err = FrameHeader::parse(&[0xff, 0xfb, 0x00, 0x00]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("invalid MP3 bitrate index")
        ));

        let err = FrameHeader::parse(&[0xff, 0xfb, 0x9c, 0x00]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("reserved MP3 sample-rate index")
        ));
    }

    #[test]
    fn rejects_unsupported_serialized_values() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 123,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::Stereo,
        };

        let err = header.to_bytes().unwrap_err();

        assert!(matches!(err, Error::UnsupportedFeature("MP3 bitrate")));
    }

    #[test]
    fn bit_writer_writes_msb_first_and_pads_last_byte() {
        let mut writer = BitWriter::new();

        writer.write_bits(0b101, 3).unwrap();
        writer.write_bits(0b10, 2).unwrap();

        assert_eq!(writer.bit_len(), 5);
        assert_eq!(writer.finish_byte_aligned(), &[0b1011_0000]);
    }

    #[test]
    fn bit_writer_writes_bytes_across_unaligned_position() {
        let mut writer = BitWriter::new();

        writer.write_bits(0b1, 1).unwrap();
        writer.write_bytes(&[0b0101_0101]).unwrap();

        assert_eq!(writer.bit_len(), 9);
        assert_eq!(writer.finish_byte_aligned(), &[0b1010_1010, 0b1000_0000]);
    }

    #[test]
    fn crc16_mpeg_audio_is_stable_for_known_header_bits() {
        assert_eq!(crc16_mpeg_audio(&[]), 0xffff);
        assert_eq!(crc16_mpeg_audio(&[0xfb, 0x90, 0x64]), 0xe30d);
    }

    #[test]
    fn packs_mpeg1_stereo_silent_side_info() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x00]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);

        let packed = side_info.pack(&header).unwrap();

        assert_eq!(packed.len(), 32);
        assert_eq!(&packed[..4], &[0x00, 0x00, 0x00, 0x00]);
        assert!(packed.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn packs_mpeg2_mono_silent_side_info() {
        let header = FrameHeader::parse(&[0xff, 0xf3, 0x80, 0xc0]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);

        let packed = side_info.pack(&header).unwrap();

        assert_eq!(packed.len(), 9);
        assert_eq!(&packed[..3], &[0x00, 0x00, 0x00]);
        assert!(packed.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn packs_window_switching_side_info() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0xc0]).unwrap();
        let mut side_info = Layer3SideInfo::silent(&header);
        side_info.granules[0][0] = Layer3GranuleChannelInfo {
            part2_3_length: 3,
            big_values: 2,
            global_gain: 210,
            scalefac_compress: 5,
            window_switching: Some(Layer3WindowSwitching {
                block_type: 2,
                mixed_block_flag: true,
                table_select: [1, 2],
                subblock_gain: [3, 4, 5],
            }),
            table_select: [0; 3],
            region0_count: 0,
            region1_count: 0,
            preflag: true,
            scalefac_scale: true,
            count1table_select: true,
        };

        let packed = side_info.pack(&header).unwrap();

        assert_eq!(packed.len(), 17);
        assert_ne!(
            packed,
            Layer3SideInfo::silent(&header).pack(&header).unwrap()
        );
    }

    #[test]
    fn rejects_side_info_for_non_layer3() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer2,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::Stereo,
        };

        let err = Layer3SideInfo::silent(&header).pack(&header).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 side info requires Layer III")
        ));
    }

    #[test]
    fn assembles_layer3_frame_without_crc() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x00]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);
        let main_data = [0xaa, 0xbb, 0xcc];

        let frame = assemble_layer3_frame(header, &side_info, &main_data).unwrap();

        assert_eq!(frame.len(), header.frame_len());
        assert_eq!(&frame[..4], &header.to_bytes().unwrap());
        assert_eq!(
            &frame[4..4 + header.layer3_side_info_len().unwrap()],
            side_info.pack(&header).unwrap()
        );
        assert_eq!(
            &frame[4 + header.layer3_side_info_len().unwrap()
                ..4 + header.layer3_side_info_len().unwrap() + main_data.len()],
            main_data
        );
        assert!(
            frame[4 + header.layer3_side_info_len().unwrap() + main_data.len()..]
                .iter()
                .all(|byte| *byte == 0)
        );
    }

    #[test]
    fn assembles_layer3_frame_with_crc() {
        let mut header = FrameHeader::parse(&[0xff, 0xfa, 0x90, 0xc0]).unwrap();
        header.protection_absent = false;
        let side_info = Layer3SideInfo::silent(&header);

        let frame = assemble_layer3_frame(header, &side_info, &[]).unwrap();
        let expected_crc = {
            let mut crc_input = Vec::new();
            crc_input.extend_from_slice(&header.to_bytes().unwrap()[1..]);
            crc_input.extend_from_slice(&side_info.pack(&header).unwrap());
            crc16_mpeg_audio(&crc_input)
        };

        assert_eq!(frame.len(), header.frame_len());
        assert_eq!(&frame[..4], &header.to_bytes().unwrap());
        assert_eq!(&frame[4..6], &expected_crc.to_be_bytes());
        assert_eq!(
            &frame[6..6 + header.layer3_side_info_len().unwrap()],
            side_info.pack(&header).unwrap()
        );
    }

    #[test]
    fn assembles_layer3_frame_from_granule_payloads() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x00]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);
        let payloads = [
            PackedBits {
                bytes: vec![0b1000_0000],
                bit_len: 1,
            },
            PackedBits {
                bytes: vec![0b0100_0000],
                bit_len: 2,
            },
            PackedBits {
                bytes: vec![0b1110_0000],
                bit_len: 3,
            },
            PackedBits {
                bytes: vec![],
                bit_len: 0,
            },
        ];

        let packed = pack_layer3_main_data_payloads(&header, &payloads).unwrap();
        assert_eq!(
            packed,
            PackedBits {
                bytes: vec![0b1011_1100],
                bit_len: 6,
            }
        );

        let frame = assemble_layer3_frame_from_payloads(header, &side_info, &payloads).unwrap();
        let main_data_start = 4 + header.layer3_side_info_len().unwrap();
        assert_eq!(frame[main_data_start], 0b1011_1100);
        assert!(frame[main_data_start + 1..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn assembles_mpeg1_layer3_pcm_frame_with_selected_scale_factors() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::Stereo,
        };
        let pcm = AudioBuffer::new(
            44_100,
            2,
            vec![0.0; usize::from(header.samples_per_frame()) * 2],
        )
        .unwrap();
        let expected =
            assemble_layer3_frame(header, &Layer3SideInfo::silent(&header), &[]).unwrap();

        let frame = assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors(
            header,
            &pcm,
            0,
            1.0,
            Layer3EntropyTables {
                big_values: &[],
                count1: &[],
            },
        )
        .unwrap();
        let provider_frame =
            assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
                header,
                &pcm,
                0,
                1.0,
                Layer3EntropyTableProvider::default(),
            )
            .unwrap();

        assert_eq!(frame, expected);
        assert_eq!(provider_frame, expected);
    }

    #[test]
    fn rejects_mpeg1_layer3_pcm_frame_shape_mismatch() {
        let header = FrameHeader {
            version: MpegVersion::Mpeg1,
            layer: Layer::Layer3,
            protection_absent: true,
            bitrate_kbps: 128,
            sample_rate: 44_100,
            padding: false,
            channel_mode: ChannelMode::Stereo,
        };
        let pcm = AudioBuffer::new(
            48_000,
            2,
            vec![0.0; usize::from(header.samples_per_frame()) * 2],
        )
        .unwrap();

        let err = assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors(
            header,
            &pcm,
            0,
            1.0,
            Layer3EntropyTables {
                big_values: &[],
                count1: &[],
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 header sample rate does not match PCM")
        ));

        // MPEG-2 LSF is now supported; MPEG-2.5 (ISO-unspecified) is rejected by
        // the shared payload preparation regardless of sample rate.
        let mpeg25 = FrameHeader {
            version: MpegVersion::Mpeg25,
            ..header
        };
        let err = assemble_mpeg1_layer3_pcm_frame_with_selected_scale_factors_and_table_provider(
            mpeg25,
            &pcm,
            0,
            1.0,
            Layer3EntropyTableProvider::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature(
                "MP3 PCM frame payload currently requires MPEG-1 or MPEG-2 LSF Layer III"
            )
        ));
    }

    #[test]
    fn rejects_layer3_payload_count_mismatch() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x90, 0x00]).unwrap();
        let err = pack_layer3_main_data_payloads(&header, &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput("MP3 main data payload count does not match header")
        ));

        let non_layer3 = FrameHeader {
            layer: Layer::Layer2,
            ..header
        };
        let err = pack_layer3_main_data_payloads(&non_layer3, &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedFeature("MP3 main data requires Layer III")
        ));
    }

    #[test]
    fn rejects_main_data_that_exceeds_frame_capacity() {
        let header = FrameHeader::parse(&[0xff, 0xfb, 0x10, 0xc0]).unwrap();
        let side_info = Layer3SideInfo::silent(&header);
        let main_data = vec![0xff; header.frame_len()];

        let err = assemble_layer3_frame(header, &side_info, &main_data).unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidInput("MP3 main data exceeds frame capacity")
        ));
    }

