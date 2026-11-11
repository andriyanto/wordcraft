//! Formatting shared between the paragraphs and runs that have the same.
//!
//! A [`ParaProps`] is 472 bytes and a [`CharProps`] 368, nearly all of it `None` in most
//! paragraphs and runs, and a document has only a handful of distinct ones. (A table's
//! [`TableProps`] and its cells' [`CellProps`] are shared the same way: a table's cells mostly
//! have the same, and a table held inline made every block in the document as large as one.) Held inline, they made
//! a paragraph 928 bytes and a run 376 whatever they said, so a file of a million empty paragraphs
//! took over a gigabyte. [`Shared`] holds one behind a counted pointer instead: equal formatting is
//! one allocation however many paragraphs and runs carry it, and unset formatting is one allocation
//! for the whole process.
//!
//! It reads like the value it holds ([`Deref`](std::ops::Deref)) and is changed like it
//! ([`DerefMut`](std::ops::DerefMut)): the first change to formatting something else also holds
//! copies it first, so a change never reaches another paragraph or run.

use std::cell::RefCell;
use std::sync::{Arc, LazyLock};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::props::{CellProps, CharProps, ParaProps, TableProps};

/// Most recently shared values remembered per thread and type (see [`Shared::new`]).
const RECENT: usize = 16;

/// A kind of formatting [`Shared`] can hold.
pub trait Shareable: Clone + PartialEq + Sized + 'static {
    /// The one allocation every unset value of this type shares.
    fn unset() -> &'static Arc<Self>;
    /// Run `f` on this thread's recently shared values, most recent first.
    fn recent<R>(f: impl FnOnce(&mut Vec<Arc<Self>>) -> R) -> R;
}

macro_rules! shareable {
    ($ty:ty) => {
        impl Shareable for $ty {
            fn unset() -> &'static Arc<Self> {
                static UNSET: LazyLock<Arc<$ty>> = LazyLock::new(|| Arc::new(<$ty>::default()));
                &UNSET
            }
            fn recent<R>(f: impl FnOnce(&mut Vec<Arc<Self>>) -> R) -> R {
                thread_local! {
                    static RECENT_VALUES: RefCell<Vec<Arc<$ty>>> = const { RefCell::new(Vec::new()) };
                }
                RECENT_VALUES.with(|r| f(&mut r.borrow_mut()))
            }
        }
    };
}

shareable!(ParaProps);
shareable!(CharProps);
shareable!(CellProps);
shareable!(TableProps);

/// Formatting held once for every paragraph or run that has the same (see the module).
pub struct Shared<T: Shareable>(Arc<T>);

impl<T: Shareable> Shared<T> {
    /// `value`, sharing the allocation of an equal value where one is at hand: the unset value's,
    /// or that of one of the last few values shared on this thread. Runs of paragraphs with the
    /// same formatting (the usual case in a document) so come to one allocation as they are read.
    pub fn new(value: T) -> Self {
        let unset = T::unset();
        if value == **unset {
            return Shared(unset.clone());
        }
        T::recent(|recent| {
            if let Some(at) = recent.iter().position(|r| **r == value) {
                let found = recent.remove(at);
                recent.insert(0, found.clone());
                return Shared(found);
            }
            let fresh = Arc::new(value);
            recent.truncate(RECENT - 1);
            recent.insert(0, fresh.clone());
            Shared(fresh)
        })
    }

    /// The value itself: taken when nothing else holds it, copied otherwise.
    pub fn into_inner(self) -> T {
        Arc::unwrap_or_clone(self.0)
    }

    /// Whether `a` and `b` are the same allocation (so certainly equal).
    pub fn same(a: &Self, b: &Self) -> bool {
        Arc::ptr_eq(&a.0, &b.0)
    }
}

impl<T: Shareable> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

impl<T: Shareable> Default for Shared<T> {
    fn default() -> Self {
        Shared(T::unset().clone())
    }
}

impl<T: Shareable> std::ops::Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Shareable> std::ops::DerefMut for Shared<T> {
    /// The value to change: copied first when something else holds it too.
    fn deref_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }
}

impl<T: Shareable> AsRef<T> for Shared<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: Shareable> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Shared::new(value)
    }
}

impl<T: Shareable> From<&T> for Shared<T> {
    fn from(value: &T) -> Self {
        Shared::new(value.clone())
    }
}

impl<T: Shareable> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || *self.0 == *other.0
    }
}

impl<T: Shareable> PartialEq<T> for Shared<T> {
    fn eq(&self, other: &T) -> bool {
        *self.0 == *other
    }
}

impl<T: Shareable + std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (*self.0).fmt(f)
    }
}

impl<T: Shareable + Serialize> Serialize for Shared<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        (*self.0).serialize(s)
    }
}

impl<'de, T: Shareable + Deserialize<'de>> Deserialize<'de> for Shared<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Shared::new)
    }
}

impl Shared<ParaProps> {
    /// For serde's `skip_serializing_if`.
    pub fn is_unset(this: &Self) -> bool {
        this.is_empty()
    }
}

impl Shared<CharProps> {
    /// For serde's `skip_serializing_if`.
    pub fn is_unset(this: &Self) -> bool {
        this.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_formatting_is_one_allocation() {
        let (a, b) = (Shared::<ParaProps>::default(), Shared::new(ParaProps::default()));
        assert!(Shared::same(&a, &b));
        assert!(Shared::<ParaProps>::is_unset(&a));
    }

    #[test]
    fn equal_formatting_shares_and_a_change_copies() {
        let bold = CharProps { bold: Some(true), ..Default::default() };
        let (a, mut b) = (Shared::new(bold.clone()), Shared::new(bold.clone()));
        assert!(Shared::same(&a, &b), "the same formatting, read twice");
        b.italic = Some(true);
        assert!(!Shared::same(&a, &b));
        assert_eq!(*a, bold, "a change to one doesn't reach the other");
        assert_eq!(b.italic, Some(true));
        assert!(a != b && a == bold);
    }

    #[test]
    fn changing_unset_formatting_leaves_the_shared_unset_value_alone() {
        let mut p = Shared::new(ParaProps::default());
        p.keep_next = Some(true);
        assert!(Shared::<ParaProps>::default().is_empty());
        assert_eq!(p.keep_next, Some(true));
    }

    #[test]
    fn it_serializes_as_the_value_it_holds() {
        let bold = CharProps { bold: Some(true), ..Default::default() };
        let json = serde_json::to_string(&Shared::new(bold.clone())).unwrap();
        assert_eq!(json, serde_json::to_string(&bold).unwrap());
        let back: Shared<CharProps> = serde_json::from_str(&json).unwrap();
        assert_eq!(*back, bold);
    }
}
