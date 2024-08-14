use alloc::vec::Vec;
use log::info;
use nolock::queues::spsc::unbounded;


use super::e1000_descriptor::{TxBuffer, tx_conncect_buffer_to_descriptors, RxBufferPacket, tx_conncect_buffer_to_descriptors_vecless};
use super::e1000_driver::{IntelE1000Device, get_tx_ring};

//Not used, but kept for future feature
#[allow(dead_code)]
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

///transmits data using the vecless transmit version
pub fn transmit_test(data: Vec<u8>, protocol: NetworkProtocol, device: &IntelE1000Device) {
    //caller has to ensure that the data + the corresponding headers is not larger than the MTU = 1500 bytes

    let tx_buffer = TxBuffer::new(data, protocol);
    let mut tx_ring_lock = get_tx_ring().lock();
    let tx_ring = tx_ring_lock.as_mut();
    if let Some(tx_ring) = tx_ring {
        tx_conncect_buffer_to_descriptors_vecless(tx_ring, &tx_buffer, &device.registers);
    } else {
        info!("tx_ring could not be obtained for tranmit")
    }
}

///transmits data using the vec transmit version - should not be called
#[allow(dead_code)]
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

///dequeues one packet from the receive buffer
pub fn receive_data(device: &IntelE1000Device) -> Option<RxBufferPacket>{
    match device.rx_buffer_consumer.try_dequeue() {
        Ok(packet) => Some(packet),
        Err(_) =>{
            info!("Receive Queue is empty");
            None
        }
    }
}
