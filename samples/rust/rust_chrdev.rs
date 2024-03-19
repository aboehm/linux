// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.
use core::pin::Pin;

use alloc::boxed::Box;
use kernel::miscdev;
use kernel::prelude::*;

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Alexander Böhm & Antonia Siegert",
    description: "Rust character device sample",
    license: "GPL",
}

struct Callback {}

impl miscdev::MiscDev for Callback {
    type Data = ();
    type OpenData = ();

    fn open(open_data: &()) -> Result<()> {
        pr_info!("Open data located at {:p}", open_data);
        Ok(open_data.clone())
    }

    fn read(context: (), count: usize, ppos: isize) -> Result<Vec<u8>> {
        pr_info!("Context data points to {:p}", &context);
        pr_info!("Got read request for {count} bytes from position {ppos}");
        let mut res = Vec::new();
        for i in "Hello CLT!".bytes() {
            res.try_push(i)?;
        }
        pr_info!(
            "OMG! I do not have a persitent state yet! Will give you the same response FOREVER!"
        );
        Ok(res)
    }

    fn write(context: (), data: &[u8], pos: isize) -> Result<isize> {
        pr_info!("Context data points to {:p}", &context);
        pr_info!(
            "Got write request for {data:?} bytes from position {pos} -> Nope not doing it! Yet.."
        );
        Err(EINVAL)
    }
}

struct RustChrdev {
    // Rust will never read from this, therefore it is assumed dead code, but the kernel does.
    _registration: Pin<Box<miscdev::Registration<Callback>>>,
}

impl kernel::Module for RustChrdev {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust device driver init\n");
        pr_info!("*module = {:p}\n", _module);
        let state = ();
        let registration = miscdev::Registration::new_pinned_registered(state)?;
        Ok(RustChrdev {
            _registration: registration,
        })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust device driver exit\n");
        pr_info!("*module = {:p}\n", self);
    }
}
