//! Vorbis identification and comment header packing.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/info.c`
//! (`_vorbis_pack_info`, `_vorbis_pack_comment`, `_v_writestring`): the first
//! two of the three Vorbis I header packets (Vorbis I spec §4.2.1–4.2.3). The
//! third, the setup header, packs the whole codec configuration and is a
//! separate stage. Derivative work of libvorbis/aoTuV (BSD-3-Clause); see
//! `LICENSE-THIRDPARTY`.
//!
//! Every header packet begins with a one-byte packet type (`0x01` id, `0x03`
//! comment, `0x05` setup) followed by the literal `"vorbis"`, then the
//! type-specific fields packed LSb-first via the Ogg bit packer, and a trailing
//! framing flag bit. After the 7-byte preamble the stream is byte-aligned, so
//! the 32-bit fields and embedded strings stay byte-aligned until that flag.

// Consumed by the Vorbis stream assembler; the live encoder still ships via FFI.
#![allow(dead_code)]

use crate::codebook::ov_ilog;
use crate::oggpack::BitWriter;

/// The common `"vorbis"` signature every header packet carries after its type.
const VORBIS_SIGNATURE: &[u8; 6] = b"vorbis";

/// Packet type byte for the identification header.
const PACKET_TYPE_ID: u32 = 0x01;
/// Packet type byte for the comment header.
const PACKET_TYPE_COMMENT: u32 = 0x03;

/// Write each byte of `s` as eight bits (libvorbis `_v_writestring`).
fn write_string(w: &mut BitWriter, s: &[u8]) {
    for &byte in s {
        w.write(u32::from(byte), 8);
    }
}

/// Pack the Vorbis identification header (Vorbis I spec §4.2.2).
///
/// `blocksize_short`/`blocksize_long` are the two window sizes (powers of two,
/// `short <= long`); the header stores their `log2` as `ilog(blocksize - 1)`.
/// The three bitrate fields are the nominal/upper/lower hints (use `0` for
/// "unset", as libvorbis does for pure VBR).
#[must_use]
pub fn pack_identification_header(
    channels: u8,
    sample_rate: u32,
    bitrate_upper: i32,
    bitrate_nominal: i32,
    bitrate_lower: i32,
    blocksize_short: u32,
    blocksize_long: u32,
) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.write(PACKET_TYPE_ID, 8);
    write_string(&mut w, VORBIS_SIGNATURE);

    w.write(0x00, 32); // vorbis_version
    w.write(u32::from(channels), 8);
    w.write(sample_rate, 32);
    w.write(bitrate_upper as u32, 32);
    w.write(bitrate_nominal as u32, 32);
    w.write(bitrate_lower as u32, 32);
    w.write(ov_ilog(blocksize_short - 1) as u32, 4);
    w.write(ov_ilog(blocksize_long - 1) as u32, 4);
    w.write(1, 1); // framing flag

    w.into_bytes()
}

/// Pack the Vorbis comment header (Vorbis I spec §4.2.3): the vendor string and
/// a list of UTF-8 `comments` (each typically `"TAG=value"`).
#[must_use]
pub fn pack_comment_header(vendor: &[u8], comments: &[Vec<u8>]) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.write(PACKET_TYPE_COMMENT, 8);
    write_string(&mut w, VORBIS_SIGNATURE);

    w.write(vendor.len() as u32, 32);
    write_string(&mut w, vendor);

    w.write(comments.len() as u32, 32);
    for comment in comments {
        w.write(comment.len() as u32, 32);
        write_string(&mut w, comment);
    }
    w.write(1, 1); // framing flag

    w.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oggpack::BitReader;

    fn read_string(r: &mut BitReader, bytes: usize) -> Vec<u8> {
        (0..bytes).map(|_| r.read(8) as u8).collect()
    }

    #[test]
    fn identification_header_round_trips() {
        let packet = pack_identification_header(2, 44_100, 0, 128_000, 0, 256, 2048);
        let mut r = BitReader::new(&packet);

        assert_eq!(r.read(8), PACKET_TYPE_ID, "packet type");
        assert_eq!(read_string(&mut r, 6), VORBIS_SIGNATURE, "signature");
        assert_eq!(r.read(32), 0, "vorbis_version");
        assert_eq!(r.read(8), 2, "channels");
        assert_eq!(r.read(32), 44_100, "sample rate");
        assert_eq!(r.read(32), 0, "bitrate upper");
        assert_eq!(r.read(32), 128_000, "bitrate nominal");
        assert_eq!(r.read(32), 0, "bitrate lower");
        assert_eq!(r.read(4), 8, "short blocksize log2 (256)");
        assert_eq!(r.read(4), 11, "long blocksize log2 (2048)");
        assert_eq!(r.read(1), 1, "framing flag");
    }

    #[test]
    fn identification_header_signature_bytes() {
        let packet = pack_identification_header(1, 48_000, 0, 0, 0, 256, 2048);
        // Byte 0 is the type; bytes 1..7 are the literal "vorbis".
        assert_eq!(packet[0], 0x01);
        assert_eq!(&packet[1..7], VORBIS_SIGNATURE);
    }

    #[test]
    fn comment_header_round_trips() {
        let vendor = b"sonare-codec".to_vec();
        let comments = vec![
            b"TITLE=Test".to_vec(),
            b"ARTIST=sonare".to_vec(),
            b"ENCODER=sonare-codec".to_vec(),
        ];
        let packet = pack_comment_header(&vendor, &comments);
        let mut r = BitReader::new(&packet);

        assert_eq!(r.read(8), PACKET_TYPE_COMMENT, "packet type");
        assert_eq!(read_string(&mut r, 6), VORBIS_SIGNATURE, "signature");

        let vlen = r.read(32) as usize;
        assert_eq!(read_string(&mut r, vlen), vendor, "vendor");

        let count = r.read(32) as usize;
        assert_eq!(count, comments.len(), "comment count");
        for expected in &comments {
            let len = r.read(32) as usize;
            assert_eq!(&read_string(&mut r, len), expected, "comment");
        }
        assert_eq!(r.read(1), 1, "framing flag");
    }

    #[test]
    fn comment_header_handles_no_comments() {
        let packet = pack_comment_header(b"v", &[]);
        let mut r = BitReader::new(&packet);
        assert_eq!(r.read(8), PACKET_TYPE_COMMENT);
        assert_eq!(read_string(&mut r, 6), VORBIS_SIGNATURE);
        assert_eq!(r.read(32), 1, "vendor length");
        assert_eq!(read_string(&mut r, 1), b"v");
        assert_eq!(r.read(32), 0, "zero comments");
        assert_eq!(r.read(1), 1, "framing flag");
    }

    #[test]
    fn short_long_blocksizes_encode_their_log2() {
        // ilog(bs - 1) == log2(bs) for power-of-two block sizes.
        let packet = pack_identification_header(1, 48_000, 0, 0, 0, 512, 4096);
        let mut r = BitReader::new(&packet);
        // Skip type(8)+sig(48)+version(32)+ch(8)+rate(32)+3*bitrate(96) = 224 bits.
        for _ in 0..28 {
            r.read(8);
        }
        assert_eq!(r.read(4), 9, "short 512 -> 9");
        assert_eq!(r.read(4), 12, "long 4096 -> 12");
    }
}
