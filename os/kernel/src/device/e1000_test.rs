use log::info;
use alloc::vec::Vec;

use crate::e1000_device;
use super::e1000_driver::{IntelE1000Device, build_ethernet_header};
use super::e1000_descriptor::{E1000RxDescriptor, E1000TxDescriptor};
use super::e1000_register::E1000Registers;


pub fn e1000_fake_test(){
    let device = e1000_device();
    let mac = device.mac_address;
//    let ethernet_header = build_ethernet_header(mac);
    let data_array: [u8; 64] = [0b01010101; 64];
//    let mut data_vec = Vec::from(ethernet_header.to_bytes().to_vec());
//    data_vec.extend_from_slice(&data_array);
    //fake_transmit(data_vec, device);
}

pub fn fake_transmit(data_vec: Vec<u8>, rx_ring: &mut Vec<E1000RxDescriptor>, device: IntelE1000Device) {
    //should be called, when TXDW interrupt is received

}

pub fn fake_transmit_lbm(rx_ring: &mut Vec<E1000RxDescriptor>, registers: &E1000Registers) {
    //should be called, when TXDW interrupt is received
    //while i am at it, i should dealloc the sent data here

    //get sent data from tx_ring
    //has to be done via registers, not obtaining a lock within the interrupt handler leads to a deadlock
    let tx_ring_low = registers.read_tdbal();
    let tx_ring_high = registers.read_tdbah();
    let tx_ring_len = registers.read_tdlen();
    let tx_ring_head = registers.read_tdh();
    let tx_ring_tail = registers.read_tdt();
    //combine tx_ring_low and tx_ring_high to get the address of the ring
    let tx_ring_address = (tx_ring_high as u64) << 32 | tx_ring_low as u64;
    //get the index of the descriptor that was sent
//    let sent_index = (tx_ring_tail - 1) % (tx_ring_len/16);
    let sent_index = if tx_ring_tail == 0 {
        (tx_ring_len / 16) - 1
    } else {
        (tx_ring_tail - 1) % (tx_ring_len / 16)
    };
    //get the descriptor that was sent
    let tx_ring_ptr = tx_ring_address as *mut E1000TxDescriptor;
    let sent_descriptor_ptr = unsafe { tx_ring_ptr.offset(sent_index as isize) };
    let sent_descriptor = unsafe { sent_descriptor_ptr.as_mut().unwrap_or_else(|| {
        info!("Sent descriptor is null");
        panic!();
    }) };
    //get the data that was sent
    let sent_data = sent_descriptor.buffer_addr as *const u8;
    let sent_data_len = sent_descriptor.length as usize;

    //copy data to rx_ring
    //get current rx_descriptor
    let rx_ring_head = registers.read_rdh();
    let rdlen = registers.read_rdlen();
    let mut i = 0;
    for descriptor in rx_ring.iter_mut(){
        if i == rx_ring_head as usize{
            //copy data to rx_descriptor
            let rx_data =   descriptor.buffer_addr as *mut u8;
            unsafe {
                core::ptr::copy_nonoverlapping(sent_data, rx_data, sent_data_len);
            }
            descriptor.length = sent_data_len as u16;
            descriptor.status = 1;
            //increment rx_ring_head
            registers.write_rdh(((rx_ring_head + 1) % (rdlen/16)) as u32);
        }
        //increment i
        i += 1;
    }
}