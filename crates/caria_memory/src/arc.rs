use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr::NonNull;

/// Automatic Reference Counting wrapper for Caria Runtime Objects
pub struct CariaArc<T> {
    ptr: NonNull<ArcInner<T>>,
}

struct ArcInner<T> {
    rc: AtomicUsize,
    data: T,
}

impl<T> CariaArc<T> {
    pub fn new(data: T) -> Self {
        let inner = Box::new(ArcInner {
            rc: AtomicUsize::new(1),
            data,
        });
        Self {
            ptr: NonNull::new(Box::into_raw(inner)).unwrap(),
        }
    }
}

impl<T> Clone for CariaArc<T> {
    fn clone(&self) -> Self {
        unsafe {
            self.ptr.as_ref().rc.fetch_add(1, Ordering::Relaxed);
        }
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for CariaArc<T> {
    fn drop(&mut self) {
        unsafe {
            if self.ptr.as_ref().rc.fetch_sub(1, Ordering::Release) == 1 {
                // Drop the inner data
                let _ = Box::from_raw(self.ptr.as_ptr());
            }
        }
    }
}
