//! Checking a layout against the document laid out from scratch.
//!
//! [`layout`](crate::layout) reuses what it worked out before (each paragraph's lines, and from
//! there on whatever a later stage adds). Reuse must never change the result: the pages are a
//! function of the document and the options alone. With [`verify_against_full`] on, every call
//! also lays the document out with nothing to reuse and compares the two, page by page; a
//! difference is kept for [`take_mismatch`]. It doubles the cost of a layout, so it is for tests:
//! a script of random edits with the check on finds the edit after which reuse went wrong.

use std::cell::{Cell, RefCell};

use crate::{DocLayout, Page};

thread_local! {
    static VERIFY: Cell<bool> = const { Cell::new(false) };
    static MISMATCH: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Turn the check on or off for this thread.
pub fn verify_against_full(on: bool) {
    VERIFY.with(|v| v.set(on));
}

pub(crate) fn verifying() -> bool {
    VERIFY.with(Cell::get)
}

/// The first difference found since the last call, if any: which page, and the text of the two
/// layouts around where they part.
pub fn take_mismatch() -> Option<String> {
    MISMATCH.with(|m| m.borrow_mut().take())
}

pub(crate) fn record(got: &DocLayout, full: &DocLayout) {
    if let Err(why) = same_layout(got, full) {
        MISMATCH.with(|m| {
            m.borrow_mut().get_or_insert(why);
        });
    }
}

/// Whether two layouts are the same pages with the same index; `Err` says where they differ.
pub fn same_layout(a: &DocLayout, b: &DocLayout) -> Result<(), String> {
    if a.pages.len() != b.pages.len() {
        return Err(format!("{} pages against {}", a.pages.len(), b.pages.len()));
    }
    for (n, (pa, pb)) in a.pages.iter().zip(&b.pages).enumerate() {
        same_page(pa, pb).map_err(|why| format!("page {}: {why}", n + 1))?;
    }
    let sorted = |l: &DocLayout| {
        let mut v: Vec<String> = l.index.iter().map(|e| format!("{e:?}")).collect();
        v.sort();
        v
    };
    if sorted(a) != sorted(b) {
        return Err("the same pages, but the index of paragraphs differs".into());
    }
    if a.text_boxes != b.text_boxes {
        return Err(format!("{} text boxes laid out against {}", a.text_boxes, b.text_boxes));
    }
    Ok(())
}

fn same_page(a: &Page, b: &Page) -> Result<(), String> {
    let (sa, sb) = (format!("{a:?}"), format!("{b:?}"));
    if sa == sb {
        return Ok(());
    }
    // Where the two descriptions part, on a character boundary, with a little of what is around.
    let at = sa.bytes().zip(sb.bytes()).position(|(x, y)| x != y).unwrap_or(sa.len().min(sb.len()));
    let around = |s: &str| {
        let mut from = at.saturating_sub(120);
        while !s.is_char_boundary(from) {
            from -= 1;
        }
        let mut to = (at + 120).min(s.len());
        while !s.is_char_boundary(to) {
            to += 1;
        }
        s.get(from..to).unwrap_or("").to_string()
    };
    Err(format!("differs at character {at}:\n  got:  …{}…\n  full: …{}…", around(&sa), around(&sb)))
}
