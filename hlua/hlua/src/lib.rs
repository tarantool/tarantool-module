//! High-level zero-cost bindings for Lua
//!
//! Lua is an interpreted programming language. This crate allows you to execute Lua code.
//!
//! # General usage
//!
//! In order to execute Lua code you first need a *Lua context*, which is represented in this
//! library with [the `Lua` struct](struct.Lua.html). You can then call
//! [the `execute` method](struct.Lua.html#method.execute) on this object.
//!
//! For example:
//!
//! ```
//! use hlua::Lua;
//!
//! let mut lua = Lua::new();
//! lua.execute::<()>("a = 12 * 5").unwrap();
//! ```
//!
//! This example puts the value `60` in the global variable `a`. The values of all global variables
//! are stored within the `Lua` struct. If you execute multiple Lua scripts on the same context,
//! each script will have access to the same global variables that were modified by the previous
//! scripts.
//!
//! In order to do something actually useful with Lua, we will need to make Lua and Rust
//! communicate with each other. This can be done in four ways:
//!
//! - You can use methods on the `Lua` struct to read or write the values of global variables with
//!   the [`get`](struct.Lua.html#method.get) and [`set`](struct.Lua.html#method.set) methods. For
//!   example you can write to a global variable with a Lua script then read it from Rust, or you
//!   can write to a global variable from Rust then read it from a Lua script.
//!
//! - The Lua script that you execute with the [`execute`](struct.Lua.html#method.execute) method
//!   can return a value.
//!
//! - You can set the value of a global variable to a Rust functions or closures, which can then be
//!   invoked with a Lua script. See [the `Function` struct](struct.Function.html) for more
//!   information. For example if you set the value of the global variable `foo` to a Rust
//!   function, you can then call it from Lua with `foo()`.
//!
//! - Similarly you can set the value of a global variable to a Lua function, then call it from
//!   Rust. The function call can return a value.
//!
//! Which method(s) you use depends on which API you wish to expose to your Lua scripts.
//!
//! # Pushing and loading values
//!
//! The interface between Rust and Lua involves two things:
//!
//! - Sending values from Rust to Lua, which is known as *pushing* the value.
//! - Sending values from Lua to Rust, which is known as *loading* the value.
//!
//! Pushing (ie. sending from Rust to Lua) can be done with
//! [the `set` method](struct.Lua.html#method.set):
//!
//! ```
//! # use hlua::Lua;
//! # let mut lua = Lua::new();
//! lua.set("a", 50);
//! ```
//!
//! You can push values that implement [the `Push` trait](trait.Push.html) or
//! [the `PushOne` trait](trait.PushOne.html) depending on the situation:
//!
//! - Integers, floating point numbers and booleans.
//! - `String` and `&str`.
//! - Any Rust function or closure whose parameters and loadable and whose return type is pushable.
//!   See the documentation of [the `Function` struct](struct.Function.html) for more information.
//! - [The `AnyLuaValue` struct](struct.AnyLuaValue.html). This enumeration represents any possible
//!   value in Lua.
//! - The [`LuaCode`](struct.LuaCode.html) and
//!   [`LuaCodeFromReader`](struct.LuaCodeFromReader.html) structs. Since pushing these structs can
//!   result in an error, you need to use [`checked_set`](struct.Lua.html#method.checked_set)
//!   instead of `set`.
//! - `Vec`s and `HashMap`s whose content is pushable.
//! - As a special case, `Result` can be pushed only as the return type of a Rust function or
//!   closure. If they contain an error, the Rust function call is considered to have failed.
//! - As a special case, tuples can be pushed when they are the return type of a Rust function or
//!   closure. They implement `Push` but not `PushOne`.
//! - TODO: userdata
//!
//! Loading (ie. sending from Lua to Rust) can be done with
//! [the `get` method](struct.Lua.html#method.get):
//!
//! ```no_run
//! # use hlua::Lua;
//! # let mut lua = Lua::new();
//! let a: i32 = lua.get("a").unwrap();
//! ```
//!
//! You can load values that implement [the `LuaRead` trait](trait.LuaRead.html):
//!
//! - Integers, floating point numbers and booleans.
//! - `String` and [`StringInLua`](struct.StringInLua.html) (ie. the equivalent of `&str`). Loading
//!   the latter has no cost while loading a `String` performs an allocation.
//! - Any function (Lua or Rust), with [the `LuaFunction` struct](struct.LuaFunction.html). This
//!   can then be used to execute the function.
//! - [The `AnyLuaValue` struct](struct.AnyLuaValue.html). This enumeration represents any possible
//!   value in Lua.
//! - [The `LuaTable` struct](struct.LuaTable.html). This struct represents a table in Lua, where
//!   keys and values can be of different types. The table can then be iterated and individual
//!   elements can be loaded or modified.
//! - As a special case, tuples can be loaded when they are the return type of a Lua function or as
//!   the return type of [`execute`](struct.Lua.html#method.execute).
//! - TODO: userdata
//!
use std::ffi::{CStr, CString};
use std::io::Read;
use std::io::Error as IoError;
use std::borrow::Borrow;
use std::num::NonZeroI32;
use std::error::Error;
use std::fmt;
use std::convert::From;
use std::io;

pub use any::{AnyHashableLuaValue, AnyLuaString, AnyLuaValue};
pub use functions_write::{Function, InsideCallback};
pub use functions_write::{function0, function1, function2, function3, function4, function5};
pub use functions_write::{function6, function7, function8, function9, function10};
pub use lua_functions::LuaFunction;
pub use lua_functions::LuaFunctionCallError;
pub use lua_functions::{LuaCode, LuaCodeFromReader};
pub use lua_tables::{LuaTable, LuaTableIterator, MethodCallError};
pub use tuples::TuplePushError;
pub use userdata::UserdataOnStack;
pub use userdata::{push_userdata, read_userdata, push_some_userdata};
pub use values::StringInLua;

pub type LuaTableMap = std::collections::HashMap<AnyHashableLuaValue, AnyLuaValue>;
pub type LuaSequence = Vec<AnyLuaValue>;

// Needed for `lua_error` macro
pub use ffi::luaL_error;

mod any;
pub mod debug;
mod functions_write;
mod lua_functions;
mod lua_tables;
mod macros;
mod rust_tables;
mod userdata;
mod values;
mod tuples;

type LuaState = *mut ffi::lua_State;

/// Main object of the library.
///
/// The lifetime parameter corresponds to the lifetime of the content of the Lua context.
///
/// # About panic safety
///
/// This type isn't panic safe. This means that if a panic happens while you were using the `Lua`,
/// then it will probably stay in a corrupt state. Trying to use the `Lua` again will most likely
/// result in another panic but shouldn't result in unsafety.
#[derive(Debug)]
pub struct Lua {
    lua: *mut ffi::lua_State,
    must_be_closed: bool,
}

/// RAII guard for a value pushed on the stack.
///
/// You shouldn't have to manipulate this type directly unless you are fiddling with the
/// library's internals.
#[derive(Debug)]
pub struct PushGuard<L>
where
    L: AsLua,
{
    lua: L,
    size: i32,
}

impl<L: AsLua> PushGuard<L> {
    /// Creates a new `PushGuard` from this Lua context representing `size` items on the stack.
    /// When this `PushGuard` is destroyed, `size` items will be popped.
    ///
    /// This is unsafe because the Lua stack can be corrupted if this is misused.
    #[inline]
    pub unsafe fn new(lua: L, size: i32) -> Self {
        PushGuard {
            lua,
            size,
        }
    }

    #[inline]
    fn assert_one_and_forget(self) -> i32 {
        assert_eq!(self.size, 1);
        self.forget_internal()
    }

    /// Returns the number of elements managed by this `PushGuard`.
    #[inline]
    pub fn size(&self) -> i32 {
        self.size
    }

    /// Prevents the value from being popped when the `PushGuard` is destroyed, and returns the
    /// number of elements on the Lua stack.
    ///
    /// This is unsafe because the Lua stack can be corrupted if this is misused.
    #[inline]
    pub unsafe fn forget(self) -> i32 {
        self.forget_internal()
    }

    /// Internal crate-only version of `forget`. It is generally assumed that code within this
    /// crate that calls this method knows what it is doing.
    #[inline]
    fn forget_internal(mut self) -> i32 {
        let size = self.size;
        self.size = 0;
        size
    }

    /// Destroys the guard, popping the value. Returns the inner part,
    /// which returns access when using by-value capture.
    #[inline]
    pub fn into_inner(self) -> L {
        use std::{mem::{self, MaybeUninit}, ptr};

        let mut res = MaybeUninit::uninit();
        unsafe {
            ptr::copy_nonoverlapping(&self.lua, res.as_mut_ptr(), 1);
            if self.size != 0 {
                ffi::lua_pop(self.lua.as_lua(), self.size);
            }
        };
        mem::forget(self);

        unsafe { res.assume_init() }
    }
}

/// Trait for objects that have access to a Lua context. When using a context returned by a
/// `AsLua`, you are not allowed to modify the stack.
// TODO: the lifetime should be an associated lifetime instead
pub trait AsLua {
    fn as_lua(&self) -> *mut ffi::lua_State;

    fn try_push<T>(self, v: T) -> Result<PushGuard<Self>, (<T as Push<Self>>::Err, Self)>
    where
        Self: Sized,
        T: Push<Self>,
    {
        v.push_to_lua(self)
    }

    fn push<T, E>(self, v: T) -> PushGuard<Self>
    where
        Self: Sized,
        T: Push<Self, Err = E>,
        E: Into<Void>,
    {
        v.push_no_err(self)
    }

    fn push_one<T, E>(self, v: T) -> PushGuard<Self>
    where
        Self: Sized,
        T: PushOne<Self, Err = E>,
        E: Into<Void>,
    {
        v.push_no_err(self)
    }

    fn read<T>(self) -> Result<T, Self>
    where
        Self: Sized,
        T: LuaRead<Self>,
    {
        T::lua_read(self)
    }

    fn read_at<T>(self, index: i32) -> Result<T, Self>
    where
        Self: Sized,
        T: LuaRead<Self>,
    {
        T::lua_read_at_maybe_zero_position(self, index)
    }

    fn read_at_nz<T>(self, index: NonZeroI32) -> Result<T, Self>
    where
        Self: Sized,
        T: LuaRead<Self>,
    {
        T::lua_read_at_position(self, index)
    }
}

impl<T> AsLua for &'_ T
where
    T: AsLua,
{
    fn as_lua(&self) -> *mut ffi::lua_State {
        T::as_lua(self)
    }
}

impl AsLua for Lua {
    #[inline]
    fn as_lua(&self) -> *mut ffi::lua_State {
        self.lua
    }
}

impl AsLua for *mut ffi::lua_State {
    fn as_lua(&self) -> *mut ffi::lua_State {
        *self
    }
}

impl<L> AsLua for PushGuard<L>
where
    L: AsLua,
{
    #[inline]
    fn as_lua(&self) -> *mut ffi::lua_State {
        self.lua.as_lua()
    }
}

/// Types that can be given to a Lua context, for example with `lua.set()` or as a return value
/// of a function.
pub trait Push<L: AsLua> {
    /// Error that can happen when pushing a value.
    type Err;

    /// Pushes the value on the top of the stack.
    ///
    /// Must return a guard representing the elements that have been pushed.
    ///
    /// You can implement this for any type you want by redirecting to call to
    /// another implementation (for example `5.push_to_lua`) or by calling
    /// `userdata::push_userdata`.
    fn push_to_lua(self, lua: L) -> Result<PushGuard<L>, (Self::Err, L)>;

    /// Same as `push_to_lua` but can only succeed and is only available if `Err` is `Void`.
    // TODO: when https://github.com/rust-lang/rust/issues/20041 is fixed, use `Self::Err == Void`
    #[inline]
    fn push_no_err<E>(self, lua: L) -> PushGuard<L>
    where
        Self: Sized,
        Self: Push<L, Err = E>,
        E: Into<Void>,
    {
        match self.push_to_lua(lua) {
            Ok(p) => p,
            Err(_) => unreachable!(),
        }
    }
}

// TODO(gmoshkin): PushRef for pushing by reference

/// Extension trait for `Push`. Guarantees that only one element will be pushed.
///
/// This should be implemented on most types that implement `Push`, except for tuples.
///
/// > **Note**: Implementing this trait on a type that pushes multiple elements will most likely
/// > result in panics.
// Note for the implementation: since this trait is not unsafe, it is mostly a hint. Functions can
// require this trait if they only accept one pushed element, but they must also add a runtime
// assertion to make sure that only one element was actually pushed.
pub trait PushOne<L: AsLua>: Push<L> {}

/// Type that cannot be instantiated.
///
/// Will be replaced with `!` eventually (<https://github.com/rust-lang/rust/issues/35121>).
#[derive(Debug, Copy, Clone)]
pub enum Void {}

impl fmt::Display for Void {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        unreachable!("Void cannot be instantiated")
    }
}

pub const NEGATIVE_ONE: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(-1) };
pub const NEGATIVE_TWO: NonZeroI32 = unsafe { NonZeroI32::new_unchecked(-2) };

/// Types that can be obtained from a Lua context.
///
/// Most types that implement `Push` also implement `LuaRead`, but this is not always the case
/// (for example `&'static str` implements `Push` but not `LuaRead`).
pub trait LuaRead<L>: Sized {
    /// Reads the data from Lua.
    #[inline]
    fn lua_read(lua: L) -> Result<Self, L> {
        Self::lua_read_at_position(lua, NEGATIVE_ONE)
    }

    fn lua_read_at_maybe_zero_position(lua: L, index: i32) -> Result<Self, L> {
        if let Some(index) = NonZeroI32::new(index) {
            Self::lua_read_at_position(lua, index)
        } else {
            Err(lua)
        }
    }

    /// Reads the data from Lua at a given position.
    fn lua_read_at_position(lua: L, index: NonZeroI32) -> Result<Self, L>;
}

/// Error that can happen when executing Lua code.
#[derive(Debug)]
pub enum LuaError {
    /// There was a syntax error when parsing the Lua code.
    SyntaxError(String),

    /// There was an error during execution of the Lua code
    /// (for example not enough parameters for a function call).
    ExecutionError(String),

    /// There was an IoError while reading the source code to execute.
    ReadError(IoError),

    /// The call to `execute` has requested the wrong type of data.
    WrongType{
        rust_expected: String,
        lua_actual: String,
    },
}

impl LuaError {
    pub fn wrong_type<T, L: AsLua>(lua: L, n_values: i32) -> Self {
        Self::WrongType {
            rust_expected: std::any::type_name::<T>().into(),
            lua_actual: lua_typename(lua, n_values),
        }
    }
}

pub fn lua_typename(lua: impl AsLua, n_values: i32) -> String {
    let l_ptr = lua.as_lua();
    macro_rules! single_typename {
        ($i:expr) => {
            unsafe {
                let lua_type = ffi::lua_type(l_ptr, -$i);
                let typename = ffi::lua_typename(l_ptr, lua_type);
                std::ffi::CStr::from_ptr(typename).to_string_lossy()
            }
        }
    }

    if n_values == 1 {
        return single_typename!(1).into_owned()
    }

    let mut res = vec![std::borrow::Cow::Borrowed("(")];
    for i in 1..n_values {
        res.push(single_typename!(i));
        res.push(", ".into());
    }
    res.push(single_typename!(n_values));
    res.push(")".into());
    res.join("")
}

impl fmt::Display for LuaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LuaError::*;

        match *self {
            SyntaxError(ref s) => write!(f, "Syntax error: {}", s),
            ExecutionError(ref s) => write!(f, "Execution error: {}", s),
            ReadError(ref e) => write!(f, "Read error: {}", e),
            WrongType{
                rust_expected: ref e1,
                lua_actual: ref e2
            } => write!(f, "Wrong type returned by Lua: {} expected, got {}", e1, e2),
        }
    }
}

impl Error for LuaError {
    fn description(&self) -> &str {
        use LuaError::*;

        match *self {
            SyntaxError(ref s) => &s,
            ExecutionError(ref s) => &s,
            ReadError(_) => "read error",
            WrongType{rust_expected: _, lua_actual: _} => "wrong type returned by Lua",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        use LuaError::*;

        match *self {
            SyntaxError(_) => None,
            ExecutionError(_) => None,
            ReadError(ref e) => Some(e),
            WrongType{rust_expected: _, lua_actual: _} => None,
        }
    }
}

impl From<io::Error> for LuaError {
    fn from(e: io::Error) -> Self {
        LuaError::ReadError(e)
    }
}

impl Lua {
    /// Builds a new empty Lua context.
    ///
    /// There are no global variables and the registry is totally empty. Even the functions from
    /// the standard library can't be used.
    ///
    /// If you want to use the Lua standard library in the scripts of this context, see
    /// [the openlibs method](#method.openlibs)
    ///
    /// # Example
    ///
    /// ```
    /// use hlua::Lua;
    /// let mut lua = Lua::new();
    /// ```
    ///
    /// # Panic
    ///
    /// The function panics if the underlying call to `lua_newstate` fails
    /// (which indicates lack of memory).
    #[inline]
    pub fn new() -> Lua {
        let lua = unsafe { ffi::luaL_newstate() };
        if lua.is_null() {
            panic!("lua_newstate failed");
        }

        // called whenever lua encounters an unexpected error.
        extern "C" fn panic(lua: *mut ffi::lua_State) -> libc::c_int {
            let err = unsafe { ffi::lua_tostring(lua, -1) };
            let err = unsafe { CStr::from_ptr(err) };
            let err = String::from_utf8(err.to_bytes().to_vec()).unwrap();
            panic!("PANIC: unprotected error in call to Lua API ({})\n", err);
        }

        unsafe { ffi::lua_atpanic(lua, panic) };

        Lua {
            lua,
            must_be_closed: true,
        }
    }

    /// Takes an existing `lua_State` and build a Lua object from it.
    ///
    /// If `close_at_the_end` is true, `lua_close` will be called on the `lua_State` in the
    /// destructor.
    #[inline]
    pub unsafe fn from_existing_state<T>(lua: *mut T, close_at_the_end: bool) -> Lua {
        Lua {
            lua: std::mem::transmute(lua),
            must_be_closed: close_at_the_end,
        }
    }

    /// Opens all standard Lua libraries.
    ///
    /// See the reference for the standard library here:
    /// <https://www.lua.org/manual/5.2/manual.html#6>
    ///
    /// This is done by calling `luaL_openlibs`.
    ///
    /// # Example
    ///
    /// ```
    /// use hlua::Lua;
    /// let mut lua = Lua::new();
    /// lua.openlibs();
    /// ```
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn openlibs(&self) {
        unsafe { ffi::luaL_openlibs(self.lua) }
    }

    /// Opens base library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_base>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_base(&self) {
        unsafe { ffi::luaopen_base(self.lua) }
    }

    /// Opens bit32 library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_bit32>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_bit(&self) {
        unsafe { ffi::luaopen_bit(self.lua) }
    }

    /// Opens debug library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_debug>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_debug(&self) {
        unsafe { ffi::luaopen_debug(self.lua) }
    }

    /// Opens io library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_io>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_io(&self) {
        unsafe { ffi::luaopen_io(self.lua) }
    }

    /// Opens math library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_math>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_math(&self) {
        unsafe { ffi::luaopen_math(self.lua) }
    }

    /// Opens os library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_os>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_os(&self) {
        unsafe { ffi::luaopen_os(self.lua) }
    }

    /// Opens package library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_package>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_package(&self) {
        unsafe { ffi::luaopen_package(self.lua) }
    }

    /// Opens string library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_string>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_string(&self) {
        unsafe { ffi::luaopen_string(self.lua) }
    }

    /// Opens table library.
    ///
    /// <https://www.lua.org/manual/5.2/manual.html#pdf-luaopen_table>
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn open_table(&self) {
        unsafe { ffi::luaopen_table(self.lua) }
    }

    /// Executes some Lua code in the context.
    ///
    /// The code will have access to all the global variables you set with methods such as `set`.
    /// Every time you execute some code in the context, the code can modify these global variables.
    ///
    /// The template parameter of this function is the return type of the expression that is being
    /// evaluated.
    /// In order to avoid compilation error, you should call this function either by doing
    /// `lua.execute::<T>(...)` or `let result: T = lua.execute(...);` where `T` is the type of
    /// the expression.
    /// The function will return an error if the actual return type of the expression doesn't
    /// match the template parameter.
    ///
    /// The return type must implement the `LuaRead` trait. See
    /// [the documentation at the crate root](index.html#pushing-and-loading-values) for more
    /// information.
    ///
    /// # Examples
    ///
    /// Without a return value:
    ///
    /// ```
    /// use hlua::Lua;
    /// let mut lua = Lua::new();
    /// lua.execute::<()>("function multiply_by_two(a) return a * 2 end").unwrap();
    /// lua.execute::<()>("twelve = multiply_by_two(6)").unwrap();
    /// ```
    ///
    /// With a return value:
    ///
    /// ```
    /// use hlua::Lua;
    /// let mut lua = Lua::new();
    ///
    /// let twelve: i32 = lua.execute("return 3 * 4;").unwrap();
    /// let sixty = lua.execute::<i32>("return 6 * 10;").unwrap();
    /// ```
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn execute<'lua, T>(&'lua self, code: &str) -> Result<T, LuaError>
    where
        T: LuaRead<PushGuard<LuaFunction<PushGuard<&'lua Self>>>>,
    {
        LuaFunction::load(self, code)?
            .into_call()
    }

    /// Executes some Lua code on the context.
    ///
    /// This does the same thing as [the `execute` method](#method.execute), but the code to
    /// execute is loaded from an object that implements `Read`.
    ///
    /// Use this method when you potentially have a large amount of code (for example if you read
    /// the code from a file) in order to avoid having to put everything in memory first before
    /// passing it to the Lua interpreter.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use hlua::Lua;
    ///
    /// let mut lua = Lua::new();
    /// let script = File::open("script.lua").unwrap();
    /// lua.execute_from_reader::<(), _>(script).unwrap();
    /// ```
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn execute_from_reader<'lua, T>(&'lua self, code: impl Read) -> Result<T, LuaError>
    where
        T: LuaRead<PushGuard<LuaFunction<PushGuard<&'lua Self>>>>,
    {
        LuaFunction::load_from_reader(self, code)?
            .into_call()
    }

    /// Reads the value of a global variable.
    ///
    /// Returns `None` if the variable doesn't exist or has the wrong type.
    ///
    /// The type must implement the `LuaRead` trait. See
    /// [the documentation at the crate root](index.html#pushing-and-loading-values) for more
    /// information.
    ///
    /// # Example
    ///
    /// ```
    /// use hlua::Lua;
    /// let mut lua = Lua::new();
    /// lua.execute::<()>("a = 5").unwrap();
    /// let a: i32 = lua.get("a").unwrap();
    /// assert_eq!(a, 5);
    /// ```
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn get<'lua, V, I>(&'lua self, index: I) -> Option<V>
    where
        I: Borrow<str>,
        V: LuaRead<PushGuard<&'lua Self>>,
    {
        let index = CString::new(index.borrow()).unwrap();
        let guard = unsafe {
            ffi::lua_getglobal(self.lua, index.as_ptr());
            let guard = PushGuard::new(self, 1);
            if ffi::lua_isnil(self.as_lua(), -1) {
                return None;
            }
            guard
        };
        LuaRead::lua_read(guard).ok()
    }

    /// Reads the value of a global, capturing the context by value.
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn into_get<V, I>(self, index: I) -> Result<V, PushGuard<Self>>
    where
        I: Borrow<str>,
        V: LuaRead<PushGuard<Self>>,
    {
        let index = CString::new(index.borrow()).unwrap();
        unsafe {
            ffi::lua_getglobal(self.lua, index.as_ptr());
            let is_nil = ffi::lua_isnil(self.lua, -1);
            let guard = PushGuard::new(self, 1);
            if is_nil {
                Err(guard)
            } else {
                LuaRead::lua_read(guard)
            }
        }
    }

    /// Modifies the value of a global variable.
    ///
    /// If you want to write an array, you are encouraged to use
    /// [the `empty_array` method](#method.empty_array) instead.
    ///
    /// The type must implement the `PushOne` trait. See
    /// [the documentation at the crate root](index.html#pushing-and-loading-values) for more
    /// information.
    ///
    /// # Example
    ///
    /// ```
    /// use hlua::Lua;
    /// let mut lua = Lua::new();
    ///
    /// lua.set("a", 12);
    /// let six: i32 = lua.execute("return a / 2;").unwrap();
    /// assert_eq!(six, 6);
    /// ```
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn set<'lua, I, V, E>(&'lua self, index: I, value: V)
    where
        I: Borrow<str>,
        V: PushOne<&'lua Self, Err = E>,
        E: Into<Void>,
    {
        match self.checked_set(index, value) {
            Ok(_) => (),
            Err(_) => unreachable!(),
        }
    }

    /// Modifies the value of a global variable.
    // TODO: docs
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn checked_set<'lua, I, V>(&'lua self, index: I, value: V)
        -> Result<(), <V as Push<&'lua Self>>::Err>
    where
        I: Borrow<str>,
        V: PushOne<&'lua Self>,
    {
        unsafe {
            ffi::lua_pushglobaltable(self.lua);
            match index.borrow().push_to_lua(self.as_lua()) {
                Ok(pushed) => {
                    debug_assert_eq!(pushed.size, 1);
                    pushed.forget()
                }
                Err(_) => unreachable!(),
            };
            match value.push_to_lua(self) {
                Ok(pushed) => {
                    assert_eq!(pushed.size, 1);
                    pushed.forget()
                }
                Err((err, lua)) => {
                    ffi::lua_pop(lua.as_lua(), 2);
                    return Err(err);
                }
            };
            ffi::lua_settable(self.lua, -3);
            ffi::lua_pop(self.lua, 1);
            Ok(())
        }
    }

    /// Sets the value of a global variable to an empty array, then loads it.
    ///
    /// This is the function you should use if you want to set the value of a global variable to
    /// an array. After calling it, you will obtain a `LuaTable` object which you can then fill
    /// with the elements of the array.
    ///
    /// # Example
    ///
    /// ```
    /// use hlua::Lua;
    /// let mut lua = Lua::new();
    /// lua.openlibs();     // Necessary for `ipairs`.
    ///
    /// {
    ///     let mut array = lua.empty_array("my_values");
    ///     array.set(1, 10);       // Don't forget that Lua arrays are indexed from 1.
    ///     array.set(2, 15);
    ///     array.set(3, 20);
    /// }
    ///
    /// let sum: i32 = lua.execute(r#"
    ///     local sum = 0
    ///     for i, val in ipairs(my_values) do
    ///         sum = sum + val
    ///     end
    ///     return sum
    /// "#).unwrap();
    ///
    /// assert_eq!(sum, 45);
    /// ```
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn empty_array<'lua, I>(&'lua self, index: I) -> LuaTable<PushGuard<&'lua Self>>
    where
        I: Borrow<str>,
    {
        unsafe {
            ffi::lua_pushglobaltable(self.as_lua());
            match index.borrow().push_to_lua(self.as_lua()) {
                Ok(pushed) => pushed.forget(),
                Err(_) => unreachable!(),
            };
            ffi::lua_newtable(self.as_lua());
            ffi::lua_settable(self.as_lua(), -3);
            ffi::lua_pop(self.as_lua(), 1);

            // TODO: cleaner implementation
            self.get(index).unwrap()
        }
    }

    /// Loads the array containing the global variables.
    ///
    /// In lua, the global variables accessible from the lua code are all part of a table which
    /// you can load here.
    ///
    /// # Examples
    ///
    /// The function can be used to write global variables, just like `set`.
    ///
    /// ```
    /// use hlua::Lua;
    /// let mut lua = Lua::new();
    /// lua.globals_table().set("a", 5);
    /// assert_eq!(lua.get::<i32, _>("a"), Some(5));
    /// ```
    ///
    /// A more useful feature for this function is that it allows you to set the metatable of the
    /// global variables. See TODO for more info.
    ///
    /// ```
    /// use hlua::Lua;
    /// use hlua::AnyLuaValue;
    ///
    /// let mut lua = Lua::new();
    /// {
    ///     let mut metatable = lua.globals_table().get_or_create_metatable();
    ///     metatable.set("__index", hlua::function2(|_: AnyLuaValue, var: String| -> AnyLuaValue {
    ///         println!("The user tried to access the variable {:?}", var);
    ///         AnyLuaValue::LuaNumber(48.0)
    ///     }));
    /// }
    ///
    /// let b: i32 = lua.execute("return b * 2;").unwrap();
    /// // -> The user tried to access the variable "b"
    ///
    /// assert_eq!(b, 96);
    /// ```
    #[inline]
    // TODO(gmoshkin): this method should be part of AsLua
    pub fn globals_table<'lua>(&'lua self) -> LuaTable<PushGuard<&'lua Self>> {
        unsafe {
            ffi::lua_pushglobaltable(self.lua);
            let guard = PushGuard::new(self, 1);
            LuaRead::lua_read(guard).ok().unwrap()
        }
    }
}

 impl Drop for Lua {
     #[inline]
     fn drop(&mut self) {
         if self.must_be_closed {
             unsafe { ffi::lua_close(self.lua) }
         }
     }
 }

 impl<L: AsLua> Drop for PushGuard<L> {
     #[inline]
     fn drop(&mut self) {
         if self.size != 0 {
             unsafe {
                 ffi::lua_pop(self.lua.as_lua(), self.size);
             }
         }
     }
 }

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Nil;

impl<L> Push<L> for Nil
where
    L: AsLua,
{
    type Err = Void;      // TODO: use `!` instead (https://github.com/rust-lang/rust/issues/35121)

    fn push_to_lua(self, lua: L) -> Result<PushGuard<L>, (Void, L)> {
        unsafe {
            ffi::lua_pushnil(lua.as_lua());
            Ok(PushGuard::new(lua, 1))
        }
    }
}

impl<L: AsLua> PushOne<L> for Nil {}

impl<L: AsLua> LuaRead<L> for Nil {
    fn lua_read_at_maybe_zero_position(lua: L, index: i32) -> Result<Nil, L> {
        if let Some(index) = NonZeroI32::new(index) {
            Self::lua_read_at_position(lua, index)
        } else {
            Ok(Nil)
        }
    }

    fn lua_read_at_position(lua: L, index: NonZeroI32) -> Result<Nil, L> {
        if unsafe { ffi::lua_isnil(lua.as_lua(), index.into()) } {
            Ok(Nil)
        } else {
            Err(lua)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AbsoluteIndex(NonZeroI32);

impl AbsoluteIndex {
    pub fn new<L>(index: NonZeroI32, lua: L) -> Self
    where
        L: AsLua,
    {
        let top = unsafe { ffi::lua_gettop(lua.as_lua()) };
        let i = index.get();
        if unsafe { ffi::is_relative_index(i) } {
            Self(NonZeroI32::new(top + i + 1).expect("Invalid relative index"))
        } else {
            Self(index)
        }
    }
}

impl From<AbsoluteIndex> for i32 {
    fn from(index: AbsoluteIndex) -> i32 {
        index.0.get()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Lua,
        LuaError,
    };

    #[test]
    fn open_base_opens_base_library() {
        let mut lua = Lua::new();
        match lua.execute::<()>("return assert(true)") {
            Err(LuaError::ExecutionError(_)) => { },
            Err(_) => panic!("Wrong error"),
            Ok(_) => panic!("Unexpected success"),
        }
        lua.open_base();
        let result: bool = lua.execute("return assert(true)").unwrap();
        assert_eq!(result, true);
    }

    #[test]
    fn opening_all_libraries_doesnt_panic() {
        let mut lua = Lua::new();
        lua.open_base();
        lua.open_bit();
        lua.open_debug();
        lua.open_io();
        lua.open_math();
        lua.open_os();
        lua.open_package();
        lua.open_string();
        lua.open_table();
    }
}