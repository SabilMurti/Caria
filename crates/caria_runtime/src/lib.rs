#[macro_use]
extern crate lazy_static;

pub mod runtime;
pub mod ffi_aot;
pub mod value;

pub use runtime::CariaRuntime;
pub use value::CariaValue;
