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

impl miscdev::MiscDev for ChrdevCallback {
    fn read(count: usize, ppos: isize) -> Result<Vec<u8>> {
        let mut res = Vec::new();
        pr_info!("Got read request for {count} bytes from position {ppos}");
        if ppos == 0 {
            res.try_push(0x30)?;
            res.try_push(0x0a)?;
        }
        Ok(res)
    }
}

impl kernel::Module for RustChrdev {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust device driver init\n");
        pr_info!("*module = {:p}\n", _module);
        let mut reg = miscdev::Registration::<ChrdevCallback>::new_pinned()?;
        miscdev::Registration::register(reg.as_mut())?;
        Ok(RustChrdev(reg))
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust device driver exit\n");
        pr_info!("*module = {:p}\n", self);
    }
}
