// SPDX-License-Identifier: GPL-2.0

//! Miscellaneous devices.
//!
#[allow(unused_imports)]
use core::{ffi::c_void, marker::PhantomData, mem::MaybeUninit, pin::Pin};

use crate::{c_str, pr_info};
use alloc::{boxed::Box, vec::Vec};
use kernel::bindings::{
    file, inode, misc_deregister, misc_register, miscdevice, MISC_DYNAMIC_MINOR,
};

/// Registration for miscellaneous device
///
/// ```rust,ignore
/// # use kernel::error::Result;
/// # use kernel::bindings::{MiscDev, Registration};
/// struct MyMiscDevice;
///
/// impl MiscDev for MyMiscDevice {
///     ...
/// }
///
/// fn register_device() -> Result<Registration<MyMiscDevice>> {
///   Registration::register()?
/// }
///
/// ```
#[allow(dead_code)]
pub struct Registration<T: MiscDev> {
    /// Is module registered
    registered: bool,
    /// Holds device information
    pub miscdev: miscdevice,
    /// Holds the miscellaneous device callback implementation
    marker: PhantomData<T>,
}

impl<T: MiscDev> Default for Registration<T> {
    fn default() -> Self {
        Registration {
            registered: false,
            miscdev: miscdevice::default(),
            marker: PhantomData,
        }
    }
}

impl<T: MiscDev> Registration<T> {
    #[allow(dead_code)]
    const VTABLE: kernel::bindings::file_operations = kernel::bindings::file_operations {
        open: Some(Self::open_callback),
        release: None,
        read: Some(Self::read_callback),
        write: None,
        llseek: Some(kernel::bindings::noop_llseek),
        check_flags: None,
        compat_ioctl: None,
        copy_file_range: None,
        fallocate: None,
        fadvise: None,
        fasync: None,
        flock: None,
        flush: None,
        fsync: None,
        get_unmapped_area: None,
        iterate_shared: None,
        iopoll: None,
        lock: None,
        mmap: None,
        mmap_supported_flags: 0,
        owner: core::ptr::null_mut(),
        poll: None,
        read_iter: None,
        remap_file_range: None,
        setlease: None,
        show_fdinfo: None,
        splice_read: None,
        splice_eof: None,
        splice_write: None,
        unlocked_ioctl: None,
        uring_cmd: None,
        uring_cmd_iopoll: None,
        write_iter: None,
    };

    #[allow(dead_code)]
    /// Register a miscellaneous device implementation
    pub fn new_pinned() -> kernel::error::Result<Pin<Box<Registration<T>>>> {
        pr_info!("box = {:p}\n", Box::try_new(1)?);
        Ok(Pin::from(Box::try_new(Registration::default())?))
    }

    #[allow(dead_code)]
    /// Register a miscellaneous device implementation
    pub fn register(self: Pin<&mut Self>) -> kernel::error::Result<()> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.registered {
            // Already registered.
            return Err(kernel::prelude::EINVAL);
        }

        this.miscdev.minor = MISC_DYNAMIC_MINOR as i32;
        this.miscdev.name = c_str!("rchrdev").as_char_ptr();
        this.miscdev.fops = &Self::VTABLE;
        let res = unsafe { misc_register(&mut this.miscdev) };
        kernel::error::to_result(res)?;
        this.registered = true;

        pr_info!("Registered a new misc device `rchrdev`\n");
        Ok(())
    }

    unsafe extern "C" fn open_callback(_inode: *mut inode, filp: *mut file) -> core::ffi::c_int {
        pr_info!("Called open_callback\n");
        let ptr = crate::container_of!(unsafe { (*filp).private_data }, Self, miscdev);
        unsafe { (*filp).private_data = ptr as *mut core::ffi::c_void };
        0
    }

    unsafe extern "C" fn read_callback(
        filp: *mut kernel::bindings::file,
        buffer: *mut core::ffi::c_char,
        count: usize,
        ppos: *mut kernel::bindings::loff_t,
    ) -> isize {
        pr_info!("Called read_callback\n");
        let device_buf = match T::read(count, unsafe { *ppos } as isize) {
            Ok(rlen) => rlen,
            Err(err) => return -(err.to_errno() as isize),
        };
        let device_buf_len = device_buf.len() as u64;
        let res = unsafe {
            kernel::bindings::_copy_to_user(
                buffer as *mut c_void,
                device_buf.as_ptr() as *const c_void,
                device_buf_len,
            )
        };
        if res != 0 {
            -(kernel::bindings::EFAULT as isize)
        } else {
            pr_info!("Read_response has {device_buf_len} bytes\n");
            unsafe { *filp }.f_pos += device_buf_len as i64;
            device_buf_len as isize
        }
    }
}

impl<T: MiscDev> Drop for Registration<T> {
    fn drop(&mut self) {
        if self.registered {
            unsafe { misc_deregister(&mut self.miscdev) };
        }
    }
}

/// Trait for callback of miscellaneous device
///
/// ```ignore
/// # use kernel::bindings::MiscDev;
/// struct MyMiscDevice;
///
/// impl MiscDev for MyMiscDevice {
///     fn read(count: usize, _pos: isize) -> kernel::error::Result<Vec<u8>> {
///         "Hello world"[..count].as_bytes().to_vec()
///     }
/// }
///
/// ```
pub trait MiscDev {
    /// Returns the content of a read request
    fn read(_count: usize, _pos: isize) -> crate::error::Result<Vec<u8>>;
}
