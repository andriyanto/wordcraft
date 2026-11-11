//! WMF and EMF pictures, as embedded in documents.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = wordcraft_metafile::parse(data);
});
