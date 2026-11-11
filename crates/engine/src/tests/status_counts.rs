//! The status bar's counts, kept block by block ([`Session::status_counts`]), against the
//! whole-document counts they replace.

use serde_json::json;
use wordcraft_doc::{Document, count_chars, count_chars_no_spaces};

use crate::Session;

/// (words, characters, characters without spaces) counted over the whole document, as the status
/// bar did on every edit before the cache.
fn whole(s: &Session) -> (usize, usize, usize) {
    let text = s.counted_text();
    (s.word_count(), count_chars(&text), count_chars_no_spaces(&text))
}

fn cached(s: &mut Session) -> (usize, usize, usize) {
    let c = s.status_counts();
    (c.words, c.chars, c.chars_no_spaces)
}

#[test]
fn status_counts_match_the_whole_document_counts_through_edits() {
    let mut s = Session::new(Document::new());
    assert_eq!(cached(&mut s), whole(&s));
    let steps: &[(&str, serde_json::Value)] = &[
        ("text.insert", json!({"text": "The quick brown fox"})),
        ("text.newParagraph", json!({})),
        ("text.insert", json!({"text": "jumps over 漢字 and かな, twice."})),
        ("para.numbering", json!({})),
        ("text.newParagraph", json!({})),
        ("text.insert", json!({"text": "a second numbered item"})),
        ("text.newParagraph", json!({})),
        ("para.numbering", json!({})),
        ("insert.table", json!({"rows": 2, "cols": 3})),
        ("text.insert", json!({"text": "cell text"})),
        ("caret.docEnd", json!({})),
        ("text.insert", json!({"text": "after the table"})),
        ("references.footnote", json!({})),
        ("text.insert", json!({"text": "a note with five words"})),
        ("caret.docStart", json!({})),
        ("text.insert", json!({"text": "Start: "})),
        ("para.bullets", json!({})),
        ("text.delete", json!({})),
        ("edit.undo", json!({})),
        ("edit.undo", json!({})),
        ("edit.redo", json!({})),
        ("edit.selectAll", json!({})),
        ("text.insert", json!({"text": "all replaced"})),
        ("edit.undo", json!({})),
    ];
    for count_notes in [true, false] {
        s.prefs.count_notes = count_notes;
        for (id, params) in steps {
            // A command that isn't available where the caret is changes nothing; the counts
            // must hold either way.
            let _ = s.run(id, params);
            assert_eq!(cached(&mut s), whole(&s), "after {id} (count_notes: {count_notes})");
        }
    }
    assert!(whole(&s).0 > 10, "the script wrote a document: {:?}", whole(&s));
}
