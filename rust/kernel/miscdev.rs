// SPDX-License-Identifier: GPL-2.0

//! Miscellaneous devices.
//!
#[allow(unused_imports)]
use core::{ffi::c_void, marker::PhantomData, mem::MaybeUninit, pin::Pin};

use crate::{c_str, pr_info, prelude::vtable, types::ForeignOwnable};
use alloc::{boxed::Box, vec::Vec};
use kernel::{
    bindings::{file, inode, misc_deregister, misc_register, miscdevice, MISC_DYNAMIC_MINOR},
    error::Result,
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
    miscdev: miscdevice,
    /// Open
    open_data: MaybeUninit<T::OpenData>,
    /// Holds the miscellaneous device callback implementation
    marker: PhantomData<T>,
}

impl<T: MiscDev> Default for Registration<T> {
    fn default() -> Self {
        Registration {
            registered: false,
            miscdev: miscdevice::default(),
            open_data: MaybeUninit::uninit(),
            marker: PhantomData,
        }
    }
}

impl<T: MiscDev<OpenData = ()>> Registration<T> {
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
    pub fn register(self: &mut Self) -> kernel::error::Result<Self> {
        let mut reg = Registration::default();
        reg.miscdev.minor = MISC_DYNAMIC_MINOR as i32;
        reg.miscdev.name = c_str!("chrdev").as_char_ptr();
        reg.miscdev.fops = &Self::VTABLE;
        pr_info!("*open_callback = {:p}", unsafe {
            (*reg.miscdev.fops).open.unwrap()
        });

        let res = unsafe { misc_register(&mut reg.miscdev) };
        kernel::error::to_result(res)?;
        pr_info!("Registered a new misc device\n");

        reg.registered = true;
        Ok(reg)
    }

    unsafe extern "C" fn open_callback(_inode: *mut inode, filp: *mut file) -> core::ffi::c_int {
        pr_info!("Called open_callback\n");
        let ptr = crate::container_of!(unsafe { (*filp).private_data }, Self, miscdev);
        unsafe { (*filp).private_data = ptr as *mut core::ffi::c_void };
        0
    }

    unsafe extern "C" fn read_callback(
        _filp: *mut kernel::bindings::file,
        _buffer: *mut core::ffi::c_char,
        _count: usize,
        _ppos: *mut kernel::bindings::loff_t,
    ) -> isize {
        pr_info!("Called read_callback\n");
        0
        /*let device_buf = T::read(count);
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
            device_buf_len as isize
        }*/
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
///     fn read(count: usize) -> Vec<u8> {
///         "Hello world"[..count].as_bytes().to_vec()
///     }
/// }
///
/// ```
#[vtable]
pub trait MiscDev {
    /// 1
    type Data: ForeignOwnable + Send + Sync = ();

    /// 1
    type OpenData: Sync = ();

    /// Open
    fn open(_context: &Self::OpenData, _filp: &kernel::bindings::file) -> Result<Self::Data> {
        pr_info!("Open called\n");
        // let inode_ptr = crate::container_of!(unsafe { (*filp).private_data }, Self, miscdev);
        // unsafe { (*filp).private_data = inode_ptr as *mut core::ffi::c_void };
        core::prelude::v1::Err(kernel::error::code::EINVAL)
    }

    /// Returns the content of a read request
    fn read(_count: usize) -> Vec<u8> {
        pr_info!("Read called\n");
        Vec::new()
    }
}
