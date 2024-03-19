// SPDX-License-Identifier: GPL-2.0

//! Miscellaneous devices.
//!
use core::{ffi::c_void, marker::PhantomPinned, mem::MaybeUninit, pin::Pin};

use crate::{c_str, pr_info};
use alloc::{boxed::Box, vec::Vec};
use kernel::{
    bindings::{file, inode, misc_deregister, misc_register, miscdevice, MISC_DYNAMIC_MINOR},
    prelude::*,
    types::ForeignOwnable,
};

/// Registration for miscellaneous device
///
/// ```rust,no_run
/// # use kernel::prelude::*;
/// # use kernel::bindings::{MiscDev, Registration};
///
/// struct MyMiscDevice {
///     _registration: Pin<Box<Registration<()>>>,
/// }
///
/// impl MiscDev for MyMiscDevice {
///     ...
/// }
///
/// impl kernel::Module for RustCltModule {
///     fn init(_module: &'static ThisModule) -> Result<Self> {
///         let registration = miscdev::Registration::new_pinned_registered(())?;
///         Ok(MyMiscDevice {
///             _registration: registration,
///         })
///     }
/// }
/// ```
pub struct Registration<T: MiscDev> {
    /// Is module registered
    registered: bool,
    /// Holds device information
    miscdev: miscdevice,
    /// Hold device state
    open_data: MaybeUninit<T::OpenData>,
    /// Unpin isn't allowed for `Registration`
    _pin: PhantomPinned,
}

/// SAFETY: Caused by `miscdev` but only used by the kernel after initialization.
unsafe impl<T: MiscDev> Sync for Registration<T> {}

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
    const FOPS: kernel::bindings::file_operations = kernel::bindings::file_operations {
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

    /// Register the device on the kernel. When the device file is open, supply `T::open` with `data`.
    pub fn new_pinned_registered(data: T::OpenData) -> Result<Pin<Box<Self>>> {
        let registration = Registration::default();
        let registration = Box::try_new(registration)?;
        pr_info!("Registration place at {:p}", &registration);
        let mut registration = Pin::from(registration);
        Self::register(registration.as_mut(), data)?;
        Ok(registration)
    }

    /// Register the device on the kernel with an already pinned data
    fn register(self: Pin<&mut Self>, data: T::OpenData) -> Result<()> {
        let registration = unsafe { self.get_unchecked_mut() };
        if registration.registered {
            // Already registered.
            return Err(EINVAL);
        }

        // Prepare kernel structure for misc device, ref [`chrdev.c`](chrdev.c)
        registration.miscdev.minor = MISC_DYNAMIC_MINOR as i32;
        registration.miscdev.name = c_str!("rchrdev").as_char_ptr();
        registration.miscdev.fops = &Self::FOPS;
        registration.registered = true;
        registration.open_data.write(data);

        let res = unsafe { misc_register(&mut registration.miscdev) };
        if res < 0 {
            // Device registration failed, revert all changes
            registration.registered = false;
            unsafe { registration.open_data.assume_init_drop() };
            kernel::error::to_result(res)?;
        }

        pr_info!(
            "Registration data placed at {:p}\n",
            registration.open_data.as_ptr()
        );

        pr_info!("Registered a new misc device `rchrdev`\n");
        Ok(())
    }

    /// Unsafe wrapper to unpack kernel structures into safe rust world
    unsafe extern "C" fn open_callback(_inode: *mut inode, filp: *mut file) -> core::ffi::c_int {
        pr_info!("Called open_callback\n");
        pr_info!("file pointer private data at {:p}\n", unsafe {
            (*filp).private_data
        });

        let reg: *const Self = crate::container_of!(unsafe { (*filp).private_data }, Self, miscdev);
        pr_info!("Registration data placed at {:p}\n", unsafe {
            (*reg).open_data.as_ptr()
        });
        let open_data: &T::OpenData = unsafe { (*reg).open_data.as_ptr().as_ref().unwrap() };

        if let Ok(data) = T::open(open_data) {
            // Transfer ownership to kernel
            let ptr = ForeignOwnable::into_foreign(data);
            unsafe { (*filp).private_data = ptr as *mut core::ffi::c_void };
            pr_info!("Data from open will be placed at {:p}", unsafe {
                (*filp).private_data
            });
            0
        } else {
            -(EFAULT.to_errno() as core::ffi::c_int)
        }
    }

    /// Unsafe wrapper to unpack kernel structures into safe rust world
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
        // Borrow data from kernel of type `Data`
        let data = unsafe { <T as MiscDev>::Data::borrow((*filp).private_data) };
        pr_info!("Data for misc device placed at {:p}", &data);
        let device_buf = match T::read(data, count, unsafe { *ppos } as isize) {
            Ok(rlen) => rlen,
            Err(err) => return -(err.to_errno() as isize),
        };
        let device_buf_len = device_buf.len() as u64;
        // Copy kernel data from kernel to user space
        let res = unsafe {
            kernel::bindings::_copy_to_user(
                buffer as *mut c_void,
                device_buf.as_ptr() as *const c_void,
                device_buf_len,
            )
        };
        if res == 0 {
            pr_info!("Read_response has {device_buf_len} bytes\n");
            unsafe { *filp }.f_pos += device_buf_len as i64;
            device_buf_len as isize
        } else {
            pr_err!("Problem while copying data to user space: {res}");
            -(EINVAL.to_errno() as isize)
        }
    }

    /// Unsafe wrapper to unpack kernel structures into safe rust world
    unsafe extern "C" fn write_callback(
        filp: *mut file,
        buf: *const core::ffi::c_char,
        count: usize,
        ppos: *mut kernel::bindings::loff_t,
    ) -> isize {
        pr_info!("Called write_callback\n");
        // Borrow data from kernel of type `Data`
        let data = unsafe { <T as MiscDev>::Data::borrow((*filp).private_data) };
        pr_info!("Data for misc device placed at {:p}\n", &data);

        let mut buffer = if let Ok(buffer) = Vec::try_with_capacity(count) {
            buffer
        } else {
            pr_err!("Can't allocate {count} bytes\n");
            return -(EFAULT.to_errno() as isize);
        };
        if buffer.try_resize(count, 0u8).is_err() {
            pr_err!("Can't resize vector to {count} elements\n");
            return -(EFAULT.to_errno() as isize);
        }
        // Copy user data from user to kernel space
        let res = unsafe {
            kernel::bindings::_copy_from_user(
                buffer.as_slice().as_ptr() as *mut c_void,
                buf as *const c_void,
                count.try_into().unwrap(),
            )
        };
        if res != 0 {
            pr_err!("Problem while copying data from user space: {res}");
            return -(EINVAL.to_errno() as isize);
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

    /// Unsafe wrapper to unpack kernel structures into safe rust world
    unsafe extern "C" fn release_callback(_inode: *mut inode, filp: *mut file) -> core::ffi::c_int {
        pr_info!("Called release_callback\n");
        pr_info!("file pointer private data at {:p}\n", unsafe {
            (*filp).private_data
        });
        let data = unsafe { <T as MiscDev>::Data::from_foreign((*filp).private_data) };
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

/// Trait for callback of miscellaneous device.
///
/// ```rust,no_run
/// use core::sync::atomic::{AtomicUsize, Ordering};
/// # use kernel::bindings::MiscDev;
///
/// /// String that should be returned by the device
/// const READ_DATA: &str = "Hello CLT\n";
///
/// /// A simple reader
/// struct SimpleReader {}
///
/// impl miscdev::MiscDev for SimpleReader {
///     type Data = Arc<AtomicUsize>;
///     type OpenData = ();
///
///     fn open(_: &Self::OpenData) -> Result<Self::Data> {
///         // Set head at the begin of the string
///         Ok(Arc::try_new(AtomicUsize::new(0))?)
///     }
///
///     fn read(context: ArcBorrow<'_, AtomicUsize>, count: usize, _ppos: isize) -> Result<Vec<u8>> {
///         // Get head position
///         let head = context.load(Ordering::Relaxed);
///         // Determine the head position after read
///         let to = core::cmp::min(head + count, READ_DATA.len());
///
///         // Fill the read buffer
///         let mut buf = Vec::new();
///         for i in READ_DATA[head..to].as_bytes() {
///             buf.try_push(*i)?;
///         }
///
///         // Update the head position
///         context.store(to, Ordering::Relaxed);
///         Ok(buf)
///     }
/// }
/// ```
pub trait MiscDev {
    /// Representation of a context of a opened file
    type Data: ForeignOwnable + Send + Sync;
    /// Data which is presented when a device file should be opened
    type OpenData: Sync;

    /// A device file shall be opened. All relevant data for the open device file can be generated. The ownership of the returned `Data` will be transfered to a Kernel managed data structure and borrowed for `read` and `write` operations.
    fn open(_data: &Self::OpenData) -> Result<Self::Data> {
        Err(EINVAL)
    }

    /// A read operation was called for the device file. `Data` will be borrowed from the Kernel owned data structure. `count` represents the requested bytes. `_pos` is the current position in the file. A buffer is returned.
    fn read(
        _context: <Self::Data as ForeignOwnable>::Borrowed<'_>,
        _count: usize,
        _pos: isize,
    ) -> Result<Vec<u8>> {
        Err(EINVAL)
    }

    /// A write operation was called for the device file. `Data` will be borrowed from the Kernel owned data structure. `_data` represents the submitted data buffer that should be written. `_pos` is the current position in the file. The number of written bytes is returned.
    fn write(
        _context: <Self::Data as ForeignOwnable>::Borrowed<'_>,
        _data: &[u8],
        _pos: isize,
    ) -> Result<isize> {
        Err(EINVAL)
    }

    /// All file handles are closed. The ownership of `Data` is retured and the lifetime of the context ends here.
    fn release(_context: Self::Data) -> Result<()> {
        Err(EINVAL)
    }
}

/// Calculates the offset of a field from the beginning of the struct it belongs to.
/// (copied from [Rust For Linux Project](https://github.com/Rust-for-Linux/linux/blob/18b7491480025420896e0c8b73c98475c3806c6f/rust/kernel/lib.rs#L191))
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
/// (copied from [Rust For Linux Project](https://github.com/Rust-for-Linux/linux/blob/18b7491480025420896e0c8b73c98475c3806c6f/rust/kernel/lib.rs#L223))
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
