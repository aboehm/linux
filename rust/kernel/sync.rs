// SPDX-License-Identifier: GPL-2.0

//! Synchronisation primitives.
//!
//! This module contains the kernel APIs related to synchronisation that have been ported or
//! wrapped for usage by Rust code in the kernel.

use crate::types::Opaque;

mod arc;
mod condvar;
pub mod guard;
pub mod lock;
mod locked_by;
pub mod revocable;

pub use crate::str::CStr;
pub use arc::{Arc, ArcBorrow, UniqueArc};
pub use condvar::CondVar;
pub use core::pin::Pin;
pub use lock::{mutex::Mutex, spinlock::SpinLock};
pub use locked_by::LockedBy;

/// Represents a lockdep class. It's a wrapper around C's `lock_class_key`.
#[repr(transparent)]
pub struct LockClassKey(Opaque<bindings::lock_class_key>);

// SAFETY: `bindings::lock_class_key` is designed to be used concurrently from multiple threads and
// provides its own synchronization.
unsafe impl Sync for LockClassKey {}

impl LockClassKey {
    /// Creates a new lock class key.
    pub const fn new() -> Self {
        Self(Opaque::uninit())
    }

    pub(crate) fn as_ptr(&self) -> *mut bindings::lock_class_key {
        self.0.get()
    }
}

/// A trait for types that need a lock class during initialisation.
///
/// Implementers of this trait benefit from the [`init_with_lockdep`] macro that generates a new
/// class for each initialisation call site.
pub trait NeedsLockClass {
    /// Initialises the type instance so that it can be safely used.
    ///
    /// Callers are encouraged to use the [`init_with_lockdep`] macro as it automatically creates a
    /// new lock class on each usage.
    fn init(
        self: Pin<&mut Self>,
        name: &'static CStr,
        key1: &'static LockClassKey,
        key2: &'static LockClassKey,
    );
}

/// Defines a new static lock class and returns a pointer to it.
#[doc(hidden)]
#[macro_export]
macro_rules! static_lock_class {
    () => {{
        static CLASS: $crate::sync::LockClassKey = $crate::sync::LockClassKey::new();
        &CLASS
    }};
}

/// Returns the given string, if one is provided, otherwise generates one based on the source code
/// location.
#[doc(hidden)]
#[macro_export]
macro_rules! optional_name {
    () => {
        $crate::c_str!(::core::concat!(::core::file!(), ":", ::core::line!()))
    };
    ($name:literal) => {
        $crate::c_str!($name)
    };
}
