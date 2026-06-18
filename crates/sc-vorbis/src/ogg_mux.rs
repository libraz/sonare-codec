//! Ogg page muxing for a Vorbis logical bitstream.
//!
//! Lays the three Vorbis header packets (identification, comment, setup) and the
//! audio packets into Ogg pages per the Ogg bitstream spec (RFC 3533) and the
//! Vorbis I mapping: the identification header takes the BOS page alone, the
//! comment and setup headers share the following page(s), and the audio packets
//! carry the running granule (sample) position, with the final page flagged EOS.
//!
//! This is framing only — it does not synthesize the packets; it is the inverse
//! of the page parser the decoder side relies on. Written from the Ogg/Vorbis
//! specs (not ported from libogg), so no upstream notice attaches.

// Some entry points (e.g. mux_packets, demux) are exercised only by tests and
// the stream codec, not the shipping encoder path.
#![allow(dead_code)]

/// The Ogg page capture pattern.
const OGG_CAPTURE: &[u8; 4] = b"OggS";

/// Granule sentinel for a page on which no packet completes (Ogg's `-1`).
const NO_GRANULE: u64 = u64::MAX;

/// `continued packet` page flag.
const FLAG_CONTINUED: u8 = 0x01;
/// `beginning of stream` page flag.
const FLAG_BOS: u8 = 0x02;
/// `end of stream` page flag.
const FLAG_EOS: u8 = 0x04;

/// Maximum lacing values (segments) a single Ogg page can carry.
const MAX_SEGMENTS: usize = 255;

/// One packet queued for muxing.
pub struct OggPacket<'a> {
    /// The packet payload.
    pub data: &'a [u8],
    /// Sample position reached once this packet is decoded (the page that
    /// completes the packet carries this as its granule position).
    pub granule: u64,
    /// Force a page boundary immediately after this packet.
    pub flush: bool,
}

/// Ogg's CRC32 (polynomial `0x04c11db7`, no input/output reflection, init 0) —
/// the same generator the decoder verifies pages against.
fn ogg_crc(bytes: &[u8]) -> u32 {
    let mut crc = 0u32;
    for &byte in bytes {
        crc ^= u32::from(byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04c1_1db7
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Builds an Ogg Vorbis stream from the three header packets and the ordered
/// audio packets (each paired with its end granule position). The first audio
/// packet begins a fresh page after the setup header, and the last page is
/// flagged end-of-stream.
#[must_use]
pub fn mux_vorbis(
    serial: u32,
    id_header: &[u8],
    comment_header: &[u8],
    setup_header: &[u8],
    audio: &[(Vec<u8>, u64)],
) -> Vec<u8> {
    let mut packets = Vec::with_capacity(3 + audio.len());
    // The identification header must occupy the BOS page by itself.
    packets.push(OggPacket {
        data: id_header,
        granule: 0,
        flush: true,
    });
    // Comment and setup share the following page(s); flush after setup so the
    // first audio packet starts a new page.
    packets.push(OggPacket {
        data: comment_header,
        granule: 0,
        flush: false,
    });
    packets.push(OggPacket {
        data: setup_header,
        granule: 0,
        flush: true,
    });
    // Audio packets accumulate into shared pages (the muxer flushes a page when
    // it fills, and the final packet flushes the tail) — one page per packet
    // would bury small packets under Ogg page headers.
    for (data, granule) in audio {
        packets.push(OggPacket {
            data,
            granule: *granule,
            flush: false,
        });
    }
    mux_packets(serial, &packets)
}

/// Lays an ordered packet list into Ogg pages for one logical stream. Each
/// packet is segmented into 255-byte lacing values; pages hold up to 255
/// segments and a packet that overflows a page continues onto the next (flagged
/// `continued`). The first page is BOS and the last EOS.
#[must_use]
pub fn mux_packets(serial: u32, packets: &[OggPacket]) -> Vec<u8> {
    let mut stream = Vec::new();
    let mut seq: u32 = 0;
    let mut bos_pending = true;

    // Current-page accumulators.
    let mut laces: Vec<u8> = Vec::new();
    let mut payload: Vec<u8> = Vec::new();
    let mut continued = false; // this page begins mid-packet

    // Granule of the last packet that completed on the current page (Ogg's `-1`
    // until one does); a full page carries it even when more packets follow.
    let mut page_granule = NO_GRANULE;

    for (idx, pkt) in packets.iter().enumerate() {
        let is_last = idx + 1 == packets.len();

        // Split the packet into (lacing, byte-chunk) pieces: full 255s then the
        // remainder (which is 0..254, so a length that is a multiple of 255
        // emits a terminating zero lace).
        let mut offset = 0usize;
        loop {
            let remaining = pkt.data.len() - offset;
            let take = remaining.min(255);

            if laces.len() == MAX_SEGMENTS {
                // Page full: flush it carrying the granule of the last packet
                // that completed on it (`-1` if a single packet still spans it).
                emit_page(
                    &mut stream,
                    serial,
                    &mut seq,
                    &mut bos_pending,
                    continued,
                    false,
                    page_granule,
                    &laces,
                    &payload,
                );
                laces.clear();
                payload.clear();
                // This flush is always mid-packet (a completed packet that fills
                // the page is handled by the post-completion flush below), so the
                // next page continues the current packet.
                continued = true;
                page_granule = NO_GRANULE;
            }

            laces.push(take as u8);
            payload.extend_from_slice(&pkt.data[offset..offset + take]);
            offset += take;

            // A lace < 255 terminates the packet; a final exact-multiple packet
            // terminates with the zero lace just pushed.
            if take < 255 {
                break;
            }
        }

        // The packet has completed on the current page.
        page_granule = pkt.granule;
        if pkt.flush || is_last || laces.len() == MAX_SEGMENTS {
            emit_page(
                &mut stream,
                serial,
                &mut seq,
                &mut bos_pending,
                continued,
                is_last,
                page_granule,
                &laces,
                &payload,
            );
            laces.clear();
            payload.clear();
            continued = false;
            page_granule = NO_GRANULE;
        }
    }

    stream
}

/// Parses an Ogg stream back into its ordered logical packets — the inverse of
/// [`mux_packets`]. Reassembles across pages by the lacing rule (a value < 255
/// terminates a packet, 255 continues it). Returns `None` on a malformed or
/// truncated stream. Safe on arbitrary input: every index is length-checked.
#[must_use]
pub fn demux(mut input: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut packets = Vec::new();
    let mut current = Vec::new();
    while !input.is_empty() {
        if input.len() < 27 || &input[0..4] != OGG_CAPTURE {
            return None;
        }
        let seg_count = usize::from(input[26]);
        let table_end = 27 + seg_count;
        if input.len() < table_end {
            return None;
        }
        let laces = &input[27..table_end];
        let payload_len: usize = laces.iter().map(|&l| usize::from(l)).sum();
        let page_end = table_end.checked_add(payload_len)?;
        if input.len() < page_end {
            return None;
        }
        let payload = &input[table_end..page_end];

        let mut off = 0usize;
        for &lace in laces {
            let take = usize::from(lace);
            let end = off.checked_add(take)?;
            if end > payload.len() {
                return None;
            }
            current.extend_from_slice(&payload[off..end]);
            off = end;
            if lace < 255 {
                packets.push(std::mem::take(&mut current));
            }
        }
        input = &input[page_end..];
    }
    Some(packets)
}

/// Appends one fully-formed page (header + lacing + payload + CRC) to `stream`.
#[allow(clippy::too_many_arguments)]
fn emit_page(
    stream: &mut Vec<u8>,
    serial: u32,
    seq: &mut u32,
    bos_pending: &mut bool,
    continued: bool,
    eos: bool,
    granule: u64,
    laces: &[u8],
    payload: &[u8],
) {
    let mut header_type = 0u8;
    if continued {
        header_type |= FLAG_CONTINUED;
    }
    if *bos_pending {
        header_type |= FLAG_BOS;
        *bos_pending = false;
    }
    if eos {
        header_type |= FLAG_EOS;
    }

    let mut page = Vec::with_capacity(27 + laces.len() + payload.len());
    page.extend_from_slice(OGG_CAPTURE);
    page.push(0); // stream structure version
    page.push(header_type);
    page.extend_from_slice(&granule.to_le_bytes());
    page.extend_from_slice(&serial.to_le_bytes());
    page.extend_from_slice(&seq.to_le_bytes());
    page.extend_from_slice(&0u32.to_le_bytes()); // CRC placeholder
    page.push(laces.len() as u8);
    page.extend_from_slice(laces);
    page.extend_from_slice(payload);

    let crc = ogg_crc(&page);
    page[22..26].copy_from_slice(&crc.to_le_bytes());

    *seq = seq.wrapping_add(1);
    stream.extend_from_slice(&page);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A parsed Ogg page (header fields plus its lacing values and payload).
    struct ParsedPage {
        header_type: u8,
        granule: u64,
        serial: u32,
        seq: u32,
        laces: Vec<u8>,
        payload: Vec<u8>,
    }

    /// Walks an Ogg stream into pages, verifying each page's CRC.
    fn parse_pages(mut input: &[u8]) -> Vec<ParsedPage> {
        let mut pages = Vec::new();
        while !input.is_empty() {
            assert!(input.len() >= 27, "page header truncated");
            assert_eq!(&input[0..4], OGG_CAPTURE, "capture pattern");
            let seg_count = usize::from(input[26]);
            let table_end = 27 + seg_count;
            assert!(input.len() >= table_end, "segment table truncated");
            let laces = input[27..table_end].to_vec();
            let payload_len: usize = laces.iter().map(|&l| usize::from(l)).sum();
            let page_end = table_end + payload_len;
            assert!(input.len() >= page_end, "payload truncated");

            // Verify the stored CRC matches a recompute with the field zeroed.
            let stored = u32::from_le_bytes([input[22], input[23], input[24], input[25]]);
            let mut page = input[..page_end].to_vec();
            page[22..26].copy_from_slice(&0u32.to_le_bytes());
            assert_eq!(ogg_crc(&page), stored, "page CRC mismatch");

            pages.push(ParsedPage {
                header_type: input[5],
                granule: u64::from_le_bytes(input[6..14].try_into().unwrap()),
                serial: u32::from_le_bytes(input[14..18].try_into().unwrap()),
                seq: u32::from_le_bytes(input[18..22].try_into().unwrap()),
                laces,
                payload: input[table_end..page_end].to_vec(),
            });
            input = &input[page_end..];
        }
        pages
    }

    /// Reassembles the logical packets from parsed pages using the lacing rule:
    /// a lace < 255 terminates a packet; 255 means it continues.
    fn reassemble(pages: &[ParsedPage]) -> Vec<Vec<u8>> {
        let mut packets = Vec::new();
        let mut current = Vec::new();
        for page in pages {
            let mut off = 0usize;
            for &lace in &page.laces {
                let take = usize::from(lace);
                current.extend_from_slice(&page.payload[off..off + take]);
                off += take;
                if lace < 255 {
                    packets.push(std::mem::take(&mut current));
                }
            }
        }
        packets
    }

    #[test]
    fn standard_layout_round_trips() {
        let id = b"\x01vorbis-id".to_vec();
        let comment = b"\x03vorbis-comment".to_vec();
        let setup = b"\x05vorbis-setup-codebooks".to_vec();
        let audio = vec![
            (vec![0xAAu8; 40], 1024u64),
            (vec![0xBBu8; 55], 2048u64),
            (vec![0xCCu8; 33], 3072u64),
        ];

        let stream = mux_vorbis(0x1234_5678, &id, &comment, &setup, &audio);
        let pages = parse_pages(&stream);

        // Packets reassemble in order: id, comment, setup, then audio.
        let packets = reassemble(&pages);
        assert_eq!(packets[0], id);
        assert_eq!(packets[1], comment);
        assert_eq!(packets[2], setup);
        assert_eq!(packets[3], audio[0].0);
        assert_eq!(packets[4], audio[1].0);
        assert_eq!(packets[5], audio[2].0);
        assert_eq!(packets.len(), 6);

        // BOS on the first page only, EOS on the last only.
        assert_eq!(pages[0].header_type & FLAG_BOS, FLAG_BOS);
        assert_eq!(pages[0].header_type & FLAG_EOS, 0);
        let last = pages.len() - 1;
        assert_eq!(pages[last].header_type & FLAG_EOS, FLAG_EOS);
        for (i, page) in pages.iter().enumerate() {
            if i != 0 {
                assert_eq!(page.header_type & FLAG_BOS, 0, "stray BOS on page {i}");
            }
        }

        // The id header occupies the BOS page alone.
        assert_eq!(pages[0].payload, id);

        // Serial is constant; sequence numbers increment from zero.
        for (i, page) in pages.iter().enumerate() {
            assert_eq!(page.serial, 0x1234_5678);
            assert_eq!(page.seq, i as u32);
        }
    }

    #[test]
    fn batched_audio_packets_carry_the_final_granule_and_eos() {
        // Small audio packets share one page; that page carries the granule of
        // the last packet completing on it, and is flagged end-of-stream.
        let audio: Vec<(Vec<u8>, u64)> = (1..=5).map(|i| (vec![i as u8; 10], i * 512)).collect();
        let stream = mux_vorbis(7, b"\x01id", b"\x03c", b"\x05s", &audio);
        let pages = parse_pages(&stream);

        assert_eq!(reassemble(&pages).len(), 8, "3 headers + 5 audio packets");
        let last = &pages[pages.len() - 1];
        assert_eq!(last.granule, 2560, "page granule is the last packet's");
        assert_eq!(last.header_type & FLAG_EOS, FLAG_EOS);
    }

    #[test]
    fn page_granules_are_monotonic_across_many_pages() {
        // Enough packets to force several pages (>255 one-segment packets per
        // page); each page's granule must be non-decreasing.
        let audio: Vec<(Vec<u8>, u64)> = (1..=600u64).map(|i| (vec![0xEE; 4], i * 128)).collect();
        let stream = mux_vorbis(9, b"\x01id", b"\x03c", b"\x05s", &audio);
        let pages = parse_pages(&stream);

        let audio_granules: Vec<u64> = pages
            .iter()
            .map(|p| p.granule)
            .filter(|&g| g != 0)
            .collect();
        assert!(audio_granules.len() >= 2, "expected multiple audio pages");
        assert!(
            audio_granules.windows(2).all(|w| w[1] >= w[0]),
            "page granules not monotonic: {audio_granules:?}"
        );
        assert_eq!(*audio_granules.last().unwrap(), 600 * 128);
        // And the packets still all reassemble in order.
        let packets = reassemble(&pages);
        assert_eq!(packets.len(), 603);
    }

    #[test]
    fn a_packet_spanning_many_pages_reassembles() {
        // A packet longer than one page worth of segments (255 * 255 = 65025
        // payload bytes) must span pages with the continued flag, then rejoin.
        let big = vec![0x5Au8; 70_000];
        let packets = vec![
            OggPacket {
                data: b"\x01id",
                granule: 0,
                flush: true,
            },
            OggPacket {
                data: &big,
                granule: 0,
                flush: true,
            },
            OggPacket {
                data: b"end",
                granule: 4096,
                flush: true,
            },
        ];
        let stream = mux_packets(3, &packets);
        let pages = parse_pages(&stream);

        // The oversized packet forces at least one continued page.
        assert!(
            pages.iter().any(|p| p.header_type & FLAG_CONTINUED != 0),
            "no continued page emitted for the spanning packet"
        );

        let reassembled = reassemble(&pages);
        assert_eq!(reassembled.len(), 3);
        assert_eq!(reassembled[1], big);
        assert_eq!(reassembled[2], b"end");
    }

    #[test]
    fn empty_packet_list_makes_no_pages() {
        assert!(mux_packets(1, &[]).is_empty());
    }

    #[test]
    fn demux_inverts_the_muxer() {
        let id = b"\x01vorbis-id".to_vec();
        let comment = b"\x03c".to_vec();
        let setup = b"\x05setup".to_vec();
        let audio = vec![
            (vec![0u8; 0], 256u64), // an empty (silent) packet must survive
            (vec![0xABu8; 300], 512u64),
            (vec![0xCDu8; 7], 768u64),
        ];
        let stream = mux_vorbis(42, &id, &comment, &setup, &audio);
        let packets = demux(&stream).expect("demux");
        assert_eq!(packets.len(), 6);
        assert_eq!(packets[0], id);
        assert_eq!(packets[1], comment);
        assert_eq!(packets[2], setup);
        assert!(packets[3].is_empty());
        assert_eq!(packets[4], audio[1].0);
        assert_eq!(packets[5], audio[2].0);
    }

    #[test]
    fn demux_rejects_garbage() {
        assert!(demux(b"not an ogg stream at all").is_none());
        assert!(demux(b"OggS").is_none()); // truncated header
    }

    #[test]
    fn exact_multiple_of_255_terminates_with_zero_lace() {
        // A 255-byte packet needs a 255 lace then a terminating 0 lace.
        let data = vec![1u8; 255];
        let packets = vec![OggPacket {
            data: &data,
            granule: 0,
            flush: true,
        }];
        let stream = mux_packets(1, &packets);
        let pages = parse_pages(&stream);
        assert_eq!(pages[0].laces, vec![255, 0]);
        assert_eq!(reassemble(&pages)[0].len(), 255);
    }
}
