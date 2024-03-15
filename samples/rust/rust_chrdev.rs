// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.
use core::pin::Pin;

use alloc::boxed::Box;
use kernel::miscdev;
use kernel::prelude::*;
use kernel::{
    new_mutex, new_spinlock,
    sync::{Arc, ArcBorrow, Mutex, SpinLock, UniqueArc},
    types::ForeignOwnable,
};

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Alexander Böhm",
    description: "Rust character device sample",
    license: "GPL",
}

#[pin_data]
struct State {
    #[pin]
    buffer: Mutex<Vec<u8>>,
}

impl State {
    fn try_new() -> Result<Arc<Self>> {
        let mut data = Vec::new();
        for i in "Hello CLT\n".as_bytes().iter() {
            data.try_push(*i)?;
        }

        let data = pin_init!(Self {
            buffer <- new_mutex!(data),
        });
        Arc::pin_init(data)
    }
}

struct Callback {}

impl miscdev::MiscDev for Callback {
    type Data = Arc<State>;
    type OpenData = Arc<State>;

    fn open(open_data: &Self::OpenData) -> Result<Self::Data> {
        pr_info!("Open data located at {:p}", open_data);
        Ok(open_data.clone())
    }

    fn read(context: ArcBorrow<'_, State>, count: usize, ppos: isize) -> Result<Vec<u8>> {
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

    fn write(
        context: ArcBorrow<'_, State>,
        data: &[u8],
        _pos: isize,
    ) -> kernel::error::Result<isize> {
        let mut buffer = context.buffer.lock();
        for i in data.iter() {
            buffer.try_push(*i)?
        }
        Ok(data.len().try_into().unwrap())
    }
}

struct RustChrdev {
    registration: Pin<Box<miscdev::Registration<Callback>>>,
}

unsafe impl Sync for RustChrdev {}

impl kernel::Module for RustChrdev {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust device driver init\n");
        pr_info!("*module = {:p}\n", _module);
        let state = State::try_new()?;
        let registration = miscdev::Registration::new_pinned(state)?;
        Ok(RustChrdev { registration })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust device driver exit\n");
        pr_info!("*module = {:p}\n", self);
    }
}
