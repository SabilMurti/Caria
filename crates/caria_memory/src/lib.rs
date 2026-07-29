pub mod arc;
pub mod alloc;

pub use arc::CariaArc;
pub use alloc::MemoryManager;

pub fn manage_ir() {
    // Hooks to inject retain/release into IR
}
