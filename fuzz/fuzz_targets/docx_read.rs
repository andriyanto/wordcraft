//! `.docx` and Word XML: read any bytes, and write back whatever was read (the writer must take
//! every document the reader can produce).
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for doc in [wordcraft_docx::read(data), wordcraft_docx::read_xml(data)].into_iter().flatten() {
        let _ = wordcraft_docx::write(&doc);
    }
});
