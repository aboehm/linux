// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.
use core::pin::Pin;

use alloc::boxed::Box;
use kernel::prelude::*;
use kernel::{
    miscdev, new_mutex,
    sync::{Arc, ArcBorrow, Mutex},
};

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Alexander Böhm",
    description: "Rust character device sample",
    license: "GPL",
}

#[pin_data]
struct Context {
    #[pin]
    buffer: Mutex<Vec<u8>>,
}

impl Context {
    fn try_new() -> Result<Arc<Self>> {
        let mut data = Vec::new();
        for i in "Hello CLT\n".as_bytes().iter() {
            data.try_push(*i)?;
        }

        let mutex = pin_init!(Context {
            buffer <- new_mutex!(data),
        });
        Arc::pin_init(mutex)
    }
}

struct Callback {}

impl miscdev::MiscDev for Callback {
    type Data = Arc<Context>;
    type OpenData = Arc<Context>;

    fn open(open_data: &Arc<Context>) -> Result<Arc<Context>> {
        pr_info!("Open data located at {:p}", open_data);
        Ok(open_data.clone())
    }

    fn read(context: ArcBorrow<'_, Context>, count: usize, ppos: isize) -> Result<Vec<u8>> {
        pr_info!("Context data points to {:p}", &context);
        pr_info!("Got read request for {count} bytes from position {ppos}");
        let mut res = Vec::new();
        let mut buffer = context.buffer.lock();
        let len = core::cmp::min(count, buffer.len());
        for i in buffer.drain(..len) {
            res.try_push(i)?;
        }
        Ok(res)
    }

    fn write(context: ArcBorrow<'_, Context>, data: &[u8], _pos: isize) -> Result<isize> {
        let mut buffer = context.buffer.lock();
        for i in data.iter() {
            buffer.try_push(*i)?
        }
        Ok(data.len().try_into().unwrap())
    }
}

struct RustChrdev {
    _registration: Pin<Box<miscdev::Registration<Callback>>>,
}

unsafe impl Sync for RustChrdev {}

impl kernel::Module for RustChrdev {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust device driver init\n");
        pr_info!("*module = {:p}\n", _module);
        let state = Context::try_new()?;
        let registration = miscdev::Registration::new_pinned(state)?;
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
