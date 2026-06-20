use super::*;

pub fn pack_section_data(sections: &[AacSection], band_width: usize) -> Result<Vec<u8>, Error> {
    Ok(pack_section_data_with_len(sections, band_width)?.bytes)
}

/// Packs AAC section codebook and length metadata while preserving bit length.
pub fn pack_section_data_with_len(
    sections: &[AacSection],
    band_width: usize,
) -> Result<PackedBits, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.end <= section.start
            || section.start % band_width != 0
            || section.end % band_width != 0
        {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }

        writer.write_bits(u32::from(section.codebook.id()), 4)?;
        let mut band_count = (section.end - section.start) / band_width;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs AAC quad-codebook section metadata with caller-supplied codebook ids.
pub fn pack_quad_section_data_with_len(
    sections: &[AacQuadSection],
    band_width: usize,
) -> Result<PackedBits, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.codebook_id > 4 {
            return Err(Error::InvalidInput("AAC quad codebook id must be 0..=4"));
        }
        if section.end <= section.start
            || section.start % band_width != 0
            || section.end % band_width != 0
        {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }

        writer.write_bits(u32::from(section.codebook_id), 4)?;
        let mut band_count = (section.end - section.start) / band_width;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs standard AAC spectral section metadata with caller-supplied codebook ids.
pub fn pack_spectral_section_data_with_len(
    sections: &[AacSpectralSection],
    band_width: usize,
) -> Result<PackedBits, Error> {
    if band_width == 0 {
        return Err(Error::InvalidInput(
            "AAC section band width must be non-zero",
        ));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.codebook_id > AacCodebook::Escape.id() {
            return Err(Error::InvalidInput(
                "AAC spectral codebook id must be 0..=11",
            ));
        }
        if section.end <= section.start
            || section.start % band_width != 0
            || section.end % band_width != 0
        {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }

        writer.write_bits(u32::from(section.codebook_id), 4)?;
        let mut band_count = (section.end - section.start) / band_width;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

pub fn pack_section_data_with_offsets(
    sections: &[AacSection],
    offsets: &[usize],
) -> Result<PackedBits, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;

        writer.write_bits(u32::from(section.codebook.id()), 4)?;
        let mut band_count = end_band - start_band;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

/// Packs standard AAC spectral section metadata with scale-factor band offsets.
pub fn pack_spectral_section_data_with_offsets(
    sections: &[AacSpectralSection],
    offsets: &[usize],
) -> Result<PackedBits, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.codebook_id > AacCodebook::Escape.id() {
            return Err(Error::InvalidInput(
                "AAC spectral codebook id must be 0..=11",
            ));
        }
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;

        writer.write_bits(u32::from(section.codebook_id), 4)?;
        let mut band_count = end_band - start_band;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}

pub(crate) fn pack_magnitude_section_data_with_offsets(
    sections: &[AacMagnitudeSection<'_>],
    offsets: &[usize],
) -> Result<PackedBits, Error> {
    if offsets.len() < 2 {
        return Err(Error::InvalidInput("AAC scale-factor offsets are empty"));
    }

    let mut writer = CoreBitWriter::new();
    for section in sections {
        if section.end <= section.start {
            return Err(Error::InvalidInput("invalid AAC section range"));
        }
        let start_band = offset_band_index(offsets, section.start)?;
        let end_band = offset_band_index(offsets, section.end)?;

        writer.write_bits(u32::from(section.codebook_id), 4)?;
        let mut band_count = end_band - start_band;
        if band_count == 0 {
            return Err(Error::InvalidInput("invalid AAC section length"));
        }
        while band_count >= 31 {
            writer.write_bits(31, 5)?;
            band_count -= 31;
        }
        writer.write_bits(
            u32::try_from(band_count)
                .map_err(|_| Error::InvalidInput("AAC section is too long"))?,
            5,
        )?;
    }
    let bit_len = writer.bit_len();
    Ok(PackedBits {
        bytes: writer.finish_byte_aligned(),
        bit_len,
    })
}
