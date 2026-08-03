#[macro_use]
extern crate lazy_static;

pub mod runtime;
pub mod ffi_aot;
pub mod value;
pub mod embed;

pub use runtime::CariaRuntime;
pub use value::CariaValue;
pub use embed::{CariaEngine, CariaError};
