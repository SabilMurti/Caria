use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr::NonNull;
use std::ops::Deref;
use std::fmt;

/// Automatic Reference Counting wrapper for Caria Runtime Objects
pub struct CariaArc<T> {
    ptr: NonNull<ArcInner<T>>,
}

unsafe impl<T: Send + Sync> Send for CariaArc<T> {}
unsafe impl<T: Send + Sync> Sync for CariaArc<T> {}

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

    pub fn strong_count(&self) -> usize {
        unsafe { self.ptr.as_ref().rc.load(Ordering::SeqCst) }
    }
}

impl<T> Deref for CariaArc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T> AsRef<T> for CariaArc<T> {
    fn as_ref(&self) -> &T {
        unsafe { &self.ptr.as_ref().data }
    }
}

impl<T: PartialEq> PartialEq for CariaArc<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: fmt::Debug> fmt::Debug for CariaArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CariaArc")
            .field(&**self)
            .finish()
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
                // Drop the inner data safely
                let _ = Box::from_raw(self.ptr.as_ptr());
            }
        }
    }
}
