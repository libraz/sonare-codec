#![deny(unsafe_code)]
#![warn(clippy::all)]

use sc_core::Error;

const AAC_SAMPLE_DURATION: u32 = 1024;

/// Demuxes AAC-LC samples from a minimal M4A/MP4 container into ADTS frames.
///
/// This intentionally targets the single-track layout emitted by `mux_aac`.
/// General-purpose MP4 demuxing remains delegated to the decode backend.
pub fn demux_aac(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mdat = find_box_payload(input, b"mdat").ok_or(Error::UnsupportedFormat)?;
    let stsz = find_box_payload(input, b"stsz").ok_or(Error::UnsupportedFormat)?;
    let esds = find_box_payload(input, b"esds").ok_or(Error::UnsupportedFormat)?;
    let asc = parse_audio_specific_config(esds)?;
    let sample_sizes = parse_stsz_sample_sizes(stsz)?;
    samples_to_adts(mdat, &sample_sizes, asc)
}

/// Muxes one or more AAC ADTS frames into a minimal M4A/MP4 container.
///
/// Raw AAC access units are intentionally not accepted yet because they do not
/// carry the profile, sample-rate index, and channel configuration needed for
/// the `mp4a` sample description.
pub fn mux_aac(adts: &[u8]) -> Result<Vec<u8>, Error> {
    let frames = parse_adts_frames(adts)?;
    let first = frames
        .first()
        .ok_or(Error::InvalidInput("AAC stream has no frames"))?;
    let asc = audio_specific_config(first.profile, first.sample_rate_index, first.channels);
    let sample_rate = sample_rate_from_index(first.sample_rate_index)?;

    let mdat_payload_len = frames
        .iter()
        .try_fold(0_usize, |sum, frame| sum.checked_add(frame.payload.len()))
        .ok_or(Error::InvalidInput("AAC stream is too large for MP4 mdat"))?;
    let mut mdat_payload = Vec::with_capacity(mdat_payload_len);
    let mut sample_sizes = Vec::with_capacity(frames.len());
    for frame in &frames {
        if frame.profile != first.profile
            || frame.sample_rate_index != first.sample_rate_index
            || frame.channels != first.channels
        {
            return Err(Error::UnsupportedFeature(
                "AAC ADTS parameter changes within stream",
            ));
        }
        sample_sizes.push(u32::try_from(frame.payload.len()).map_err(|_| {
            Error::InvalidInput("AAC frame is too large for 32-bit MP4 sample size")
        })?);
        mdat_payload.extend_from_slice(frame.payload);
    }

    let ftyp = ftyp_box();
    let mdat = box_with_payload(*b"mdat", &mdat_payload)?;
    let chunk_offset = u32::try_from(ftyp.len() + 8)
        .map_err(|_| Error::InvalidInput("MP4 chunk offset exceeds 32-bit range"))?;
    let moov = moov_box(
        sample_rate,
        first.channels,
        &asc,
        &sample_sizes,
        chunk_offset,
    )?;

    let mut out = Vec::with_capacity(ftyp.len() + mdat.len() + moov.len());
    out.extend_from_slice(&ftyp);
    out.extend_from_slice(&mdat);
    out.extend_from_slice(&moov);
    Ok(out)
}

#[derive(Debug)]
struct AdtsFrame<'a> {
    profile: u8,
    sample_rate_index: u8,
    channels: u8,
    payload: &'a [u8],
}

fn parse_adts_frames(mut input: &[u8]) -> Result<Vec<AdtsFrame<'_>>, Error> {
    let mut frames = Vec::with_capacity(input.len() / 7);
    while !input.is_empty() {
        if input.len() < 7 {
            return Err(Error::InvalidInput("truncated AAC ADTS header"));
        }
        if input[0] != 0xff || input[1] & 0xf0 != 0xf0 {
            return Err(Error::InvalidInput("missing AAC ADTS sync word"));
        }

        let protection_absent = input[1] & 0x01 != 0;
        let header_len = if protection_absent { 7 } else { 9 };
        let profile = ((input[2] >> 6) & 0x03) + 1;
        let sample_rate_index = (input[2] >> 2) & 0x0f;
        let channels = ((input[2] & 0x01) << 2) | ((input[3] >> 6) & 0x03);
        let frame_len = (usize::from(input[3] & 0x03) << 11)
            | (usize::from(input[4]) << 3)
            | usize::from(input[5] >> 5);

        if sample_rate_index >= 13 {
            return Err(Error::InvalidInput("invalid AAC ADTS sample-rate index"));
        }
        if channels == 0 {
            return Err(Error::UnsupportedFeature(
                "AAC program config elements are not supported",
            ));
        }
        if frame_len < header_len {
            return Err(Error::InvalidInput("invalid AAC ADTS frame length"));
        }
        if input.len() < frame_len {
            return Err(Error::InvalidInput("truncated AAC ADTS frame"));
        }

        frames.push(AdtsFrame {
            profile,
            sample_rate_index,
            channels,
            payload: &input[header_len..frame_len],
        });
        input = &input[frame_len..];
    }

    Ok(frames)
}

fn sample_rate_from_index(index: u8) -> Result<u32, Error> {
    const SAMPLE_RATES: [u32; 13] = [
        96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
        8_000, 7_350,
    ];
    SAMPLE_RATES
        .get(usize::from(index))
        .copied()
        .ok_or(Error::InvalidInput("invalid AAC sample-rate index"))
}

fn audio_specific_config(profile: u8, sample_rate_index: u8, channels: u8) -> [u8; 2] {
    [
        (profile << 3) | (sample_rate_index >> 1),
        ((sample_rate_index & 1) << 7) | (channels << 3),
    ]
}

fn ftyp_box() -> Vec<u8> {
    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(b"M4A ");
    write_u32(&mut payload, 0);
    payload.extend_from_slice(b"M4A ");
    payload.extend_from_slice(b"mp42");
    box_with_payload(*b"ftyp", &payload).expect("static ftyp box is representable")
}

fn moov_box(
    sample_rate: u32,
    channels: u8,
    asc: &[u8; 2],
    sample_sizes: &[u32],
    chunk_offset: u32,
) -> Result<Vec<u8>, Error> {
    let duration = checked_duration(sample_sizes.len())?;
    let mvhd = mvhd_box(sample_rate, duration);
    let trak = trak_box(sample_rate, channels, asc, sample_sizes, chunk_offset)?;
    box_with_children(*b"moov", &[mvhd, trak])
}

fn trak_box(
    sample_rate: u32,
    channels: u8,
    asc: &[u8; 2],
    sample_sizes: &[u32],
    chunk_offset: u32,
) -> Result<Vec<u8>, Error> {
    let duration = checked_duration(sample_sizes.len())?;
    let tkhd = tkhd_box(duration);
    let mdia = mdia_box(sample_rate, channels, asc, sample_sizes, chunk_offset)?;
    box_with_children(*b"trak", &[tkhd, mdia])
}

fn mdia_box(
    sample_rate: u32,
    channels: u8,
    asc: &[u8; 2],
    sample_sizes: &[u32],
    chunk_offset: u32,
) -> Result<Vec<u8>, Error> {
    let mdhd = mdhd_box(sample_rate, checked_duration(sample_sizes.len())?);
    let hdlr = hdlr_box();
    let minf = minf_box(sample_rate, channels, asc, sample_sizes, chunk_offset)?;
    box_with_children(*b"mdia", &[mdhd, hdlr, minf])
}

fn minf_box(
    sample_rate: u32,
    channels: u8,
    asc: &[u8; 2],
    sample_sizes: &[u32],
    chunk_offset: u32,
) -> Result<Vec<u8>, Error> {
    let smhd = full_box(*b"smhd", 0, 0, &[0, 0, 0, 0])?;
    let dinf = dinf_box()?;
    let stbl = stbl_box(sample_rate, channels, asc, sample_sizes, chunk_offset)?;
    box_with_children(*b"minf", &[smhd, dinf, stbl])
}

fn stbl_box(
    sample_rate: u32,
    channels: u8,
    asc: &[u8; 2],
    sample_sizes: &[u32],
    chunk_offset: u32,
) -> Result<Vec<u8>, Error> {
    let stsd = stsd_box(sample_rate, channels, asc)?;
    let stts = stts_box(sample_sizes.len())?;
    let stsc = stsc_box(sample_sizes.len())?;
    let stsz = stsz_box(sample_sizes)?;
    let stco = stco_box(chunk_offset);
    box_with_children(*b"stbl", &[stsd, stts, stsc, stsz, stco])
}

fn mvhd_box(timescale: u32, duration: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(96);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, timescale);
    write_u32(&mut payload, duration);
    write_u32(&mut payload, 0x0001_0000);
    write_u16(&mut payload, 0x0100);
    write_u16(&mut payload, 0);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 0);
    payload.extend_from_slice(&[
        0x00, 0x01, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x01, 0x00, 0x00, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0,
    ]);
    for _ in 0..6 {
        write_u32(&mut payload, 0);
    }
    write_u32(&mut payload, 2);
    full_box(*b"mvhd", 0, 0, &payload).expect("mvhd box is representable")
}

fn tkhd_box(duration: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(80);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 1);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, duration);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 0);
    write_u16(&mut payload, 0);
    write_u16(&mut payload, 0);
    write_u16(&mut payload, 0x0100);
    write_u16(&mut payload, 0);
    payload.extend_from_slice(&[
        0x00, 0x01, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x01, 0x00, 0x00, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0, 0,
    ]);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 0);
    full_box(*b"tkhd", 0, 0x0000_0007, &payload).expect("tkhd box is representable")
}

fn mdhd_box(sample_rate: u32, duration: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(20);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, sample_rate);
    write_u32(&mut payload, duration);
    write_u16(&mut payload, 0x55c4);
    write_u16(&mut payload, 0);
    full_box(*b"mdhd", 0, 0, &payload).expect("mdhd box is representable")
}

fn hdlr_box() -> Vec<u8> {
    let mut payload = Vec::with_capacity(25);
    write_u32(&mut payload, 0);
    payload.extend_from_slice(b"soun");
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 0);
    write_u32(&mut payload, 0);
    payload.extend_from_slice(b"SoundHandler\0");
    full_box(*b"hdlr", 0, 0, &payload).expect("hdlr box is representable")
}

fn dinf_box() -> Result<Vec<u8>, Error> {
    let url_payload = Vec::new();
    let url = full_box(*b"url ", 0, 1, &url_payload)?;

    let mut dref_payload = Vec::with_capacity(4 + url.len());
    write_u32(&mut dref_payload, 1);
    dref_payload.extend_from_slice(&url);
    let dref = full_box(*b"dref", 0, 0, &dref_payload)?;
    box_with_children(*b"dinf", &[dref])
}

fn stsd_box(sample_rate: u32, channels: u8, asc: &[u8; 2]) -> Result<Vec<u8>, Error> {
    let esds = esds_box(asc)?;
    let mut entry = Vec::with_capacity(28 + esds.len());
    entry.extend_from_slice(&[0; 6]);
    write_u16(&mut entry, 1);
    write_u16(&mut entry, 0);
    write_u16(&mut entry, 0);
    write_u32(&mut entry, 0);
    write_u16(&mut entry, u16::from(channels));
    write_u16(&mut entry, 16);
    write_u16(&mut entry, 0);
    write_u16(&mut entry, 0);
    write_u32(&mut entry, sample_rate << 16);
    entry.extend_from_slice(&esds);
    let mp4a = box_with_payload(*b"mp4a", &entry)?;

    let mut payload = Vec::with_capacity(4 + mp4a.len());
    write_u32(&mut payload, 1);
    payload.extend_from_slice(&mp4a);
    full_box(*b"stsd", 0, 0, &payload)
}

fn esds_box(asc: &[u8; 2]) -> Result<Vec<u8>, Error> {
    let mut decoder_specific = vec![0x05, asc.len() as u8];
    decoder_specific.extend_from_slice(asc);

    let mut decoder_config = vec![0x04, (13 + decoder_specific.len()) as u8, 0x40, 0x15];
    decoder_config.extend_from_slice(&[0, 0, 0]);
    write_u32(&mut decoder_config, 0);
    write_u32(&mut decoder_config, 0);
    decoder_config.extend_from_slice(&decoder_specific);

    let mut sl_config = vec![0x06, 0x01, 0x02];

    let descriptor_len = 3 + decoder_config.len() + sl_config.len();
    let mut es_descriptor = vec![0x03, descriptor_len as u8];
    write_u16(&mut es_descriptor, 1);
    es_descriptor.push(0);
    es_descriptor.extend_from_slice(&decoder_config);
    es_descriptor.append(&mut sl_config);

    full_box(*b"esds", 0, 0, &es_descriptor)
}

fn stts_box(sample_count: usize) -> Result<Vec<u8>, Error> {
    let mut payload = Vec::with_capacity(12);
    write_u32(&mut payload, 1);
    write_u32(
        &mut payload,
        u32::try_from(sample_count).map_err(|_| Error::InvalidInput("too many AAC samples"))?,
    );
    write_u32(&mut payload, AAC_SAMPLE_DURATION);
    full_box(*b"stts", 0, 0, &payload)
}

fn stsc_box(sample_count: usize) -> Result<Vec<u8>, Error> {
    let mut payload = Vec::with_capacity(16);
    write_u32(&mut payload, 1);
    write_u32(&mut payload, 1);
    write_u32(
        &mut payload,
        u32::try_from(sample_count).map_err(|_| Error::InvalidInput("too many AAC samples"))?,
    );
    write_u32(&mut payload, 1);
    full_box(*b"stsc", 0, 0, &payload)
}

fn stsz_box(sample_sizes: &[u32]) -> Result<Vec<u8>, Error> {
    let mut payload = Vec::with_capacity(8 + sample_sizes.len().saturating_mul(4));
    write_u32(&mut payload, 0);
    write_u32(
        &mut payload,
        u32::try_from(sample_sizes.len())
            .map_err(|_| Error::InvalidInput("too many AAC samples"))?,
    );
    for &size in sample_sizes {
        write_u32(&mut payload, size);
    }
    full_box(*b"stsz", 0, 0, &payload)
}

fn stco_box(chunk_offset: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(8);
    write_u32(&mut payload, 1);
    write_u32(&mut payload, chunk_offset);
    full_box(*b"stco", 0, 0, &payload).expect("stco box is representable")
}

#[derive(Clone, Copy, Debug)]
struct AudioSpecificConfig {
    profile: u8,
    sample_rate_index: u8,
    channels: u8,
}

fn parse_audio_specific_config(esds_payload: &[u8]) -> Result<AudioSpecificConfig, Error> {
    let descriptor = esds_payload
        .windows(4)
        .find(|window| window[0] == 0x05 && window[1] == 0x02)
        .ok_or(Error::UnsupportedFormat)?;
    let asc0 = descriptor[2];
    let asc1 = descriptor[3];
    let profile = asc0 >> 3;
    let sample_rate_index = ((asc0 & 0x07) << 1) | (asc1 >> 7);
    let channels = (asc1 >> 3) & 0x0f;

    if profile == 0 || profile > 4 {
        return Err(Error::UnsupportedFeature("AAC profile"));
    }
    if sample_rate_index >= 13 {
        return Err(Error::InvalidInput("invalid AAC sample-rate index"));
    }
    if channels == 0 {
        return Err(Error::UnsupportedFeature(
            "AAC program config elements are not supported",
        ));
    }

    Ok(AudioSpecificConfig {
        profile,
        sample_rate_index,
        channels,
    })
}

fn parse_stsz_sample_sizes(stsz_payload: &[u8]) -> Result<Vec<u32>, Error> {
    if stsz_payload.len() < 12 {
        return Err(Error::InvalidInput("truncated MP4 stsz box"));
    }
    let sample_size = read_u32(&stsz_payload[4..8])?;
    let sample_count = read_u32(&stsz_payload[8..12])?;
    let sample_count = usize::try_from(sample_count)
        .map_err(|_| Error::InvalidInput("MP4 sample count is too large"))?;

    if sample_size != 0 {
        return Ok(vec![sample_size; sample_count]);
    }

    let entries_len = sample_count
        .checked_mul(4)
        .ok_or(Error::InvalidInput("MP4 stsz box is too large"))?;
    if stsz_payload.len() < 12 + entries_len {
        return Err(Error::InvalidInput("truncated MP4 stsz sample table"));
    }

    let mut sizes = Vec::with_capacity(sample_count);
    for entry in stsz_payload[12..12 + entries_len].chunks_exact(4) {
        sizes.push(read_u32(entry)?);
    }
    Ok(sizes)
}

fn samples_to_adts(
    mdat_payload: &[u8],
    sample_sizes: &[u32],
    asc: AudioSpecificConfig,
) -> Result<Vec<u8>, Error> {
    let mut offset = 0_usize;
    let out_len = sample_sizes.iter().try_fold(0_usize, |sum, &sample_size| {
        let sample_size = usize::try_from(sample_size)
            .map_err(|_| Error::InvalidInput("AAC sample size is too large"))?;
        sum.checked_add(sample_size)
            .and_then(|value| value.checked_add(7))
            .ok_or(Error::InvalidInput("AAC ADTS output is too large"))
    })?;
    let mut out = Vec::with_capacity(out_len);
    for &sample_size in sample_sizes {
        let sample_size = usize::try_from(sample_size)
            .map_err(|_| Error::InvalidInput("AAC sample size is too large"))?;
        let end = offset
            .checked_add(sample_size)
            .ok_or(Error::InvalidInput("AAC sample offset overflow"))?;
        let payload = mdat_payload
            .get(offset..end)
            .ok_or(Error::InvalidInput("truncated MP4 mdat AAC sample"))?;
        write_adts_frame(&mut out, asc, payload)?;
        offset = end;
    }
    if offset != mdat_payload.len() {
        return Err(Error::InvalidInput("MP4 mdat contains trailing AAC bytes"));
    }
    Ok(out)
}

fn write_adts_frame(
    out: &mut Vec<u8>,
    asc: AudioSpecificConfig,
    access_unit: &[u8],
) -> Result<(), Error> {
    let frame_len = access_unit
        .len()
        .checked_add(7)
        .ok_or(Error::InvalidInput("AAC ADTS frame is too large"))?;
    if frame_len > 0x1fff {
        return Err(Error::InvalidInput("AAC ADTS frame exceeds 13-bit length"));
    }
    let adts_profile = asc
        .profile
        .checked_sub(1)
        .ok_or(Error::UnsupportedFeature("AAC profile"))?;
    if adts_profile > 3 {
        return Err(Error::UnsupportedFeature("AAC profile"));
    }

    out.push(0xff);
    out.push(0xf1);
    out.push((adts_profile << 6) | (asc.sample_rate_index << 2) | (asc.channels >> 2));
    out.push(((asc.channels & 0x03) << 6) | (((frame_len >> 11) & 0x03) as u8));
    out.push(((frame_len >> 3) & 0xff) as u8);
    out.push((((frame_len & 0x07) << 5) as u8) | 0x1f);
    out.push(0xfc);
    out.extend_from_slice(access_unit);
    Ok(())
}

fn find_box_payload<'a>(input: &'a [u8], target: &[u8; 4]) -> Option<&'a [u8]> {
    let mut pos = 0_usize;
    while pos.checked_add(8)? <= input.len() {
        let size = read_u32(input.get(pos..pos + 4)?).ok()? as usize;
        if size < 8 || pos.checked_add(size)? > input.len() {
            return None;
        }
        let box_type = input.get(pos + 4..pos + 8)?;
        let payload = input.get(pos + 8..pos + size)?;
        if box_type == target {
            return Some(payload);
        }
        if is_container_box(box_type) {
            if let Some(found) = find_box_payload(payload, target) {
                return Some(found);
            }
        } else if let Some(offset) = full_container_child_offset(box_type) {
            if let Some(found) = find_box_payload(payload.get(offset..)?, target) {
                return Some(found);
            }
        }
        pos += size;
    }
    None
}

fn is_container_box(box_type: &[u8]) -> bool {
    matches!(
        box_type,
        b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"dinf"
    )
}

fn full_container_child_offset(box_type: &[u8]) -> Option<usize> {
    match box_type {
        b"stsd" | b"dref" => Some(8),
        b"mp4a" => Some(28),
        _ => None,
    }
}

fn checked_duration(sample_count: usize) -> Result<u32, Error> {
    u32::try_from(sample_count)
        .ok()
        .and_then(|count| count.checked_mul(AAC_SAMPLE_DURATION))
        .ok_or(Error::InvalidInput("AAC duration exceeds 32-bit MP4 field"))
}

fn box_with_children(name: [u8; 4], children: &[Vec<u8>]) -> Result<Vec<u8>, Error> {
    let payload_len = children
        .iter()
        .try_fold(0_usize, |sum, child| sum.checked_add(child.len()))
        .ok_or(Error::InvalidInput("MP4 box is too large"))?;
    let mut payload = Vec::with_capacity(payload_len);
    for child in children {
        payload.extend_from_slice(child);
    }
    box_with_payload(name, &payload)
}

fn full_box(name: [u8; 4], version: u8, flags: u32, payload: &[u8]) -> Result<Vec<u8>, Error> {
    if flags > 0x00ff_ffff {
        return Err(Error::InvalidInput("MP4 full box flags exceed 24 bits"));
    }
    let mut full_payload = Vec::with_capacity(4 + payload.len());
    full_payload.push(version);
    full_payload.extend_from_slice(&[
        ((flags >> 16) & 0xff) as u8,
        ((flags >> 8) & 0xff) as u8,
        (flags & 0xff) as u8,
    ]);
    full_payload.extend_from_slice(payload);
    box_with_payload(name, &full_payload)
}

fn box_with_payload(name: [u8; 4], payload: &[u8]) -> Result<Vec<u8>, Error> {
    let size = payload
        .len()
        .checked_add(8)
        .ok_or(Error::InvalidInput("MP4 box is too large"))?;
    let size =
        u32::try_from(size).map_err(|_| Error::InvalidInput("MP4 box exceeds 32-bit size"))?;

    let mut out = Vec::with_capacity(size as usize);
    write_u32(&mut out, size);
    out.extend_from_slice(&name);
    out.extend_from_slice(payload);
    Ok(out)
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn read_u32(input: &[u8]) -> Result<u32, Error> {
    let bytes: [u8; 4] = input
        .try_into()
        .map_err(|_| Error::InvalidInput("truncated 32-bit MP4 field"))?;
    Ok(u32::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::{demux_aac, mux_aac};
    use sc_core::Error;

    #[test]
    fn muxes_single_adts_frame_as_m4a() {
        let adts = adts_frame(&[0x11, 0x22, 0x33, 0x44]);
        let mp4 = mux_aac(&adts).unwrap();

        assert_top_level_boxes_are_sized(&mp4);
        assert_eq!(&mp4[4..8], b"ftyp");
        assert!(contains(&mp4, b"mdat"));
        assert!(contains(&mp4, b"moov"));
        assert!(contains(&mp4, b"mp4a"));
        assert!(contains(&mp4, b"esds"));
        assert_eq!(read_stco_offset(&mp4), mdat_payload_offset(&mp4));
        assert!(contains(&mp4, &[0x12, 0x10]));
        assert!(!contains(&mp4, &adts[..7]));
        assert!(contains(&mp4, &[0x11, 0x22, 0x33, 0x44]));
        assert_eq!(demux_aac(&mp4).unwrap(), adts);
    }

    #[test]
    fn muxes_multiple_adts_frames() {
        let mut adts = adts_frame(&[0xaa, 0xbb]);
        adts.extend_from_slice(&adts_frame(&[0xcc, 0xdd, 0xee]));

        let mp4 = mux_aac(&adts).unwrap();

        assert!(contains(&mp4, &[0xaa, 0xbb, 0xcc, 0xdd, 0xee]));
        assert!(contains(&mp4, &2_u32.to_be_bytes()));
        assert_eq!(demux_aac(&mp4).unwrap(), adts);
    }

    #[test]
    fn rejects_truncated_adts_frame() {
        let err = mux_aac(&[0xff, 0xf1, 0x50]).unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidInput("truncated AAC ADTS header")
        ));
    }

    #[test]
    fn rejects_parameter_changes() {
        let mut adts = adts_frame(&[0x00]);
        let mut changed = adts_frame(&[0x01]);
        changed[2] = (changed[2] & !0x3c) | (3 << 2);
        adts.extend_from_slice(&changed);

        let err = mux_aac(&adts).unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedFeature("AAC ADTS parameter changes within stream")
        ));
    }

    #[test]
    fn demux_rejects_truncated_stsz_sample_table() {
        let adts = adts_frame(&[0x11, 0x22]);
        let mut mp4 = mux_aac(&adts).unwrap();
        let stsz = find_box_payload_range(&mp4, b"stsz").unwrap();
        mp4[stsz.start + 8..stsz.start + 12].copy_from_slice(&2_u32.to_be_bytes());

        let err = demux_aac(&mp4).unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidInput("truncated MP4 stsz sample table")
        ));
    }

    #[test]
    fn demux_rejects_invalid_audio_specific_config() {
        let adts = adts_frame(&[0x11, 0x22]);
        let mut mp4 = mux_aac(&adts).unwrap();
        let asc = find_subslice(&mp4, &[0x12, 0x10]).unwrap();
        mp4[asc..asc + 2].copy_from_slice(&[0x17, 0x90]);

        let err = demux_aac(&mp4).unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidInput("invalid AAC sample-rate index")
        ));
    }

    #[test]
    fn demux_rejects_truncated_mdat_sample() {
        let adts = adts_frame(&[0xaa, 0xbb, 0xcc]);
        let mut mp4 = mux_aac(&adts).unwrap();
        let stsz = find_box_payload_range(&mp4, b"stsz").unwrap();
        mp4[stsz.start + 12..stsz.start + 16].copy_from_slice(&4_u32.to_be_bytes());

        let err = demux_aac(&mp4).unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidInput("truncated MP4 mdat AAC sample")
        ));
    }

    #[test]
    fn demux_rejects_trailing_mdat_bytes() {
        let adts = adts_frame(&[0xaa, 0xbb, 0xcc]);
        let mut mp4 = mux_aac(&adts).unwrap();
        let mdat = find_top_level_box_range(&mp4, b"mdat").unwrap();
        mp4.insert(mdat.end, 0xdd);
        let new_size = u32::try_from(mdat.end - mdat.start + 1).unwrap();
        mp4[mdat.start..mdat.start + 4].copy_from_slice(&new_size.to_be_bytes());

        let err = demux_aac(&mp4).unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidInput("MP4 mdat contains trailing AAC bytes")
        ));
    }

    fn adts_frame(payload: &[u8]) -> Vec<u8> {
        let frame_len = payload.len() + 7;
        let mut out = vec![
            0xff,
            0xf1,
            0x50,
            0x80 | (((frame_len >> 11) & 0x03) as u8),
            ((frame_len >> 3) & 0xff) as u8,
            (((frame_len & 0x07) << 5) as u8) | 0x1f,
            0xfc,
        ];
        out.extend_from_slice(payload);
        out
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn assert_top_level_boxes_are_sized(mp4: &[u8]) {
        let mut pos = 0;
        while pos < mp4.len() {
            assert!(mp4.len() - pos >= 8);
            let size = read_u32(&mp4[pos..pos + 4]) as usize;
            assert!(size >= 8);
            assert!(pos + size <= mp4.len());
            pos += size;
        }
        assert_eq!(pos, mp4.len());
    }

    fn mdat_payload_offset(mp4: &[u8]) -> u32 {
        let mut pos = 0;
        while pos + 8 <= mp4.len() {
            let size = read_u32(&mp4[pos..pos + 4]) as usize;
            if &mp4[pos + 4..pos + 8] == b"mdat" {
                return (pos + 8) as u32;
            }
            pos += size;
        }
        panic!("mdat box not found");
    }

    fn read_stco_offset(mp4: &[u8]) -> u32 {
        let stco = find_box_payload(mp4, b"stco").expect("stco box exists");
        assert_eq!(stco[0], 0);
        assert_eq!(&stco[4..8], &1_u32.to_be_bytes());
        read_u32(&stco[8..12])
    }

    fn find_box_payload<'a>(input: &'a [u8], name: &[u8; 4]) -> Option<&'a [u8]> {
        let mut pos = 0;
        while pos + 8 <= input.len() {
            let size = read_u32(&input[pos..pos + 4]) as usize;
            if size < 8 || pos + size > input.len() {
                return None;
            }

            let box_name = &input[pos + 4..pos + 8];
            let payload = &input[pos + 8..pos + size];
            if box_name == name {
                return Some(payload);
            }
            if matches!(
                box_name,
                b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"stsd" | b"mp4a"
            ) {
                let child_payload = if matches!(box_name, b"stsd") {
                    payload.get(8..)?
                } else if matches!(box_name, b"mp4a") {
                    payload.get(28..)?
                } else {
                    payload
                };
                if let Some(found) = find_box_payload(child_payload, name) {
                    return Some(found);
                }
            }
            pos += size;
        }
        None
    }

    fn find_box_payload_range(input: &[u8], name: &[u8; 4]) -> Option<std::ops::Range<usize>> {
        find_box_payload_range_from(input, name, 0)
    }

    fn find_box_payload_range_from(
        input: &[u8],
        name: &[u8; 4],
        base: usize,
    ) -> Option<std::ops::Range<usize>> {
        let mut pos = 0;
        while pos + 8 <= input.len() {
            let size = read_u32(&input[pos..pos + 4]) as usize;
            if size < 8 || pos + size > input.len() {
                return None;
            }

            let box_name = &input[pos + 4..pos + 8];
            let payload_start = pos + 8;
            let payload_end = pos + size;
            if box_name == name {
                return Some(base + payload_start..base + payload_end);
            }
            if matches!(box_name, b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl") {
                if let Some(found) = find_box_payload_range_from(
                    &input[payload_start..payload_end],
                    name,
                    base + payload_start,
                ) {
                    return Some(found);
                }
            } else if matches!(box_name, b"stsd" | b"dref" | b"mp4a") {
                let child_offset = if matches!(box_name, b"mp4a") { 28 } else { 8 };
                if let Some(child_payload) = input[payload_start..payload_end].get(child_offset..) {
                    if let Some(found) = find_box_payload_range_from(
                        child_payload,
                        name,
                        base + payload_start + child_offset,
                    ) {
                        return Some(found);
                    }
                }
            }
            pos += size;
        }
        None
    }

    fn find_top_level_box_range(input: &[u8], name: &[u8; 4]) -> Option<std::ops::Range<usize>> {
        let mut pos = 0;
        while pos + 8 <= input.len() {
            let size = read_u32(&input[pos..pos + 4]) as usize;
            if size < 8 || pos + size > input.len() {
                return None;
            }
            if &input[pos + 4..pos + 8] == name {
                return Some(pos..pos + size);
            }
            pos += size;
        }
        None
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn read_u32(input: &[u8]) -> u32 {
        u32::from_be_bytes(input.try_into().unwrap())
    }
}
