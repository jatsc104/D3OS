#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use syscall::{syscall1, syscall2, SystemCall, syscall0};

use io::{print, println};
//required for panic handler
use runtime::*;

struct EthernetHeader{
    destination_mac: [u8; 6],
    source_mac: [u8; 6],
    ethertype: u16,
}

impl EthernetHeader{
    fn new(destination_mac: [u8; 6], source_mac: [u8; 6], ethertype: u16) -> Self{
        EthernetHeader{
            destination_mac,
            source_mac,
            ethertype,
        }
    }
    fn to_bytes(&self) -> [u8; 14]{
        let mut header = [0u8; 14];
        header[0..6].copy_from_slice(&self.destination_mac);
        header[6..12].copy_from_slice(&self.source_mac);
        header[12..14].copy_from_slice(&self.ethertype.to_be_bytes());
        header
    }
}

#[no_mangle]
pub fn main() {
    println!("Hello, world!");
    let mac = syscall0(SystemCall::GetMacAddress);
    let EthernetHeader = build_ethernet_header(mac);
    let data_array: [u8; 64] = [0b01010101; 64];
    let mut data_vec = Vec::from(EthernetHeader.to_bytes().to_vec());
    data_vec.extend_from_slice(&data_array);
//TODO: change transmit call to system call - change data_vec to &Vec<u8> and resolve NetworkProtocol enum in syscall
        //furthermore, i have no access to the device, but since it will never change i can get it inside the syscall
    let data_ptr = &data_vec as *const _ as usize;
    syscall2(SystemCall::TransmitData, data_ptr, 0 as usize);
    //transmit(data_vec, NetworkProtocol::Ethernet, &mut device);
    println!("Data sent");
    //wait for the packet to be sent and received/put on the receive queue
    //timer is not accessible in the application, maybe just loop for quite some time
    //Timer::wait(5000);
    for _ in 0..500_000 {
        // This loop does nothing but waste time.
    }
//TODO: change receive_data call to system call -> to return the data, try to give a &mut Vec<u8> as argument
    let received_data: Vec<u8> = Vec::new();
    let received_data_ptr = &received_data as *const _ as usize;
    syscall1(SystemCall::ReceiveData, received_data_ptr);
    println!("Received data: {:?}", received_data);
}

fn build_ethernet_header(mac: usize) -> EthernetHeader{

    //used for debugging in loopback mode, so the packets never leave the card - use broadcast address so i use a valid mac address
    let destination_mac = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let source_mac = usize_to_mac(mac);

    EthernetHeader::new(destination_mac, source_mac, 0x0800)
}

fn usize_to_mac(mac_address_usize: usize) -> [u8; 6] {
    let mac_address = [
        (mac_address_usize & 0xff) as u8,
        ((mac_address_usize >> 8) & 0xff) as u8,
        ((mac_address_usize >> 16) & 0xff) as u8,
        ((mac_address_usize >> 24) & 0xff) as u8,
        ((mac_address_usize >> 32) & 0xff) as u8,
        ((mac_address_usize >> 40) & 0xff) as u8,
    ];
    mac_address
}

