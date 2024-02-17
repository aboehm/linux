// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.

use kernel::c_str;
use kernel::prelude::*;

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Rust for Linux Contributors",
    description: "Rust character device sample",
    license: "GPL",
}

#[allow(dead_code)]
struct RustChrdev {
    dev: kernel::bindings::dev_t,
    cdev: *mut kernel::bindings::cdev,
}

unsafe impl Sync for RustChrdev {}

unsafe extern "C" fn open_device(
    _arg1: *mut kernel::bindings::inode,
    _arg2: *mut kernel::bindings::file,
) -> core::ffi::c_int {
    0
}

unsafe extern "C" fn release_device(
    _arg1: *mut kernel::bindings::inode,
    _arg2: *mut kernel::bindings::file,
) -> core::ffi::c_int {
    0
}

unsafe extern "C" fn unlocked_ioctl_device(
    _: *mut kernel::bindings::file,
    _: core::ffi::c_uint,
    _: core::ffi::c_ulong,
) -> core::ffi::c_long {
    0
}

unsafe extern "C" fn write_device(
    _arg1: *mut kernel::bindings::file,
    _arg2: *const core::ffi::c_char,
    _arg3: usize,
    _arg4: *mut kernel::bindings::loff_t,
) -> isize {
    0
}

unsafe extern "C" fn read_device(
    _arg1: *mut kernel::bindings::file,
    _arg2: *mut core::ffi::c_char,
    _arg3: usize,
    _arg4: *mut kernel::bindings::loff_t,
) -> isize {
    0
}

const VTABLE: kernel::bindings::file_operations = kernel::bindings::file_operations {
    open: Some(open_device),
    release: Some(release_device),
    read: Some(read_device),
    write: Some(write_device),
    llseek: None,
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
    unlocked_ioctl: Some(unlocked_ioctl_device),
    uring_cmd: None,
    uring_cmd_iopoll: None,
    write_iter: None,
};

impl kernel::Module for RustChrdev {
    fn init(module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust character device sample (init)\n");
        let mut dev: kernel::bindings::dev_t = 0;

        pr_info!("Allocate chrdev region");
        let res = unsafe {
            kernel::bindings::alloc_chrdev_region(
                &mut dev,
                0,
                1,
                c_str!("sampledev0").as_char_ptr(),
            )
        };
        if res != 0 {
            pr_err!("Can't allocate chrdev region: {res}");
            return Err(crate::ENOTSUPP);
        }

        pr_info!("Allocate cdev");
        let cdev = unsafe { kernel::bindings::cdev_alloc() };
        if cdev.is_null() {
            pr_err!("Can't allocate char device");
            return Err(ENOMEM);
        }

        pr_info!("cdev allocated");
        unsafe {
            (*cdev).ops = &VTABLE;
            (*cdev).owner = module.0;
        }

        let device_number = dev + 1 as kernel::bindings::dev_t;
        pr_info!("Adding char device as number {}", device_number);
        unsafe { kernel::bindings::cdev_add(cdev, device_number, 1) };
        pr_info!("char device added");

        Ok(RustChrdev { dev, cdev })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust character device sample (exit)\n");
    }
}
