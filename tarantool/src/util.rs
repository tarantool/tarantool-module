use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::ffi::CString;

pub trait IntoClones<Tuple>: Clone {
    fn into_clones(self) -> Tuple;
}

macro_rules! impl_into_clones {
    // [@clones(self) T (...)] => [(... self,)]
    [@clones($self:ident) $h:ident ($($code:tt)*)] => { ($($code)* $self,) };
    // [@clones(self) T T ... T (...)] => [@clones(self) T ... T (... self.clone(),)]
    [@clones($self:ident) $h:ident $($t:ident)+ ($($code:tt)*)] => {
        impl_into_clones![
            @clones($self) $($t)+ ($($code)* $self.clone(),)
        ]
    };
    {$h:ident $($t:ident)*} => {
        impl<$h: Clone> IntoClones<($h $(, $t)*,)> for $h {
            fn into_clones(self) -> ($h $(, $t)*,) {
                // [@clones(self) T T ... T ()]
                impl_into_clones![@clones(self) $h $($t)* ()]
            }
        }
        impl_into_clones!{$($t)*}
    };
    () => {};
}

impl_into_clones! {T T T T T T T T T T T}

#[macro_export]
macro_rules! tuple_from_box_api {
    ($f:path [ $($args:expr),* , @out ]) => {
        {
            let mut result = ::std::mem::MaybeUninit::uninit();
            #[allow(unused_unsafe)]
            unsafe {
                if $f($($args),*, result.as_mut_ptr()) < 0 {
                    return Err($crate::error::TarantoolError::last().into());
                }
                Ok($crate::tuple::Tuple::try_from_ptr(result.assume_init()))
            }
        }
    }
}

#[macro_export]
macro_rules! expr_count {
    () => { 0 };
    ($head:expr $(, $tail:expr)*) => { 1 + $crate::expr_count!($($tail),*) }
}

#[inline]
pub fn rmp_to_vec<T>(val: &T) -> Result<Vec<u8>, Error>
where
    T: Serialize + ?Sized,
{
    Ok(rmp_serde::to_vec(val)?)
}

#[derive(Clone, Debug, Serialize, Deserialize, tlua::Push, PartialEq, Eq)]
#[serde(untagged)]
pub enum NumOrStr {
    Num(u32),
    // TODO(gmoshkin): this should be a `&str` instead, but
    // `#[derive(tlua::Push)]` doesn't support generic parameters yet
    Str(String),
}

impl Default for NumOrStr {
    fn default() -> Self {
        Self::Num(0)
    }
}

impl From<u32> for NumOrStr {
    #[inline(always)]
    fn from(n: u32) -> Self {
        Self::Num(n)
    }
}

impl From<String> for NumOrStr {
    #[inline(always)]
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}

impl From<NumOrStr> for String {
    #[inline(always)]
    fn from(s: NumOrStr) -> Self {
        match s {
            NumOrStr::Str(s) => s,
            NumOrStr::Num(n) => n.to_string(),
        }
    }
}

impl<'a> From<&'a str> for NumOrStr {
    #[inline(always)]
    fn from(s: &'a str) -> Self {
        Self::Str(s.into())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Value<'a> {
    Num(u32),
    Double(f64),
    Str(Cow<'a, str>),
    Bool(bool),
}

impl std::hash::Hash for Value<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Num(v) => v.hash(state),
            Self::Double(v) => v.to_bits().hash(state),
            Self::Str(v) => v.hash(state),
            Self::Bool(v) => v.hash(state),
        }
    }
}

impl Eq for Value<'_> {}

#[rustfmt::skip]
impl From<bool> for Value<'_> { fn from(v: bool) -> Self { Self::Bool(v) } }
#[rustfmt::skip]
impl From<u32> for Value<'_> { fn from(v: u32) -> Self { Self::Num(v) } }
#[rustfmt::skip]
impl From<f64> for Value<'_> { fn from(v: f64) -> Self { Self::Double(v) } }
#[rustfmt::skip]
impl From<String> for Value<'_> { fn from(v: String) -> Self { Self::Str(v.into()) } }
#[rustfmt::skip]
impl<'s> From<&'s str> for Value<'s> { fn from(v: &'s str) -> Self { Self::Str(v.into()) } }

#[macro_export]
macro_rules! unwrap_or {
    ($o:expr, $else:expr) => {
        if let Some(v) = $o {
            v
        } else {
            $else
        }
    };
}

#[macro_export]
macro_rules! unwrap_ok_or {
    ($o:expr, $err:pat => $($else:tt)+) => {
        match $o {
            Ok(v) => v,
            $err => $($else)+,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// DisplayAsHexBytes
////////////////////////////////////////////////////////////////////////////////

/// A wrapper for displaying byte slices as hexadecimal byte slice literals.
/// ```no_run
/// # use tarantool::util::DisplayAsHexBytes;
/// let s = format!("{}", DisplayAsHexBytes(&[1, 2, 3]));
/// assert_eq!(s, r#"b"\x01\x02\x03""#);
/// ```
pub struct DisplayAsHexBytes<'a>(pub &'a [u8]);

impl std::fmt::Display for DisplayAsHexBytes<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "b\"")?;
        for byte in self.0 {
            write!(f, "\\x{byte:02x}")?;
        }
        write!(f, "\"")?;
        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////
// str_eq
////////////////////////////////////////////////////////////////////////////////

/// Compares strings for equality.
///
/// Works at compile time unlike [`std::cmp::Eq`].
pub const fn str_eq(lhs: &str, rhs: &str) -> bool {
    let lhs = lhs.as_bytes();
    let rhs = rhs.as_bytes();
    if lhs.len() != rhs.len() {
        return false;
    }
    let mut i = 0;
    loop {
        if i == lhs.len() {
            return true;
        }
        if lhs[i] != rhs[i] {
            return false;
        }
        i += 1;
    }
}

////////////////////////////////////////////////////////////////////////////////
// to_cstring
////////////////////////////////////////////////////////////////////////////////

/// Convert `s` to a `CString` replacing any nul-bytes with `'�'` symbols.
///
/// Use this function when you need to unconditionally convert a rust string to
/// a c string without failing for any reason (other then out-of-memory), for
/// example when converting error messages.
#[inline(always)]
pub(crate) fn to_cstring_lossy(s: &str) -> CString {
    into_cstring_lossy(s.into())
}

/// Convert `s` into a `CString` replacing any nul-bytes with `'�'` symbols.
///
/// Use this function when you need to unconditionally convert a rust string to
/// a c string without failing for any reason (other then out-of-memory), for
/// example when converting error messages.
#[inline]
pub(crate) fn into_cstring_lossy(s: String) -> CString {
    match CString::new(s) {
        Ok(cstring) => cstring,
        Err(e) => {
            // Safety: the already Vec was a String a moment earlier
            let s = unsafe { String::from_utf8_unchecked(e.into_vec()) };
            // The same character String::from_utf8_lossy uses to replace non-utf8 bytes
            let s = s.replace('\0', "�");
            // Safety: s no longer contains any nul bytes.
            unsafe { CString::from_vec_unchecked(s.into()) }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// test
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use super::*;

    #[rustfmt::skip]
    #[test]
    fn check_to_cstring_lossy() {
        let message = String::from("hell\0 w\0rld\0");
        assert!(message.as_bytes().contains(&0));
        assert_eq!(to_cstring_lossy(&message).as_ref(), crate::c_str!("hell� w�rld�"));

        assert_eq!(into_cstring_lossy(message).as_ref(), crate::c_str!("hell� w�rld�"));
    }
}
