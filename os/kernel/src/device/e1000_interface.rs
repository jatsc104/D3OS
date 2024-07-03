use alloc::vec::Vec;
use crate::device::e1000_driver::{RX_NEW_DATA, RECEIVED_BUFFER};

use core::sync::atomic::{AtomicBool, Ordering};

pub struct E1000Interface{
    rx_buffer: Vec<Vec<u8>>,
}


//pub fn transmit()
//

pub fn receive() -> Option<Vec<u8>> {
    if RX_NEW_DATA.load(Ordering::SeqCst) {
        let mut rx_buffer = RECEIVED_BUFFER.lock();
        let data = rx_buffer.pop();
        if rx_buffer.is_empty() {
            RX_NEW_DATA.store(false, Ordering::SeqCst);
        }
        data
    }else {
        None
    }
}