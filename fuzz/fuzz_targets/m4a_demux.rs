#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = sonare_codec::demux_m4a_as_aac_adts(data);
    if let Ok(m4a) = sonare_codec::mux_aac_adts_as_m4a(data) {
        let _ = sonare_codec::demux_m4a_as_aac_adts(&m4a);
        let _ = sonare_codec::decode(&m4a);
    }
});
