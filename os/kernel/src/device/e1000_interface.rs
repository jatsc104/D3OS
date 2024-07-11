use alloc::vec::Vec;
use log::info;
use nolock::queues::spsc::unbounded;
//use crate::device::e1000_driver::{RX_NEW_DATA, RECEIVED_BUFFER};

//use core::sync::atomic::{AtomicBool, Ordering};

use super::e1000_descriptor::{TxBuffer, tx_conncect_buffer_to_descriptors};
use super::e1000_driver::{IntelE1000Device, get_tx_ring};

pub struct E1000Interface{
    rx_buffer: Vec<Vec<u8>>,
    rx_buffer_consumer: unbounded::UnboundedReceiver<Vec<u8>>,
}

pub enum NetworkProtocol{
    //protocols imply the following order of tx_buffer data: ethernet header -> ip header -> tcp/udp header -> payload
    Ethernet,
    Ipv4,
    Ipv6,
    TcpIpv4,
    UdpIpv4,
    TcpIpv6,
    UdpIpv6,
}
impl Clone for NetworkProtocol{
    fn clone(&self) -> Self {
        match self{
            NetworkProtocol::Ethernet => NetworkProtocol::Ethernet,
            NetworkProtocol::Ipv4 => NetworkProtocol::Ipv4,
            NetworkProtocol::Ipv6 => NetworkProtocol::Ipv6,
            NetworkProtocol::TcpIpv4 => NetworkProtocol::TcpIpv4,
            NetworkProtocol::UdpIpv4 => NetworkProtocol::UdpIpv4,
            NetworkProtocol::TcpIpv6 => NetworkProtocol::TcpIpv6,
            NetworkProtocol::UdpIpv6 => NetworkProtocol::UdpIpv6,
        }
    }
}


pub fn transmit(data: Vec<u8>, protocol: NetworkProtocol, device: &IntelE1000Device) {
    //caller has to ensure that the data + the corresponding headers is not larger than the MTU = 1500 bytes
    //but if it is, data gets divided into multiple packets by the driver anyways

    let tx_buffer = TxBuffer::new(data, protocol);
    let mut tx_ring_lock = get_tx_ring().lock();
    let tx_ring = tx_ring_lock.as_mut();
    if let Some(tx_ring) = tx_ring {
        tx_conncect_buffer_to_descriptors(tx_ring, &tx_buffer, &device.registers);
    } else {
        info!("tx_ring could not be obtained for tranmit")
    }
}

pub fn receive_data(device: &mut IntelE1000Device) -> Option<Vec<u8>>{
    device.rx_buffer_consumer.dequeue()
}

//returns ONE packet if available, else None
//pub fn receive() -> Option<Vec<u8>> {
//    if RX_NEW_DATA.load(Ordering::SeqCst) {
//        let mut rx_buffer = RECEIVED_BUFFER.lock();
//        let data = rx_buffer.pop();
//        if rx_buffer.is_empty() {
//            RX_NEW_DATA.store(false, Ordering::SeqCst);
//        }
//        data
//    }else {
//        None
//    }
//}