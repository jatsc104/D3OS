#![no_std]

extern crate alloc;
extern crate kernel;

use alloc::vec::Vec;
use kernel::e1000_device;
use kernel::device::e1000_interface::{NetworkProtocol, transmit, receive_data};
use kernel::device::e1000_driver::IntelE1000Device;
use kernel::device::pit::Timer;

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
    let mut device = e1000_device();
    let EthernetHeader = build_ethernet_header(&device);
    let data_array: [u8; 64] = [0b01010101; 64];
    let mut data_vec = Vec::from(EthernetHeader.to_bytes().to_vec());
    data_vec.extend_from_slice(&data_array);
    transmit(data_vec, NetworkProtocol::Ethernet, &mut device);
    println!("Data sent");
    //wait for the packet to be sent and received/put on the receive queue
    Timer::wait(5000);
    let received_data = receive_data(&mut device);
    match received_data{
        Some(data) => {
            println!("Received data: {:?}", data);
        }
        None => {
            println!("No data received");
        }
    }
}

fn build_ethernet_header(device: &IntelE1000Device) -> EthernetHeader{

    //used for debugging in loopback mode, so the packets never leave the card - use broadcast address so i use a valid mac address
    let destination_mac = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let source_mac = device.mac_address;

    EthernetHeader::new(destination_mac, source_mac, 0x0800)
}


