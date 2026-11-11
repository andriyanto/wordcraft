//! Word 97–2003 binary documents (`.doc`): read any bytes, then write what was read as `.docx`.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(doc) = wordcraft_docbin::read(data) {
        let _ = wordcraft_docx::write(&doc);
    }
});
