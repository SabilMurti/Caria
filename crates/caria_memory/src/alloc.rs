pub struct MemoryManager {
    total_allocated: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self { total_allocated: 0 }
    }

    pub fn allocate(&mut self, size: usize) -> *mut u8 {
        self.total_allocated += size;
        // Stub for bump allocator
        std::ptr::null_mut()
    }
}
