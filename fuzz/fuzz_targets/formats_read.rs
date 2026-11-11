//! The other formats (RTF, ODT, flat ODT, HTML, MHT, Markdown, LaTeX, plain text): the first byte
//! picks the format, the rest is the file.
#![no_main]

use libfuzzer_sys::fuzz_target;

const FORMATS: &[&str] = &["rtf", "odt", "fodt", "html", "mht", "md", "tex", "txt"];

fuzz_target!(|data: &[u8]| {
    let Some((pick, bytes)) = data.split_first() else { return };
    let ext = FORMATS[usize::from(*pick) % FORMATS.len()];
    if let Some(Ok(doc)) = wordcraft_formats::import(ext, bytes) {
        let _ = wordcraft_docx::write(&doc);
    }
});
