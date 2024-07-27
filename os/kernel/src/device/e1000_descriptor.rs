use alloc::vec::{self, Vec};
use log::info;
use core::mem::MaybeUninit;
use spin::Mutex;
use nolock::queues::mpmc::bounded;
use alloc::alloc::{GlobalAlloc, Layout, alloc_zeroed};
use core::ptr;

use alloc::boxed::Box;
use x86_64::registers;

use super::e1000_register::E1000Registers;//::{write_rdbah, write_rdbal, write_rdlen, write_rdh, write_rdt};
use super::e1000_interface::NetworkProtocol;
use super::e1000_driver::TX_NUM_DESCRIPTORS;

// Define the transmit descriptor
#[repr(C)]
#[derive(Debug)]
pub struct E1000TxDescriptor {
    buffer_addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

// Define the receive descriptor
#[repr(C)]
pub struct E1000RxDescriptor {
    buffer_addr: u64,
    length: u16,
    csum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

pub struct RxBufferPacket{
    pub length: usize,
    pub data: [u8; 1500],
}

pub fn set_up_rx_desc_ring(registers: &E1000Registers) -> Vec<E1000RxDescriptor>{

//NOTE: after into_raw, box is consumed and memory has to be managed, cleanup later with Box::from_raw and use Box destructor -> DONE
//NOTE: NOT fit for Multithreading, in case of Multithreading being implemented, synchronise access to receive_ring

    const RECEIVE_RING_SIZE: usize = 128; //maybe change later
    const BUFFER_SIZE: usize = 2048; //maybe change later, can be 256, 512, 1024, 2048, 4096, 8192, 16384 Bytes

    //set buffer size to 2048 bytes
    //should be default, but for the sake of completeness
    const E1000_RCTL_BSIZE_2048: u32 = 11 << 16;
    //enable broadcast accept mode - f.e. for ARP
    const E1000_RCTL_BAM: u32 = 1 << 15;
    //set receive descriptor minimum threshold size
    const E1000_RCTL_RDTMS: u32 = 11 << 8;
    //set loopback for testing - full duplex has to be enabled
    const E1000_RCTL_LBM: u32 = 3 << 6;
    let clear_mask = !(E1000_RCTL_BSIZE_2048 | E1000_RCTL_RDTMS);
    //write ctrl register
    let rctl = E1000Registers::read_rctl(registers);
    let settings = (rctl & clear_mask) | E1000_RCTL_BAM | E1000_RCTL_LBM;
    E1000Registers::write_rctl(registers, settings);


    //allocate memory for receive descriptor ring
    //let mut receive_ring: Box<[E1000RxDescriptor]> = vec![E1000RxDescriptor::default(); RECEIVE_RING_SIZE].into_boxed_slice();
    //let mut receive_ring: ArrayVec<[E1000RxDescriptor; RECEIVE_RING_SIZE]> = ArrayVec::new();

    //let mut receive_ring: Vec<E1000RxDescriptor> = Vec::with_capacity(RECEIVE_RING_SIZE);
    let layout = Layout::from_size_align(RECEIVE_RING_SIZE * core::mem::size_of::<E1000RxDescriptor>(), 16).unwrap();
    let receive_ring_ptr = unsafe { alloc_zeroed(layout) } as *mut E1000RxDescriptor;
    if receive_ring_ptr.is_null(){
        panic!("Failed to allocate memory for receive ring");
    }

    //build Vec from pointer - refactoring the rest of the code using the pointer is not worth it
    let mut receive_ring = unsafe {
        Vec::from_raw_parts(receive_ring_ptr, RECEIVE_RING_SIZE, RECEIVE_RING_SIZE)
    };
    
    for descriptor in receive_ring.iter_mut(){
        //let buffer: [MaybeUninit<u8>; BUFFER_SIZE] = unsafe {
        //    MaybeUninit::uninit().assume_init()
        //};
        //let buffer_addr = Box::into_raw(Box::new(buffer)) as u64;

        //let buffer = vec![0u8; BUFFER_SIZE];
        let mut buffer: Vec<u8> = Vec::with_capacity(BUFFER_SIZE);
        //for _ in 0..BUFFER_SIZE{
        //    buffer.push(0);
        //}
        buffer.resize(BUFFER_SIZE, 0);  //note that the memory might get realloced to a different address - all address operations should be done after this.
        let buffer_addr = buffer.as_ptr() as u64;

        //Set buffer address in descriptor for card to write to
        descriptor.buffer_addr = buffer_addr;

        let _ = core::mem::ManuallyDrop::new(buffer);
    }

    //init each descriptor in the ring
    //for descriptor in receive_ring.iter_mut(){
    //for _ in 0..RECEIVE_RING_SIZE{
        //allocate buffer for desc
    //    let buffer: [MaybeUninit<u8>; BUFFER_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };
        //pray to god it works and i dont have to touch this again
    //    let buffer_addr = Box::into_raw(Box::new(buffer)) as u64; //should work if memory is mapped 1:1 to physical memory
        //set buffer address in descriptor for card to write to

    //    unsafe {
    //        let descriptor = &mut *receive_ring_ptr.offset(i as isize);
    //        descriptor.buffer_addr = buffer_addr;
    //    }

        //belongs to the Vec implementation
//        let descriptor = E1000RxDescriptor{
//            buffer_addr,
//            ..Default::default()
//        };
//        receive_ring.push(descriptor);


//    }

    let ring_addr = receive_ring.as_ptr() as u64;
//    let ring_addr = receive_ring_ptr as u64;

    //write base address of receive descriptor ring to card (RDBAL and RDBAH)
    //as u32? or just as is?
    E1000Registers::write_rdbal(registers, ring_addr as u32);
    E1000Registers::write_rdbah(registers, (ring_addr >> 32) as u32);
    
    //set length in rdlen
    E1000Registers::write_rdlen(registers, RECEIVE_RING_SIZE as u32);

    //set head and tail pointers (rdh and rdt)
    E1000Registers::write_rdh(registers, 0);
    E1000Registers::write_rdt(registers, RECEIVE_RING_SIZE as u32 - 1);

    info!("Receive descriptor ring set up");
    info!("Receive descriptor ring address: {:?}", ring_addr);
    info!("RDT - Test: {:?}", E1000Registers::read_rdt(registers));

    receive_ring

}

pub fn enable_receive(registers: &E1000Registers){
    const E1000_RCTL_EN: u32 = 1 << 1;
    let rctl = E1000Registers::read_rctl(registers);
    let enable = rctl | E1000_RCTL_EN;
    E1000Registers::write_rctl(registers, enable);
}

//rewrite so it doesnt dealloc and alloc memory all the time, rather fill old memory with 0's and reuse it
pub fn replenish_rx_desc_ring(receive_ring: &mut Vec<E1000RxDescriptor>, registers: &E1000Registers){
    const BUFFER_SIZE: usize = 2048; //maybe change later, can be 256, 512, 1024, 2048, 4096, 8192, 16384 Bytes
    const E1000_RX_STATUS_DD: u8 = 1 << 0;

    //prevent immutable borrow of already mutable borrowed value receive_ring
    //try preventing using const RECEIVE_RING_SIZE so i only need to change the size at one place
    let receive_ring_len = receive_ring.len();

    for descriptor in receive_ring.iter_mut(){
        //check for processing in status field
        if descriptor.status & E1000_RX_STATUS_DD != 0{

            //dealloc old buffer
            if descriptor.buffer_addr != 0{
                unsafe{
                    let _old_buffer = Box::from_raw(descriptor.buffer_addr as *mut [MaybeUninit<u8>; BUFFER_SIZE]);
                    //old_buffer dropped here
                }
            }    
            //allocate new buffer for desc
            //inner Box now in first line, just a bit more readable
//TODO: high memory use right now. check if reusing buffers is possible
            let buffer: Box<[MaybeUninit<u8>; BUFFER_SIZE]> = Box::new(unsafe { MaybeUninit::uninit().assume_init() });
            let buffer_addr = Box::into_raw(buffer) as *mut _ as u64;

            //update descriptor with new buffer address
            descriptor.buffer_addr = buffer_addr;
            //update status field to 0
            descriptor.status = 0;

            //upate tail pointer
            let rdt = E1000Registers::read_rdt(registers);
            E1000Registers::write_rdt(registers, (rdt + 1) % receive_ring_len as u32);
        }
    }
}

//probably not necessary, but just in case i need to free memory without killing the whole driver
pub fn cleanup_receive_ring(receive_ring: &mut Vec<E1000RxDescriptor>){
    const BUFFER_SIZE: usize = 2048; //maybe change later, can be 256, 512, 1024, 2048, 4096, 8192, 16384 Bytes

    for descriptor in receive_ring.iter_mut(){
        //dealloc old buffer
        if descriptor.buffer_addr != 0{
            unsafe{
                let _old_buffer = Box::from_raw(descriptor.buffer_addr as *mut [MaybeUninit<u8>; BUFFER_SIZE]);
                //old_buffer dropped here
            }
        }
    }
}

impl Default for E1000RxDescriptor {
    fn default() -> Self {
        E1000RxDescriptor {
            buffer_addr: 0,
            length: 0,
            csum: 0,
            status: 0,
            errors: 0,
            special: 0,
        }
    }
}

impl Default for E1000TxDescriptor {
    fn default() -> Self {
        E1000TxDescriptor {
            buffer_addr: 0,
            length: 0,
            cso: 0,
            cmd: 0,
            status: 0,
            css: 0,
            special: 0,
        }
    }
}

pub struct TxBuffer{
    //check alignment rules in intel Doc;
    //alignment on arbitrary byte size
    //maximum packet size is 16288 bytes
    data: Vec<u8>,
    protocol: NetworkProtocol,
}

impl TxBuffer{
    pub fn new(data: Vec<u8>, protocol: NetworkProtocol) -> Self{
        Self{data, protocol}
    }

    pub fn size(&self) -> u16{
        self.data.len() as u16
    }

    pub fn address(&self) -> u64{
        self.data.as_ptr() as u64
    }
    
    pub fn protocol(&self) -> NetworkProtocol{
        self.protocol.clone()
    }
}


///initialises a given transmit descriptor ring with size TX_NUM_DESCRIPTORS
pub fn set_up_tx_desc_ring(registers: &E1000Registers, tx_ring: &'static Mutex<Option<Vec<E1000TxDescriptor>>>) {
//NOT READY YET; FIX INITIALISING
    //const TX_NUM_DESCRIPTORS: usize = 64;

    const E1000_TCTL_PSP: u32 = 1 << 3;     // Pad short packets
    const E1000_TCTL_CT: u32 = 0x0F << 4;  // Collision threshold - only has meaning in half duplex
    const E1000_TCTL_COLD_FD: u32 = 0x40 << 12;// Collision distance Full Duplex - recommended 0x40
    const E1000_TCTL_COLD_HD: u32 = 0x200 << 12;// Collision distance Half Duplex - recommended 0x200

    let tctl = E1000Registers::read_tctl(registers);
    let settings = tctl | E1000_TCTL_PSP | E1000_TCTL_CT | E1000_TCTL_COLD_FD;
    E1000Registers::write_tctl(registers, settings);
    // Allocate memory for the descriptors.
    //let mut descriptors: Vec<E1000TxDescriptor> = Vec::with_capacity(TX_NUM_DESCRIPTORS);
    let mut descriptors = tx_ring.lock();
    match descriptors.as_mut(){
        Some(descriptors_vec) => {

            
            // Initialize each descriptor.
            for descriptor in descriptors_vec.iter_mut() {
                // Set the buffer address to the address of some buffer.
                // This is where the E1000 card will read data to transmit.
                descriptor.buffer_addr = 0;
                
                // Set the length to the length of the data to transmit.
                descriptor.length = 0;
                
                //TODO: Set the command field to indicate that this descriptor is ready to be used.
                //descriptor.cmd = E1000_TXD_CMD_RS | E1000_TXD_CMD_EOP;
                
                // Set the status field to 0 to indicate that this descriptor has not been used yet.
                descriptor.status = 0;
            }
        }
        None => {
            info!("Tx Ring not initialized");
        }
    }

    if let Some(descriptors_vec) = descriptors.as_ref() {
        let descriptors_ptr = descriptors_vec.as_ptr();
        
        E1000Registers::write_tdbal(registers, descriptors_ptr as u32);
        E1000Registers::write_tdbah(registers, ((descriptors_ptr as u64 )>>32) as u32);
    } else {
        info!("Tx Ring not initialized");
    }
    //sizeof(E1000TxDescriptor) should be 16 bytes
    E1000Registers::write_tdlen(registers, (TX_NUM_DESCRIPTORS * core::mem::size_of::<E1000TxDescriptor>()) as u32);
    E1000Registers::write_tdh(registers, 0);
    E1000Registers::write_tdt(registers, TX_NUM_DESCRIPTORS as u32 - 1);

//    descriptors
}

pub fn enable_transmit(registers: &E1000Registers){
    const E1000_TCTL_EN: u32 = 1 << 1;
    let tctl = E1000Registers::read_tctl(registers);
    E1000Registers::write_tctl(registers, tctl | E1000_TCTL_EN);
}

fn print_tx_ring(tx_ring: &Vec<E1000TxDescriptor>){
    for (i, descriptor) in tx_ring.iter().enumerate(){
        info!("Descriptor {}: {:?}", i, descriptor);
    }
}
fn print_tdt(registers: &E1000Registers){
    info!("TDT: {:?}", E1000Registers::read_tdt(registers));
}
fn print_tdh(registers: &E1000Registers){
    info!("TDH: {:?}", E1000Registers::read_tdh(registers));
}


//passing tx:ring as slice bc is more flexible
//passing tx_ring as Vec so i can calculate here which part to access, i also need tx_ring.len() for the wrap around calculation
pub fn tx_conncect_buffer_to_descriptors(tx_ring: &mut Vec<E1000TxDescriptor>, tx_buffer: &TxBuffer, registers: &E1000Registers){
    let packets = create_packets(tx_buffer);

    let header_size = get_header_size(tx_buffer);

    //const HEADER_SIZE: usize = 14; // ethernet header size
    const MAX_DESCRIPTOR_SIZE: usize = 4096;    //should not really matter, as long as jumbo frames are not supported, since MTU is 1500 bytes.
                                                //this function should support jumbo frames, but create_packets does not
    const E1000_TXD_CMD_RS: u8 = 1 << 3;
    const E1000_TXD_CMD_EOP: u8 = 1 << 0;
    const SAFETY_MARGIN: usize = 1;

    //are these variables faster than using the registers directly? - i suppose, but do not know
    let mut tdt = E1000Registers::read_tdt(registers) as usize;
    let mut tdh = E1000Registers::read_tdh(registers) as usize;
    let tx_ring_len = tx_ring.len();
    for packet in packets{
        //wait for room in the ring buffer
        //wrap around necessary since tdt could be at the end of the ring buffer
        //maybe just check if tdh is close to 0, that would indicate that the ring buffer is pretty empty
//TODO: not correct. tdh is the value of HEAD corresponding to other descriptors in the cards internal fifo output queue.
        while(tdt + 1 + SAFETY_MARGIN)%tx_ring_len == tdh{
            //update tdh - card has responsibility to update tdh
            tdh = E1000Registers::read_tdh(registers) as usize;
            //this would be a good spot to make room for other threads in a multithreaded environment
        }
        //assign header to seperate descriptor
        let header = &packet[..header_size];
        let descriptor = &mut tx_ring[tdt];
        descriptor.buffer_addr = header.as_ptr() as u64;
        descriptor.length = header.len() as u16;
        descriptor.cmd = E1000_TXD_CMD_RS;
        descriptor.status = 0;
        //update tdt
        //tdt = (tdt + 1) % tx_ring.len();
        print_tx_ring(tx_ring);
        print_tdh(registers);
        print_tdt(registers);
        E1000Registers::write_tdt(registers, ((tdt + 1) % tx_ring.len()) as u32);
        print_tdt(registers);
        print_tdh(registers);

        //assign the payload to one or more descriptors - jumbo frames not supported so each packet should be smaller than 4096 bytes
        let payload = &packet[header_size..];
        //calculate number of chunks rounding up
        let num_chunks = (payload.len() + MAX_DESCRIPTOR_SIZE - 1) / MAX_DESCRIPTOR_SIZE;
        for (i, chunk) in payload.chunks(MAX_DESCRIPTOR_SIZE).enumerate(){
            //update tdt
            tdt = E1000Registers::read_tdt(registers) as usize;
            //wait for room in the ring buffer
            while tdt == tdh{
            //update tdh - card has responsibility to update tdh
            tdh = E1000Registers::read_tdh(registers) as usize;
            }
            let descriptor = &mut tx_ring[tdt];
            descriptor.buffer_addr = chunk.as_ptr() as u64;
            descriptor.length = chunk.len() as u16;
            descriptor.cmd = E1000_TXD_CMD_RS;
            descriptor.status = 0;
            if i == num_chunks - 1 {
                descriptor.cmd |= E1000_TXD_CMD_EOP;
            }
            //update tdt
            //tdt = (tdt + 1) % tx_ring.len();
            E1000Registers::write_tdt(registers, ((tdt + 1) % tx_ring.len()) as u32);
        }
        //old tdt is set to EOP
        //tx_ring[(tdt)%tx_ring_len].cmd |= E1000_TXD_CMD_EOP;
    }
}

pub fn create_packets(tx_buffer: &TxBuffer) -> Vec<Vec<u8>>{

    let header_size = get_header_size(tx_buffer);

    //placeholders - inject dest and src mac per parameter later
//    let destination_mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
//    let source_mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x57];
//    let ethertype: [u8; 2] = [0x08, 0x00]; //0x0800 is Ethertype for IPv4
    //combine into one header
    let header = &tx_buffer.data[..header_size]; 

    //MTU = Maximum Transmission Unit - Maximum size of a packet including header
    const MTU: usize = 1500; //limited through ethernet frame size - jumbo frames with 9728 bytes should be supported as well


    let mut packets = Vec::new();
    //header gets put into the same packet as the payload here, but gets assigned to a seperate descriptor in tx_connect_buffer_to_descriptors
    //this ensures each header to have the same lifetime as the payload, as they live in the same Vec
    for chunk in tx_buffer.data.chunks(MTU){
        let mut packet = Vec::new();
        //Add Header
        packet.extend_from_slice(&header);
        //Add Payload
        packet.extend_from_slice(chunk);
//TODO: HEADER GETS RESIZED RIGHT NOW AS WELL - FIX THIS ASAP - resolved?
        //Pad last/header packet to Max size so all packets are same size - helps with debugging
//        if packet.len() < MTU{
            //packet.extend_from_slice(&[0; MAX_PACKET_SIZE - chunk.len()]); - needs to know chunk.len() at compile time
//            packet.resize(MTU, 0);
//        }
        packets.push(packet);
    }
    packets
}

pub fn get_header_size(tx_buffer: &TxBuffer) -> usize {
    const ETHERNET_HEADER_SIZE: usize = 14;
    const IPV6_HEADER_SIZE: usize = 40;
    const UDP_HEADER_SIZE: usize = 8;

    let protocol = &tx_buffer.protocol;

    //IPv4 and TCP have variable header sizes
    match protocol{
        NetworkProtocol::Ethernet => ETHERNET_HEADER_SIZE,
        NetworkProtocol::Ipv4 => {
            //IHL field is the lower 4 bits of the first byte
            let ihl = tx_buffer.data[0] & 0x0F;
            //IHL field is the number of 32 bit words, so multiply by 4 to get the number of bytes
            (ihl as usize * 4) + ETHERNET_HEADER_SIZE
        },
        NetworkProtocol::Ipv6 => IPV6_HEADER_SIZE + ETHERNET_HEADER_SIZE,
        NetworkProtocol::TcpIpv4 => {
            let ihl = tx_buffer.data[ETHERNET_HEADER_SIZE] & 0x0F;
            let ip_header_size = ihl as usize * 4;
            //Data offset field is the upper 4 bits of the 12th byte
            let data_offset = tx_buffer.data[ETHERNET_HEADER_SIZE + ip_header_size + 12] >> 4;
            //Data offset field is the number of 32 bit words, so multiply by 4 to get the number of bytes
            let tcp_header_size = data_offset as usize * 4;
            ETHERNET_HEADER_SIZE + ip_header_size + tcp_header_size
        },
        NetworkProtocol::UdpIpv4 => {
            let ihl = tx_buffer.data[ETHERNET_HEADER_SIZE] & 0x0F;
            let ip_header_size = ihl as usize * 4;
            ETHERNET_HEADER_SIZE + ip_header_size + UDP_HEADER_SIZE
        },
        NetworkProtocol::TcpIpv6 => {
            let data_offset = tx_buffer.data[ETHERNET_HEADER_SIZE + IPV6_HEADER_SIZE + 12] >> 4;
            let tcp_header_size = data_offset as usize * 4;
            ETHERNET_HEADER_SIZE + IPV6_HEADER_SIZE + tcp_header_size
        },
        NetworkProtocol::UdpIpv6 => UDP_HEADER_SIZE + IPV6_HEADER_SIZE + ETHERNET_HEADER_SIZE,
    }
}

pub fn retrieve_packets(receive_ring: &mut Vec<E1000RxDescriptor>, registers: &E1000Registers, rx_buffer_producer: &bounded::scq::Sender<RxBufferPacket>){
    //packets should be owned by the caller to avoid transfering ownership upwards the calling hierarchy
    //packets and registers not thread safe
    const E1000_RX_STATUS_DD: u8 = 1 << 0;
    let receive_ring_len = receive_ring.len();

    for descriptor in receive_ring.iter_mut(){
        //have the read here leads to some overhead, but rdt can stay immutable and each loop iteration can be synchronised seperately in case of multithreading
        let rdt = E1000Registers::read_rdt(registers) as usize;
        if descriptor.status & E1000_RX_STATUS_DD != 0{
            //retrieve packet data and its length
            let length = descriptor.length as usize;
            let packet_data = unsafe{
                core::slice::from_raw_parts(descriptor.buffer_addr as *const u8, length)
            };

            //error checking - drop packet if error
            if descriptor.errors != 0{

                //reset status
                descriptor.status = 0;

                //advance rdt
                E1000Registers::write_rdt(registers, ((rdt + 1) % receive_ring_len) as u32);

                continue;
            }

            //add potential packet filter here

            //add packet to provided Vector
            //packets.push(packet_data.to_vec());
            //rx_buffer_producer.enqueue(packet_data.to_vec());
            enqueue_packet(rx_buffer_producer, packet_data);
            //calling function still needs to sort packets between multiple programs - is that my responisbilty or the network stacks? - should be done by transport layer

            //reset status 
            descriptor.status = 0;

            //advance rdt
            E1000Registers::write_rdt(registers, ((rdt + 1) % receive_ring_len) as u32);
        }
    }
}

fn enqueue_packet(rx_buffer_producer: &bounded::scq::Sender<RxBufferPacket>, packet_data: &[u8]){
    //using the struct with a fixed size to avoid needing to serialise the data.
    let mut packet = RxBufferPacket{
        length: packet_data.len(),
        data: [0; 1500],
    };
    packet.data[..packet_data.len()].copy_from_slice(packet_data);
    //dont call expect here, since it would panic if the buffer is full
    rx_buffer_producer.try_enqueue(packet).unwrap_or_else(|_| {
        info!("Recieved Packet could not be enqueued - Buffer probably full");
    });
}

    


pub fn rx_ring_pop(receive_ring: &mut Vec<E1000RxDescriptor>, registers: &E1000Registers, rx_buffer_producer: &bounded::scq::Sender<RxBufferPacket>){
    let rdh = E1000Registers::read_rdh(registers);
    let rdt = E1000Registers::read_rdt(registers);
    
    //check if there are packets to process
    if rdh != rdt{
        const E1000_RX_STATUS_DD: u8 = 1 << 0;
        let descriptor = &mut receive_ring[rdt as usize];
        //check if descriptor is ready to be processed
        if descriptor.status & E1000_RX_STATUS_DD != 0 {
            //packet is ready to be processed
            let length = descriptor.length as usize;
            let packet_data = unsafe{
                core::slice::from_raw_parts(descriptor.buffer_addr as *const u8, length)
            };
            descriptor.status = 0;

            //error check - drop packet if true
            if descriptor.errors != 0{
                //advance rdt
                E1000Registers::write_rdt(registers, (rdt + 1) % receive_ring.len() as u32);
                return;
            }

            //add potential packet filter here

            //add packet to provided Vector
            //packets.push(packet_data.to_vec());
            //rx_buffer_producer.enqueue(packet_data.to_vec());
            enqueue_packet(rx_buffer_producer, packet_data);

            //advance rdt
            E1000Registers::write_rdt(registers, (rdt + 1) % receive_ring.len() as u32);
        }
    }
    
}

