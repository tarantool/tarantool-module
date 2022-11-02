use crate::ffi::uuid as ffi;
use std::os::raw::c_char;

pub use ::uuid::{adapter, Error};
use serde::{Deserialize, Serialize};

type Inner = ::uuid::Uuid;

#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Default)]
pub struct Uuid {
    inner: Inner,
}

impl Uuid {
    #[inline(always)]
    pub fn random() -> Self {
        unsafe {
            let mut tt = std::mem::MaybeUninit::uninit();
            ffi::tt_uuid_create(tt.as_mut_ptr());
            Self::from_tt_uuid(tt.assume_init())
        }
    }

    #[inline(always)]
    pub fn from_inner(inner: Inner) -> Self {
        inner.into()
    }

    #[inline(always)]
    pub fn into_inner(self) -> Inner {
        self.into()
    }

    /// Convert an array of bytes in the big endian order into a `Uuid`.
    #[inline(always)]
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Inner::from_bytes(bytes).into()
    }

    /// Convert a slice of bytes in the big endian order into a `Uuid`. Return
    /// `None` if there's not enough bytes in the slice.
    #[inline(always)]
    pub fn try_from_slice(bytes: &[u8]) -> Option<Self> {
        std::convert::TryInto::try_into(bytes)
            .ok()
            .map(Self::from_bytes)
    }

    /// Convert the tarantool native (little endian) uuid representation into a
    /// `Uuid`.
    #[inline(always)]
    pub fn from_tt_uuid(mut tt: ffi::tt_uuid) -> Self {
        unsafe {
            tt.tl = tt.tl.swap_bytes();
            tt.tm = tt.tm.swap_bytes();
            tt.th = tt.th.swap_bytes();
            Self::from_bytes(std::mem::transmute(tt))
        }
    }

    /// Return an array of bytes in tarantool native (little endian) format
    #[inline(always)]
    pub fn to_tt_uuid(&self) -> ffi::tt_uuid {
        unsafe {
            let mut tt: ffi::tt_uuid = std::mem::transmute(*self.inner.as_bytes());
            tt.tl = tt.tl.swap_bytes();
            tt.tm = tt.tm.swap_bytes();
            tt.th = tt.th.swap_bytes();
            tt
        }
    }

    /// Return an array of bytes in the big endian order
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.inner.as_bytes()
    }

    /// The 'nil UUID'.
    ///
    /// The nil UUID is special form of UUID that is specified to have all
    /// 128 bits set to zero, as defined in [IETF RFC 4122 Section 4.1.7][RFC].
    ///
    /// [RFC]: https://tools.ietf.org/html/rfc4122.html#section-4.1.7
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use uuid::Uuid;
    ///
    /// let uuid = Uuid::nil();
    ///
    /// assert_eq!(
    ///     uuid.to_hyphenated().to_string(),
    ///     "00000000-0000-0000-0000-000000000000"
    /// );
    /// ```
    #[inline(always)]
    pub fn nil() -> Self {
        Inner::nil().into()
    }

    /// Tests if the UUID is nil.
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        self.inner.is_nil()
    }

    /// Parses a `Uuid` from a string of hexadecimal digits with optional
    /// hyphens.
    ///
    /// Any of the formats generated by this module (simple, hyphenated, urn)
    /// are supported by this parsing function.
    #[inline(always)]
    pub fn parse_str(input: &str) -> Result<Self, Error> {
        Inner::parse_str(input).map(Self::from)
    }

    /// Get a [`Hyphenated`] formatter.
    ///
    /// [`Hyphenated`]: adapter/struct.Hyphenated.html
    #[inline(always)]
    pub const fn to_hyphenated(self) -> adapter::Hyphenated {
        self.inner.to_hyphenated()
    }

    /// Get a borrowed [`HyphenatedRef`] formatter.
    ///
    /// [`HyphenatedRef`]: adapter/struct.HyphenatedRef.html
    #[inline(always)]
    pub const fn to_hyphenated_ref(&self) -> adapter::HyphenatedRef<'_> {
        self.inner.to_hyphenated_ref()
    }

    /// Get a [`Simple`] formatter.
    ///
    /// [`Simple`]: adapter/struct.Simple.html
    #[inline(always)]
    pub const fn to_simple(self) -> adapter::Simple {
        self.inner.to_simple()
    }

    /// Get a borrowed [`SimpleRef`] formatter.
    ///
    /// [`SimpleRef`]: adapter/struct.SimpleRef.html
    #[inline(always)]
    pub const fn to_simple_ref(&self) -> adapter::SimpleRef<'_> {
        self.inner.to_simple_ref()
    }

    /// Get a [`Urn`] formatter.
    ///
    /// [`Urn`]: adapter/struct.Urn.html
    #[inline(always)]
    pub const fn to_urn(self) -> adapter::Urn {
        self.inner.to_urn()
    }

    /// Get a borrowed [`UrnRef`] formatter.
    ///
    /// [`UrnRef`]: adapter/struct.UrnRef.html
    #[inline(always)]
    pub const fn to_urn_ref(&self) -> adapter::UrnRef<'_> {
        self.inner.to_urn_ref()
    }
}

impl From<Inner> for Uuid {
    #[inline(always)]
    fn from(inner: Inner) -> Self {
        Self { inner }
    }
}

impl From<Uuid> for Inner {
    #[inline(always)]
    fn from(uuid: Uuid) -> Self {
        uuid.inner
    }
}

impl std::fmt::Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::fmt::LowerHex for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.inner, f)
    }
}

impl std::fmt::UpperHex for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::UpperHex::fmt(&self.inner, f)
    }
}

impl std::str::FromStr for Uuid {
    type Err = Error;

    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        Self::parse_str(uuid_str)
    }
}

////////////////////////////////////////////////////////////////////////////////
/// Tuple
////////////////////////////////////////////////////////////////////////////////

impl serde::Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct _ExtStruct((c_char, serde_bytes::ByteBuf));

        let data = self.as_bytes();
        _ExtStruct((ffi::MP_UUID, serde_bytes::ByteBuf::from(data as &[_]))).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Uuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct _ExtStruct((c_char, serde_bytes::ByteBuf));

        let _ExtStruct((kind, bytes)) = serde::Deserialize::deserialize(deserializer)?;

        if kind != ffi::MP_UUID {
            return Err(serde::de::Error::custom(format!(
                "Expected UUID, found msgpack ext #{}",
                kind
            )));
        }

        let data = bytes.into_vec();
        Self::try_from_slice(&data).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "Not enough bytes for UUID: expected 16, got {}",
                data.len()
            ))
        })
    }
}

////////////////////////////////////////////////////////////////////////////////
/// Lua
////////////////////////////////////////////////////////////////////////////////

static mut CTID_UUID: Option<u32> = None;

fn ctid_uuid() -> u32 {
    unsafe {
        if CTID_UUID.is_none() {
            let lua = crate::global_lua();
            let ctid_uuid =
                tlua::ffi::luaL_ctypeid(tlua::AsLua::as_lua(&lua), crate::c_ptr!("struct tt_uuid"));
            assert!(ctid_uuid != 0);
            CTID_UUID = Some(ctid_uuid)
        }
        CTID_UUID.unwrap()
    }
}

impl<L> tlua::LuaRead<L> for Uuid
where
    L: tlua::AsLua,
{
    fn lua_read_at_position(lua: L, index: std::num::NonZeroI32) -> Result<Self, L> {
        let raw_lua = lua.as_lua();
        let index = index.get();
        unsafe {
            if tlua::ffi::lua_type(raw_lua, index) != tlua::ffi::LUA_TCDATA {
                return Err(lua);
            }
            let mut ctypeid = std::mem::MaybeUninit::uninit();
            let cdata = tlua::ffi::luaL_checkcdata(raw_lua, index, ctypeid.as_mut_ptr());
            if ctypeid.assume_init() != ctid_uuid() {
                return Err(lua);
            }
            Ok(Self::from_tt_uuid(*cdata.cast()))
        }
    }
}

impl<L: tlua::AsLua> tlua::Push<L> for Uuid {
    type Err = tlua::Void;

    #[inline(always)]
    fn push_to_lua(&self, lua: L) -> Result<tlua::PushGuard<L>, (Self::Err, L)> {
        tlua::PushInto::push_into_lua(*self, lua)
    }
}

impl<L: tlua::AsLua> tlua::PushOne<L> for Uuid {}

impl<L: tlua::AsLua> tlua::PushInto<L> for Uuid {
    type Err = tlua::Void;

    fn push_into_lua(self, lua: L) -> Result<tlua::PushGuard<L>, (Self::Err, L)> {
        unsafe {
            let cdata = tlua::ffi::luaL_pushcdata(lua.as_lua(), ctid_uuid());
            std::ptr::write(cdata as _, self.to_tt_uuid());
            Ok(tlua::PushGuard::new(lua, 1))
        }
    }
}

impl<L: tlua::AsLua> tlua::PushOneInto<L> for Uuid {}
