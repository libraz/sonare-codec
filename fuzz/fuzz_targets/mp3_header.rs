#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(header) = sc_mp3::FrameHeader::parse(data) {
        let _ = header.frame_len();
        let _ = header.samples_per_frame();
        let _ = header.to_bytes();
        let side_info = sc_mp3::Layer3SideInfo::silent(&header);
        let _ = side_info.pack(&header);
        let main_data_len = data.get(4).copied().unwrap_or_default() as usize % 16;
        let main_data = data.get(5..5 + main_data_len).unwrap_or(&[]);
        let _ = sc_mp3::assemble_layer3_frame(header, &side_info, main_data);
    }
    let _ = sc_mp3::crc16_mpeg_audio(data);
    let mut writer = sc_mp3::BitWriter::new();
    if let Some((&first, rest)) = data.split_first() {
        let _ = writer.write_bits(u32::from(first & 0x0f), 4);
        let _ = writer.write_bytes(rest.get(..rest.len().min(8)).unwrap_or(rest));
        let _ = writer.bit_len();
        let _ = writer.finish_byte_aligned();
    }
    let _ = sonare_codec::decode(data);
});
