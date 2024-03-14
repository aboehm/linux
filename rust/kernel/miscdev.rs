// SPDX-License-Identifier: GPL-2.0

//! Miscellaneous devices.
//!
use core::{
    ffi::c_void,
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    ops::Deref,
    ops::DerefMut,
    pin::Pin,
};

use crate::{c_str, pr_info};
use alloc::{boxed::Box, vec::Vec};
use kernel::{
    bindings::{file, inode, misc_deregister, misc_register, miscdevice, MISC_DYNAMIC_MINOR},
    prelude::*,
    types::ForeignOwnable,
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
    /// Hold device state
    open_data: MaybeUninit<T::OpenData>,
    /// Unpin isn't allowed for `Registration`
    _pin: PhantomPinned,
}

impl<T: MiscDev> Default for Registration<T> {
    fn default() -> Self {
        Registration {
            registered: false,
            miscdev: miscdevice::default(),
            open_data: MaybeUninit::uninit(),
            _pin: PhantomPinned,
        }
    }
}

impl<T> Registration<T>
where
    T: MiscDev,
    T::Data: 'static,
{
    #[allow(dead_code)]
    const VTABLE: kernel::bindings::file_operations = kernel::bindings::file_operations {
        open: Some(Self::open_callback),
        release: Some(Self::release_callback),
        read: Some(Self::read_callback),
        write: Some(Self::write_callback),
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

    pub fn new_pinned(data: T::OpenData) -> Result<Pin<Box<Self>>> {
        let registration = Registration::default();
        let registration = Box::try_new(registration)?;
        pr_info!("Registration place at {:p}", &registration);
        let mut registration = Pin::from(registration);
        Self::register(registration.as_mut(), data)?;
        Ok(registration)
    }

    /// Register the device
    fn register(self: Pin<&mut Self>, data: T::OpenData) -> Result<()> {
        let registration = unsafe { self.get_unchecked_mut() };
        if registration.registered {
            // Already registered.
            return Err(kernel::prelude::EINVAL);
        }
        registration.registered = true;

        registration.miscdev.minor = MISC_DYNAMIC_MINOR as i32;
        registration.miscdev.name = c_str!("rchrdev").as_char_ptr();
        registration.miscdev.fops = &Self::VTABLE;
        let res = unsafe { misc_register(&mut registration.miscdev) };
        kernel::error::to_result(res)?;
        registration.open_data.write(data);
        pr_info!(
            "Registration data placed at {:p}\n",
            registration.open_data.as_ptr()
        );

        pr_info!("Registered a new misc device `rchrdev`\n");
        Ok(())
    }

    unsafe extern "C" fn open_callback(inode: *mut inode, filp: *mut file) -> core::ffi::c_int {
        pr_info!("Called open_callback\n");
        pr_info!("file pointer private data at {:p}\n", unsafe {
            (*filp).private_data
        });

        let reg = crate::container_of!(unsafe { (*filp).private_data }, Self, miscdev);
        pr_info!("Registration data placed at {:p}\n", unsafe {
            (*reg).open_data.as_ptr()
        });
        let open_data = unsafe { (*reg).open_data.as_ptr().as_ref().unwrap() };

        if let Ok(data) = T::open(open_data) {
            let ptr = ForeignOwnable::into_foreign(data);
            unsafe { (*filp).private_data = ptr as *mut core::ffi::c_void };
            pr_info!("Data from open will be placed at {:p}", unsafe {
                (*filp).private_data
            });
            0
        } else {
            -(kernel::bindings::EFAULT as i32)
        }
    }

    unsafe extern "C" fn read_callback(
        filp: *mut kernel::bindings::file,
        buffer: *mut core::ffi::c_char,
        count: usize,
        ppos: *mut kernel::bindings::loff_t,
    ) -> isize {
        pr_info!("Called read_callback\n");
        pr_info!("file pointer private data at {:p}\n", unsafe {
            (*filp).private_data
        });
        let mut data = unsafe { <T as MiscDev>::Data::borrow((*filp).private_data) };
        pr_info!("Data for misc device placed at {:p}", &data);
        let device_buf = match T::read(data, count, unsafe { *ppos } as isize) {
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
            pr_err!("Problem while copying data to user space: {res}");
            -(kernel::bindings::EINVAL as isize)
        } else {
            pr_info!("Read_response has {device_buf_len} bytes\n");
            unsafe { *filp }.f_pos += device_buf_len as i64;
            device_buf_len as isize
        }
    }

    unsafe extern "C" fn write_callback(
        filp: *mut file,
        buf: *const core::ffi::c_char,
        count: usize,
        ppos: *mut kernel::bindings::loff_t,
    ) -> isize {
        pr_info!("Called write_callback\n");
        let mut data = unsafe { <T as MiscDev>::Data::borrow((*filp).private_data) };
        pr_info!("Data for misc device placed at {:p}\n", &data);

        let mut buffer = if let Ok(buffer) = Vec::try_with_capacity(count) {
            buffer
        } else {
            pr_err!("Can't allocate {count} bytes\n");
            return -(kernel::bindings::EFAULT as isize);
        };
        if buffer.try_resize(count, 0u8).is_err() {
            pr_err!("Can't resize vector to {count} elements\n");
            return -(kernel::bindings::EFAULT as isize);
        }
        let res = unsafe {
            kernel::bindings::_copy_from_user(
                buffer.as_slice().as_ptr() as *mut c_void,
                buf as *const c_void,
                count.try_into().unwrap(),
            )
        };
        if res != 0 {
            pr_err!("Problem while copying data from user space: {res}");
            return -(kernel::bindings::EINVAL as isize);
        }
        pr_info!("Data buffer [{}]: {buffer:x?}\n", buffer.len());

        match T::write(data, &buffer, unsafe { *ppos } as isize) {
            Ok(wlen) => {
                pr_info!("Write_response has {wlen} bytes\n");
                unsafe { *filp }.f_pos += wlen as i64;
                wlen
            }
            Err(err) => -(err.to_errno() as isize),
        }
    }

    unsafe extern "C" fn release_callback(inode: *mut inode, filp: *mut file) -> core::ffi::c_int {
        pr_info!("Called release_callback\n");
        pr_info!("file pointer private data at {:p}\n", unsafe {
            (*filp).private_data
        });
        let mut data = unsafe { <T as MiscDev>::Data::from_foreign((*filp).private_data) };
        pr_info!("Data for misc device placed at {:p}", &data);
        T::release(data).map(|_| 0).unwrap_or_else(Error::to_errno)
    }
}

impl<T: MiscDev> Drop for Registration<T> {
    fn drop(&mut self) {
        if self.registered {
            unsafe { misc_deregister(&mut self.miscdev) };
            unsafe { self.open_data.assume_init_drop() };
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
    type Data: ForeignOwnable + Send + Sync;
    type OpenData: Sync;

    /// File opened
    fn open(data: &Self::OpenData) -> crate::error::Result<Self::Data> {
        Err(EINVAL)
    }

    /// Returns the content of a read request
    fn read(
        _context: <Self::Data as ForeignOwnable>::Borrowed<'_>,
        _count: usize,
        _pos: isize,
    ) -> crate::error::Result<Vec<u8>> {
        Err(EINVAL)
    }

    /// Returns the content of a read request
    fn write(
        _context: <Self::Data as ForeignOwnable>::Borrowed<'_>,
        _data: &[u8],
        _pos: isize,
    ) -> crate::error::Result<isize> {
        Err(EINVAL)
    }

    /// File closed
    fn release(_context: Self::Data) -> Result<()> {
        Err(EINVAL)
    }
}

/// Calculates the offset of a field from the beginning of the struct it belongs to.
/// (copied from
/// https://github.com/Rust-for-Linux/linux/blob/18b7491480025420896e0c8b73c98475c3806c6f/rust/kernel/lib.rs#L191)
///
/// # Examples
///
/// ```
/// # use kernel::prelude::*;
/// # use kernel::offset_of;
/// struct Test {
///     a: u64,
///     b: u32,
/// }
///
/// assert_eq!(offset_of!(Test, b), 8);
/// ```
#[macro_export]
macro_rules! offset_of {
    ($type:ty, $($f:tt)*) => {{
        let tmp = core::mem::MaybeUninit::<$type>::uninit();
        let outer = tmp.as_ptr();
        // To avoid warnings when nesting `unsafe` blocks.
        #[allow(unused_unsafe)]
        // SAFETY: The pointer is valid and aligned, just not initialised; `addr_of` ensures that
        // we don't actually read from `outer` (which would be UB) nor create an intermediate
        // reference.
        let inner = unsafe { core::ptr::addr_of!((*outer).$($f)*) } as *const u8;
        // To avoid warnings when nesting `unsafe` blocks.
        #[allow(unused_unsafe)]
        // SAFETY: The two pointers are within the same allocation block.
        unsafe { inner.offset_from(outer as *const u8) }
    }}
}

/// Produces a pointer to an object from a pointer to one of its fields.
/// (copied from
/// https://github.com/Rust-for-Linux/linux/blob/18b7491480025420896e0c8b73c98475c3806c6f/rust/kernel/lib.rs#L223)
///
/// # Safety
///
/// Callers must ensure that the pointer to the field is in fact a pointer to the specified field,
/// as opposed to a pointer to another object of the same type. If this condition is not met,
/// any dereference of the resulting pointer is UB.
///
/// # Examples
///
/// ```
/// # use kernel::container_of;
/// struct Test {
///     a: u64,
///     b: u32,
/// }
///
/// let test = Test { a: 10, b: 20 };
/// let b_ptr = &test.b;
/// let test_alias = container_of!(b_ptr, Test, b);
/// assert!(core::ptr::eq(&test, test_alias));
/// ```
#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $type:ty, $($f:tt)*) => {{
        let ptr = $ptr as *const _ as *const u8;
        let offset = $crate::offset_of!($type, $($f)*);
        ptr.wrapping_offset(-offset) as *const $type
    }}
}
