use super::*;

/// `tapset_icdf`: the inverse CDF for the three post-filter tapsets.
pub(crate) const TAPSET_ICDF: [u8; 3] = [2, 1, 0];

/// Write the post-filter section of the CELT frame header.
///
/// Mirrors the `pf_on` branch of libopus `celt_encode_with_ec`: with the filter
/// off it writes a single "off" flag when there is room; with it on it writes
/// the "on" flag, the pitch (octave + in-octave bits), the 3-bit quantised gain,
/// and the tapset. The section only exists at `start == 0`.
pub fn encode_postfilter(
    enc: &mut RangeEncoder,
    pf: &PostfilterParams,
    start: usize,
    total_bits: i32,
) {
    if start != 0 {
        return;
    }
    if !pf.pf_on {
        if enc.ec_tell() + 16 <= total_bits {
            enc.enc_bit_logp(false, 1);
        }
        return;
    }
    enc.enc_bit_logp(true, 1);
    let pitch1 = pf.pitch_index + 1;
    let octave = 27 - (pitch1 as u32).leading_zeros() as i32;
    enc.enc_uint(octave as u32, 6);
    enc.enc_bits((pitch1 - (16 << octave)) as u32, (4 + octave) as u32);
    enc.enc_bits(pf.qg as u32, 3);
    enc.enc_icdf(pf.tapset, &TAPSET_ICDF, 2);
}

/// Read the post-filter section written by [`encode_postfilter`]. Returns the
/// decoded parameters when the filter is on, or `None` when it is off or the
/// section is absent (`start != 0` or no room), matching the decoder's gating.
#[must_use]
pub fn decode_postfilter(
    dec: &mut RangeDecoder,
    start: usize,
    total_bits: i32,
) -> Option<PostfilterParams> {
    if start != 0 || dec.ec_tell() + 16 > total_bits || !dec.dec_bit_logp(1) {
        return None;
    }
    let octave = dec.dec_uint(6) as i32;
    let pitch_index = (16 << octave) + dec.dec_bits((4 + octave) as u32) as i32 - 1;
    let qg = dec.dec_bits(3) as i32;
    // The tapset is only present when there are still bits to spare.
    let tapset = if dec.ec_tell() + 2 <= total_bits {
        dec.dec_icdf(&TAPSET_ICDF, 2)
    } else {
        0
    };
    Some(PostfilterParams {
        pf_on: true,
        pitch_index,
        gain: 0.09375 * (qg + 1) as f32,
        qg,
        tapset,
    })
}
