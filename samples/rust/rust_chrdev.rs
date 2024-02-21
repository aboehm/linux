// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.
use core::pin::Pin;

use alloc::boxed::Box;
use kernel::miscdev;
use kernel::prelude::*;

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Alexander Böhm",
    description: "Rust character device sample",
    license: "GPL",
}

struct RustChrdev(Pin<Box<miscdev::Registration<ChrdevCallback>>>);

unsafe impl Sync for RustChrdev {}

struct ChrdevCallback;

#[vtable]
impl miscdev::MiscDev for ChrdevCallback {
    fn read(_count: usize) -> Vec<u8> {
        Vec::new()
    }
}

impl kernel::Module for RustChrdev {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust device driver init\n");
        pr_info!("*module = {:p}\n", _module);
        let mut reg = miscdev::Registration::<ChrdevCallback>::new_pinned()?;
        pr_info!("reg = {:p}\n", reg);
        {
            let reg = reg.as_mut();
            pr_info!("reg unchecked mut = {:p}\n", reg);
            let reg = unsafe { reg.get_unchecked_mut() };
            pr_info!("reg unchecked mut = {:p}\n", reg);
            miscdev::Registration::register(reg)?;
        }
        Ok(RustChrdev(reg))
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust device driver exit\n");
        pr_info!("*module = {:p}\n", self);
    }
}
