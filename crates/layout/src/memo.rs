//! What a pagination pass works out again and again from the same inputs, remembered.
//!
//! A pass visits every paragraph of the document on every keystroke, and several of the things it
//! asks of each one are answered from its formatting alone. Paragraphs with the same formatting
//! share it ([`Shared`]), so an answer is kept by the formatting's address for as long as the
//! styles are the same; the formatting is held with its answer, so its address is no other's
//! while the entry is kept.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;

use wordcraft_doc::props::ParaProps;
use wordcraft_doc::resolve::ResolvedPara;
use wordcraft_doc::styles::StyleSheet;
use wordcraft_doc::{Document, Shared};

/// Most formattings remembered; past it the memo starts over.
const MAX: usize = 4096;

#[derive(Default)]
struct Resolved {
    styles: Option<Arc<StyleSheet>>,
    by_props: HashMap<usize, (Shared<ParaProps>, Arc<ResolvedPara>), FastBuild>,
}

thread_local! {
    static RESOLVED: RefCell<Resolved> = RefCell::new(Resolved::default());
}

/// `props` resolved through the document's styles ([`StyleSheet::resolve_para`]): the paragraph
/// formatting outside any table style. Resolving walks the style chain and copies the formatting
/// several times over, which was a quarter of a keystroke's layout in a long document.
pub(crate) fn resolved_para(doc: &Document, props: &Shared<ParaProps>) -> Arc<ResolvedPara> {
    RESOLVED.with(|memo| {
        let mut memo = memo.borrow_mut();
        if !memo.styles.as_ref().is_some_and(|s| Arc::ptr_eq(s, &doc.styles)) || memo.by_props.len() >= MAX {
            memo.by_props.clear();
            memo.styles = Some(doc.styles.clone());
        }
        let key = Shared::as_ptr(props) as usize;
        if let Some((_, r)) = memo.by_props.get(&key) {
            return r.clone();
        }
        let r = Arc::new(doc.styles.resolve_para(props));
        memo.by_props.insert(key, (props.clone(), r.clone()));
        r
    })
}

/// A hasher for the layout's own maps (FxHash's mixing). Their keys are the layout's own values,
/// not text a file chooses, and they are looked up for every paragraph on every pass: the
/// standard library's keyed hasher was a tenth of a keystroke's layout.
#[derive(Default, Clone, Copy)]
pub struct FastHasher(u64);

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl FastHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for FastHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for w in words {
            self.add(u64::from_le_bytes(*w));
        }
        if !rest.is_empty() {
            let mut w = [0u8; 8];
            for (to, from) in w.iter_mut().zip(rest) {
                *to = *from;
            }
            self.add(u64::from_le_bytes(w));
        }
    }
    fn write_u8(&mut self, n: u8) {
        self.add(u64::from(n));
    }
    fn write_u16(&mut self, n: u16) {
        self.add(u64::from(n));
    }
    fn write_u32(&mut self, n: u32) {
        self.add(u64::from(n));
    }
    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }
    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

pub type FastBuild = BuildHasherDefault<FastHasher>;
pub(crate) type FastMap<K, V> = HashMap<K, V, FastBuild>;
pub(crate) type FastSet<K> = std::collections::HashSet<K, FastBuild>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_formatting_is_resolved_once_and_new_styles_start_over() {
        let mut doc = Document::new();
        let props: Shared<ParaProps> = ParaProps { keep_next: Some(true), ..Default::default() }.into();
        let (a, b) = (resolved_para(&doc, &props), resolved_para(&doc, &props));
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(a.keep_next, doc.styles.resolve_para(&props).keep_next);
        // A change to the styles is a new style sheet: nothing resolved with the old one is used.
        Arc::make_mut(&mut doc.styles).default_para.space_after = Some(99.0);
        let c = resolved_para(&doc, &props);
        assert!(!Arc::ptr_eq(&a, &c));
        assert_eq!(c.space_after, 99.0);
    }

    #[test]
    fn the_fast_hasher_tells_values_apart() {
        use std::hash::BuildHasher;
        let h = |v: &(u64, u32, Option<String>)| FastBuild::default().hash_one(v);
        let base = (7u64, 3u32, Some("1.|18|18".to_string()));
        assert_eq!(h(&base), h(&base.clone()));
        assert_ne!(h(&base), h(&(8, 3, base.2.clone())));
        assert_ne!(h(&base), h(&(7, 4, base.2.clone())));
        assert_ne!(h(&base), h(&(7, 3, Some("2.|18|18".to_string()))));
        assert_ne!(h(&base), h(&(7, 3, None)));
    }
}
