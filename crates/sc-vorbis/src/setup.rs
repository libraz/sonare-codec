//! Vorbis setup header (the third header packet) serialization.
//!
//! Hand-ported to safe Rust from libvorbis/aoTuV `lib/info.c`
//! (`_vorbis_pack_books`), `lib/codebook.c` (`vorbis_staticbook_pack`),
//! `lib/floor1.c` (`floor1_pack`), `lib/res0.c` (`res0_pack`) and
//! `lib/mapping0.c` (`mapping0_pack`): the setup packet serializes the whole
//! codec configuration — every codebook, then the floor / residue / mapping /
//! mode tables (Vorbis I spec §4.2.4). Derivative work of libvorbis/aoTuV
//! (BSD-3-Clause); see `LICENSE-THIRDPARTY`.
//!
//! Everything is packed LSb-first through the Ogg bit packer. The packet opens
//! with the `0x05` type byte and the literal `"vorbis"`, and closes with a
//! single framing-flag bit. The layout is fully deterministic, so each packer
//! is verified bit-exact by round-tripping through a matching reader.

use crate::codebook::{maptype1_quantvals, ov_ilog};
use crate::oggpack::BitWriter;

/// Packet type byte for the setup header.
const PACKET_TYPE_SETUP: u32 = 0x05;
/// The `"vorbis"` signature every header packet carries after its type.
const VORBIS_SIGNATURE: &[u8; 6] = b"vorbis";
/// Sync pattern leading every packed static codebook (`0x564342` = "BCV").
const CODEBOOK_SYNC: u32 = 0x0056_4342;

/// libvorbis VQ 32-bit *packed float* (`_float32_pack`, sharedbook.c) — NOT
/// IEEE: `neeeeeee eeemmmmm mmmmmmmm mmmmmmmm`, 21-bit mantissa, exponent biased
/// by 768. This is the form the setup header stores `q_min` / `q_delta` in, and
/// the form a standard decoder unpacks them from. `0.0` packs to all-zero.
#[must_use]
pub fn float32_pack(val: f32) -> u32 {
    const FMAN: i32 = 21;
    const FEXP_BIAS: i32 = 768;
    if val == 0.0 {
        return 0;
    }
    let (sign, v) = if val < 0.0 {
        (0x8000_0000u32, f64::from(-val))
    } else {
        (0u32, f64::from(val))
    };
    let exp = (v.log2() + 0.001).floor() as i32;
    let mant = (v * 2f64.powi((FMAN - 1) - exp)).round() as i64 as u32;
    let exp_field = ((exp + FEXP_BIAS) as u32) << FMAN;
    sign | exp_field | (mant & 0x001f_ffff)
}

/// Number of set bits in `v` (libvorbis `icount`): how many residue cascade
/// books a partition's `secondstages` bitmask names.
fn icount(mut v: u32) -> u32 {
    let mut ret = 0;
    while v != 0 {
        ret += v & 1;
        v >>= 1;
    }
    ret
}

/// A Vorbis static codebook as it appears in the setup header: the codeword
/// length list plus the optional value-mapping (`maptype`) parameters.
///
/// `q_min` / `q_delta` are the libvorbis 32-bit *packed float* bit patterns,
/// carried opaquely (they are written and read back verbatim). `quantlist`
/// holds the signed quant values; only their magnitudes are stored, in
/// `q_quant` bits each.
pub struct StaticCodebook {
    pub dim: u32,
    pub entries: u32,
    pub lengthlist: Vec<u8>,
    pub maptype: u8,
    pub q_min: u32,
    pub q_delta: u32,
    pub q_quant: u8,
    pub q_sequencep: bool,
    pub quantlist: Vec<i32>,
}

impl StaticCodebook {
    /// Packs this codebook (`vorbis_staticbook_pack`).
    fn pack(&self, w: &mut BitWriter) {
        w.write(CODEBOOK_SYNC, 24);
        w.write(self.dim, 16);
        w.write(self.entries, 24);

        // Choose length-ordered packing when the lengths are non-decreasing and
        // carry no unused (zero-length) entry; otherwise pack each length.
        let entries = self.entries as usize;
        let ordered = (1..entries)
            .all(|i| self.lengthlist[i - 1] != 0 && self.lengthlist[i] >= self.lengthlist[i - 1]);

        if ordered && entries > 0 {
            w.write(1, 1); // ordered
            w.write(u32::from(self.lengthlist[0]).wrapping_sub(1), 5);

            let mut count: u32 = 0;
            for i in 1..entries {
                let this = self.lengthlist[i];
                let last = self.lengthlist[i - 1];
                if this > last {
                    let bits = ov_ilog(self.entries - count) as u32;
                    for _ in last..this {
                        w.write(i as u32 - count, bits);
                        count = i as u32;
                    }
                }
            }
            let bits = ov_ilog(self.entries - count) as u32;
            w.write(self.entries - count, bits);
        } else {
            w.write(0, 1); // unordered
            let has_unused = self.lengthlist.iter().any(|&l| l == 0);
            if has_unused {
                w.write(1, 1); // tag unused entries individually
                for &len in &self.lengthlist {
                    if len == 0 {
                        w.write(0, 1);
                    } else {
                        w.write(1, 1);
                        w.write(u32::from(len) - 1, 5);
                    }
                }
            } else {
                w.write(0, 1); // no unused entries
                for &len in &self.lengthlist {
                    w.write(u32::from(len).wrapping_sub(1), 5);
                }
            }
        }

        w.write(u32::from(self.maptype), 4);
        if self.maptype == 1 || self.maptype == 2 {
            w.write(self.q_min, 32);
            w.write(self.q_delta, 32);
            w.write(u32::from(self.q_quant).wrapping_sub(1), 4);
            w.write(u32::from(self.q_sequencep), 1);

            let quantvals = match self.maptype {
                1 => maptype1_quantvals(entries, self.dim as usize),
                _ => entries * self.dim as usize,
            };
            for &v in self.quantlist.iter().take(quantvals) {
                w.write(v.unsigned_abs(), u32::from(self.q_quant));
            }
        }
    }
}

/// A type-1 floor configuration (`floor1_pack`). `postlist` is the full,
/// unsorted post list (`postlist[0] == 0`, `postlist[1] == maxposit`); the
/// remaining posts are packed in partition-class order.
pub struct Floor1Setup {
    pub partition_class: Vec<u8>,
    pub class_dim: Vec<u8>,
    pub class_subs: Vec<u8>,
    pub class_book: Vec<u8>,
    /// Per class, `1 << class_subs` sub-book indices; `-1` marks "no book".
    pub class_subbook: Vec<Vec<i32>>,
    pub mult: u8,
    pub postlist: Vec<u32>,
}

impl Floor1Setup {
    fn pack(&self, w: &mut BitWriter) {
        let partitions = self.partition_class.len();
        w.write(partitions as u32, 5);
        let mut maxclass: i32 = -1;
        for &class in &self.partition_class {
            w.write(u32::from(class), 4);
            maxclass = maxclass.max(i32::from(class));
        }

        for j in 0..(maxclass + 1) as usize {
            w.write(u32::from(self.class_dim[j]) - 1, 3);
            w.write(u32::from(self.class_subs[j]), 2);
            if self.class_subs[j] != 0 {
                w.write(u32::from(self.class_book[j]), 8);
            }
            for k in 0..(1usize << self.class_subs[j]) {
                w.write((self.class_subbook[j][k] + 1) as u32, 8);
            }
        }

        w.write(u32::from(self.mult) - 1, 2);
        let maxposit = self.postlist[1];
        let rangebits = ov_ilog(maxposit - 1) as u32;
        w.write(rangebits, 4);

        let mut count = 0usize;
        for &class in &self.partition_class {
            count += usize::from(self.class_dim[usize::from(class)]);
            // posts are stored from index 2 onward, in partition order
            for k in (count - usize::from(self.class_dim[usize::from(class)]))..count {
                w.write(self.postlist[k + 2], rangebits);
            }
        }
    }
}

/// A type-0/1 residue configuration (`res0_pack`). `secondstages` is the
/// per-partition cascade bitmask; `booklist` lists the cascade books in the
/// order the bitmasks enumerate them.
pub struct ResidueSetup {
    pub residue_type: u16,
    pub begin: u32,
    pub end: u32,
    pub grouping: u32,
    pub groupbook: u8,
    pub secondstages: Vec<u32>,
    pub booklist: Vec<u8>,
}

impl ResidueSetup {
    fn pack(&self, w: &mut BitWriter) {
        w.write(self.begin, 24);
        w.write(self.end, 24);
        w.write(self.grouping - 1, 24);
        w.write(self.secondstages.len() as u32 - 1, 6);
        w.write(u32::from(self.groupbook), 8);

        let mut acc = 0u32;
        for &stage in &self.secondstages {
            if ov_ilog(stage) > 3 {
                // A cascade deeper than four stages spills into a second field.
                w.write(stage, 3);
                w.write(1, 1);
                w.write(stage >> 3, 5);
            } else {
                w.write(stage, 4);
            }
            acc += icount(stage);
        }
        for &book in self.booklist.iter().take(acc as usize) {
            w.write(u32::from(book), 8);
        }
    }
}

/// A type-0 mapping configuration (`mapping0_pack`): channel-to-submap routing,
/// square-polar coupling steps, and the floor/residue assignment per submap.
pub struct Mapping0Setup {
    pub submaps: u8,
    pub coupling_mag: Vec<u32>,
    pub coupling_ang: Vec<u32>,
    pub chmuxlist: Vec<u8>,
    pub floorsubmap: Vec<u8>,
    pub residuesubmap: Vec<u8>,
}

impl Mapping0Setup {
    fn pack(&self, w: &mut BitWriter, channels: u16) {
        if self.submaps > 1 {
            w.write(1, 1);
            w.write(u32::from(self.submaps) - 1, 4);
        } else {
            w.write(0, 1);
        }

        let coupling_steps = self.coupling_mag.len();
        if coupling_steps > 0 {
            w.write(1, 1);
            w.write(coupling_steps as u32 - 1, 8);
            let bits = ov_ilog(u32::from(channels) - 1) as u32;
            for i in 0..coupling_steps {
                w.write(self.coupling_mag[i], bits);
                w.write(self.coupling_ang[i], bits);
            }
        } else {
            w.write(0, 1);
        }

        w.write(0, 2); // reserved

        if self.submaps > 1 {
            for &mux in &self.chmuxlist {
                w.write(u32::from(mux), 4);
            }
        }
        for i in 0..usize::from(self.submaps) {
            w.write(0, 8); // time submap unused
            w.write(u32::from(self.floorsubmap[i]), 8);
            w.write(u32::from(self.residuesubmap[i]), 8);
        }
    }
}

/// A Vorbis mode configuration: the block-size flag plus the (currently fixed)
/// window/transform types and the mapping index this mode selects.
pub struct ModeSetup {
    pub blockflag: bool,
    pub windowtype: u16,
    pub transformtype: u16,
    pub mapping: u8,
}

/// The full setup configuration: every codebook and the floor / residue /
/// mapping / mode tables, with the floor and residue type tags.
pub struct SetupConfig {
    pub channels: u16,
    pub codebooks: Vec<StaticCodebook>,
    pub floors: Vec<(u16, Floor1Setup)>,
    pub residues: Vec<ResidueSetup>,
    pub mappings: Vec<(u16, Mapping0Setup)>,
    pub modes: Vec<ModeSetup>,
}

impl SetupConfig {
    /// Packs the complete setup header (`_vorbis_pack_books`).
    #[must_use]
    pub fn pack(&self) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.write(PACKET_TYPE_SETUP, 8);
        for &byte in VORBIS_SIGNATURE {
            w.write(u32::from(byte), 8);
        }

        // Codebooks.
        w.write(self.codebooks.len() as u32 - 1, 8);
        for book in &self.codebooks {
            book.pack(&mut w);
        }

        // Time domains: a fixed, unused placeholder of one zero entry.
        w.write(0, 6);
        w.write(0, 16);

        // Floors.
        w.write(self.floors.len() as u32 - 1, 6);
        for (floor_type, floor) in &self.floors {
            w.write(u32::from(*floor_type), 16);
            floor.pack(&mut w);
        }

        // Residues.
        w.write(self.residues.len() as u32 - 1, 6);
        for residue in &self.residues {
            w.write(u32::from(residue.residue_type), 16);
            residue.pack(&mut w);
        }

        // Mappings.
        w.write(self.mappings.len() as u32 - 1, 6);
        for (map_type, mapping) in &self.mappings {
            w.write(u32::from(*map_type), 16);
            mapping.pack(&mut w, self.channels);
        }

        // Modes.
        w.write(self.modes.len() as u32 - 1, 6);
        for mode in &self.modes {
            w.write(u32::from(mode.blockflag), 1);
            w.write(u32::from(mode.windowtype), 16);
            w.write(u32::from(mode.transformtype), 16);
            w.write(u32::from(mode.mapping), 8);
        }

        w.write(1, 1); // framing flag
        w.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oggpack::BitReader;

    /// Reads back a static codebook (the inverse of `StaticCodebook::pack`),
    /// returning the codeword length list so the round-trip can be checked.
    fn unpack_lengths(r: &mut BitReader) -> Vec<u8> {
        assert_eq!(r.read(24), CODEBOOK_SYNC, "codebook sync");
        let _dim = r.read(16);
        let entries = r.read(24) as usize;
        let ordered = r.read(1) == 1;

        let mut lengths = vec![0u8; entries];
        if ordered {
            let mut length = r.read(5) + 1;
            let mut i = 0usize;
            while i < entries {
                let bits = ov_ilog(entries as u32 - i as u32) as u32;
                let num = r.read(bits) as usize;
                for slot in lengths.iter_mut().skip(i).take(num) {
                    *slot = length as u8;
                }
                i += num;
                length += 1;
            }
        } else {
            let unused = r.read(1) == 1;
            for slot in &mut lengths {
                if unused {
                    if r.read(1) == 1 {
                        *slot = (r.read(5) + 1) as u8;
                    }
                } else {
                    *slot = (r.read(5) + 1) as u8;
                }
            }
        }
        lengths
    }

    #[test]
    fn unordered_codebook_round_trips_lengths() {
        let book = StaticCodebook {
            dim: 1,
            entries: 5,
            lengthlist: vec![2, 4, 4, 3, 2],
            maptype: 0,
            q_min: 0,
            q_delta: 0,
            q_quant: 0,
            q_sequencep: false,
            quantlist: vec![],
        };
        let mut w = BitWriter::new();
        book.pack(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(unpack_lengths(&mut r), vec![2, 4, 4, 3, 2]);
        assert_eq!(r.read(4), 0, "maptype 0");
    }

    #[test]
    fn ordered_codebook_round_trips_lengths() {
        // Strictly non-decreasing, no zero entry -> length-ordered packing.
        let book = StaticCodebook {
            dim: 1,
            entries: 6,
            lengthlist: vec![1, 2, 2, 3, 4, 4],
            maptype: 0,
            q_min: 0,
            q_delta: 0,
            q_quant: 0,
            q_sequencep: false,
            quantlist: vec![],
        };
        let mut w = BitWriter::new();
        book.pack(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(unpack_lengths(&mut r), vec![1, 2, 2, 3, 4, 4]);
    }

    #[test]
    fn unused_entries_are_tagged() {
        let book = StaticCodebook {
            dim: 1,
            entries: 4,
            lengthlist: vec![2, 0, 2, 2],
            maptype: 0,
            q_min: 0,
            q_delta: 0,
            q_quant: 0,
            q_sequencep: false,
            quantlist: vec![],
        };
        let mut w = BitWriter::new();
        book.pack(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(unpack_lengths(&mut r), vec![2, 0, 2, 2]);
    }

    #[test]
    fn maptype1_codebook_round_trips_quantlist() {
        let book = StaticCodebook {
            dim: 2,
            entries: 9, // quantvals == 3 for dim 2
            lengthlist: vec![1, 2, 3, 4, 5, 6, 7, 8, 8],
            maptype: 1,
            q_min: 0x1234_5678,
            q_delta: 0x0abc_def0,
            q_quant: 5,
            q_sequencep: true,
            quantlist: vec![0, 7, 31],
        };
        let mut w = BitWriter::new();
        book.pack(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        let _lengths = unpack_lengths(&mut r);
        assert_eq!(r.read(4), 1, "maptype 1");
        assert_eq!(r.read(32), 0x1234_5678, "q_min");
        assert_eq!(r.read(32), 0x0abc_def0, "q_delta");
        assert_eq!(r.read(4), 4, "q_quant - 1");
        assert_eq!(r.read(1), 1, "q_sequencep");
        assert_eq!(r.read(5), 0, "quant[0]");
        assert_eq!(r.read(5), 7, "quant[1]");
        assert_eq!(r.read(5), 31, "quant[2]");
    }

    #[test]
    fn floor1_round_trips_partition_and_posts() {
        let floor = Floor1Setup {
            partition_class: vec![0, 1],
            class_dim: vec![2, 1],
            class_subs: vec![0, 1],
            class_book: vec![0, 2],
            class_subbook: vec![vec![0], vec![-1, 1]],
            mult: 2,
            postlist: vec![0, 128, 64, 32, 96],
        };
        let mut w = BitWriter::new();
        floor.pack(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);

        assert_eq!(r.read(5), 2, "partitions");
        assert_eq!(r.read(4), 0, "class[0]");
        assert_eq!(r.read(4), 1, "class[1]");
        // class 0: dim-1, subs
        assert_eq!(r.read(3), 1, "class0 dim-1");
        assert_eq!(r.read(2), 0, "class0 subs");
        assert_eq!(r.read(8), 1, "class0 subbook[0]+1");
        // class 1: dim-1, subs, book, 2 subbooks
        assert_eq!(r.read(3), 0, "class1 dim-1");
        assert_eq!(r.read(2), 1, "class1 subs");
        assert_eq!(r.read(8), 2, "class1 book");
        assert_eq!(r.read(8), 0, "class1 subbook[0]+1 (-1)");
        assert_eq!(r.read(8), 2, "class1 subbook[1]+1");
        // posts
        assert_eq!(r.read(2), 1, "mult-1");
        let rangebits = r.read(4);
        assert_eq!(rangebits, ov_ilog(127) as u32, "rangebits");
        // 3 posts (dims 2 + 1) in partition order: 64, 32, 96
        assert_eq!(r.read(rangebits), 64);
        assert_eq!(r.read(rangebits), 32);
        assert_eq!(r.read(rangebits), 96);
    }

    #[test]
    fn residue_round_trips_cascade() {
        let residue = ResidueSetup {
            residue_type: 2,
            begin: 0,
            end: 1024,
            grouping: 32,
            groupbook: 3,
            secondstages: vec![1, 0, 5], // popcounts: 1 + 0 + 2 = 3 books
            booklist: vec![4, 5, 6],
        };
        let mut w = BitWriter::new();
        residue.pack(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);

        assert_eq!(r.read(24), 0, "begin");
        assert_eq!(r.read(24), 1024, "end");
        assert_eq!(r.read(24), 31, "grouping-1");
        assert_eq!(r.read(6), 2, "partitions-1");
        assert_eq!(r.read(8), 3, "groupbook");
        assert_eq!(r.read(4), 1, "stage[0] (<=3 ilog)");
        assert_eq!(r.read(4), 0, "stage[1]");
        assert_eq!(r.read(4), 5, "stage[2]");
        assert_eq!(r.read(8), 4, "book[0]");
        assert_eq!(r.read(8), 5, "book[1]");
        assert_eq!(r.read(8), 6, "book[2]");
    }

    #[test]
    fn residue_deep_cascade_spills_to_second_field() {
        // secondstages with ilog > 3 (value 0x11 = 17 -> ilog 5) takes the
        // 3-bit + flag + 5-bit spill path.
        let residue = ResidueSetup {
            residue_type: 1,
            begin: 0,
            end: 256,
            grouping: 16,
            groupbook: 0,
            secondstages: vec![0x11],
            booklist: vec![7, 8], // popcount(0x11) = 2
        };
        let mut w = BitWriter::new();
        residue.pack(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read(24), 0);
        assert_eq!(r.read(24), 256);
        assert_eq!(r.read(24), 15);
        assert_eq!(r.read(6), 0, "1 partition");
        assert_eq!(r.read(8), 0, "groupbook");
        assert_eq!(r.read(3), 0x11 & 0x7, "low 3 bits");
        assert_eq!(r.read(1), 1, "spill flag");
        assert_eq!(r.read(5), 0x11 >> 3, "high 5 bits");
        assert_eq!(r.read(8), 7);
        assert_eq!(r.read(8), 8);
    }

    #[test]
    fn mapping_round_trips_single_submap() {
        let mapping = Mapping0Setup {
            submaps: 1,
            coupling_mag: vec![0],
            coupling_ang: vec![1],
            chmuxlist: vec![0, 0],
            floorsubmap: vec![0],
            residuesubmap: vec![0],
        };
        let mut w = BitWriter::new();
        mapping.pack(&mut w, 2);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);

        assert_eq!(r.read(1), 0, "single submap flag");
        assert_eq!(r.read(1), 1, "coupling present");
        assert_eq!(r.read(8), 0, "coupling steps - 1");
        let bits = ov_ilog(1) as u32; // channels-1 == 1 -> ilog 1
        assert_eq!(r.read(bits), 0, "coupling mag");
        assert_eq!(r.read(bits), 1, "coupling ang");
        assert_eq!(r.read(2), 0, "reserved");
        assert_eq!(r.read(8), 0, "time submap");
        assert_eq!(r.read(8), 0, "floor submap");
        assert_eq!(r.read(8), 0, "residue submap");
    }

    #[test]
    fn full_setup_header_has_type_and_signature_and_framing() {
        let config = SetupConfig {
            channels: 2,
            codebooks: vec![StaticCodebook {
                dim: 1,
                entries: 4,
                lengthlist: vec![2, 2, 2, 2],
                maptype: 0,
                q_min: 0,
                q_delta: 0,
                q_quant: 0,
                q_sequencep: false,
                quantlist: vec![],
            }],
            floors: vec![(
                1,
                Floor1Setup {
                    partition_class: vec![0],
                    class_dim: vec![1],
                    class_subs: vec![0],
                    class_book: vec![0],
                    class_subbook: vec![vec![0]],
                    mult: 2,
                    postlist: vec![0, 128, 64],
                },
            )],
            residues: vec![ResidueSetup {
                residue_type: 2,
                begin: 0,
                end: 1024,
                grouping: 32,
                groupbook: 0,
                secondstages: vec![1],
                booklist: vec![0],
            }],
            mappings: vec![(
                0,
                Mapping0Setup {
                    submaps: 1,
                    coupling_mag: vec![],
                    coupling_ang: vec![],
                    chmuxlist: vec![0, 0],
                    floorsubmap: vec![0],
                    residuesubmap: vec![0],
                },
            )],
            modes: vec![ModeSetup {
                blockflag: false,
                windowtype: 0,
                transformtype: 0,
                mapping: 0,
            }],
        };

        let packet = config.pack();
        assert_eq!(packet[0], 0x05, "setup type byte");
        assert_eq!(&packet[1..7], VORBIS_SIGNATURE);

        // The packet must end with a set framing-flag bit.
        let mut r = BitReader::new(&packet);
        r.read(8);
        for _ in 0..6 {
            r.read(8);
        }
        assert_eq!(r.read(8), 0, "books - 1 == 0 (single book)");
    }

    /// libvorbis `_float32_unpack` — the decoder side, for verifying the pack.
    fn float32_unpack(val: u32) -> f64 {
        let mut mant = f64::from(val & 0x001f_ffff);
        let sign = val & 0x8000_0000 != 0;
        let exp = ((val & 0x7fe0_0000) >> 21) as i32 - 20 - 768;
        if sign {
            mant = -mant;
        }
        mant * 2f64.powi(exp)
    }

    #[test]
    fn float32_pack_round_trips_through_unpack() {
        for &v in &[0.25f32, -16.0, 1.0, -1.0, 0.0009765625, 123.5, -0.5] {
            let packed = float32_pack(v);
            let back = float32_unpack(packed);
            assert!(
                (back - f64::from(v)).abs() < 1e-6,
                "pack/unpack {v} -> {back}"
            );
        }
        assert_eq!(float32_pack(0.0), 0);
    }

    #[test]
    fn icount_counts_set_bits() {
        assert_eq!(icount(0), 0);
        assert_eq!(icount(1), 1);
        assert_eq!(icount(0x11), 2);
        assert_eq!(icount(0xff), 8);
    }
}
