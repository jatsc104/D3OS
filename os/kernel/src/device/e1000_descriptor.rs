use alloc::{rc::Rc, vec::Vec};
use log::info;
use core::mem::MaybeUninit;
use spin::MutexGuard;

use alloc::boxed::Box;
use x86_64::registers;
use crate::device::pit::Timer;

use super::e1000_register::E1000Registers;//::{write_rdbah, write_rdbal, write_rdlen, write_rdh, write_rdt};

// Define the transmit descriptor
#[repr(C)]
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

pub fn set_up_rx_desc_ring(registers: &E1000Registers) -> Vec<E1000RxDescriptor>{

//NOTE: after into_raw, box is consumed and memory has to be managed, cleanup later with Box::from_raw and use Box destructor -> DONE
//NOTE: NOT fit for Multithreading, in case of Multithreading being implemented, synchronise access to receive_ring

    const RECEIVE_RING_SIZE: usize = 128; //maybe change later
    const BUFFER_SIZE: usize = 2048; //maybe change later, can be 256, 512, 1024, 2048, 4096, 8192, 16384 Bytes

    //allocate memory for receive descriptor ring
    //let mut receive_ring: Box<[E1000RxDescriptor]> = vec![E1000RxDescriptor::default(); RECEIVE_RING_SIZE].into_boxed_slice();
    //let mut receive_ring: ArrayVec<[E1000RxDescriptor; RECEIVE_RING_SIZE]> = ArrayVec::new();
    let mut receive_ring: Vec<E1000RxDescriptor> = Vec::with_capacity(RECEIVE_RING_SIZE);

    //init each descriptor in the ring
    //for descriptor in receive_ring.iter_mut(){
    for _ in 0..RECEIVE_RING_SIZE{
        //allocate buffer for desc
        //let buffer = vec![0u8; BUFFER_SIZE].into_boxed_slice();
        //this is very very very not schön, pls change bc this is all undefinded behaviour
        let buffer: [MaybeUninit<u8>; BUFFER_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };
        //pray to god it works and i dont have to touch this again
        let buffer_addr = Box::into_raw(Box::new(buffer)) as u64; //should work if memory is mapped 1:1 to physical memory
        //set buffer address in descriptor for card to write to
        //descriptor.buffer_addr = buffer_addr;
        let descriptor = E1000RxDescriptor{
            buffer_addr,
            ..Default::default()
        };
        receive_ring.push(descriptor);
    }


    //let ring_addr = Box::into_raw(receive_ring) as u64;
    //*const _ as u64 is cast to raw pointer to u64 */
    let ring_addr = receive_ring.as_ptr() as u64;

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
    Timer::wait(4000);

    receive_ring

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

pub struct TxBuffer{
    //check alignment rules in intel Doc;
    //alignment on arbitrary byte size
    //maximum packet size is 16288 bytes
    data: Vec<u8>,
}

impl TxBuffer{
    pub fn new(data: Vec<u8>) -> Self{
        Self{data}
    }

    pub fn size(&self) -> u16{
        self.data.len() as u16
    }

    pub fn address(&self) -> u64{
        self.data.as_ptr() as u64
    }
}



pub fn set_up_tx_desc_ring(registers: &E1000Registers) -> Vec<E1000TxDescriptor>{
//NOT READY YET; FIX INITIALISING
    const NUM_DESCRIPTORS: usize = 64;
    // Allocate memory for the descriptors.
    let mut descriptors: Vec<E1000TxDescriptor> = Vec::with_capacity(NUM_DESCRIPTORS);

    // Initialize each descriptor.
    for descriptor in descriptors.iter_mut() {
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

    E1000Registers::write_tdbal(registers, descriptors.as_ptr() as u32);
    E1000Registers::write_tdbah(registers, ((descriptors.as_ptr() as u64 )>>32) as u32);
    //sizeof(E1000TxDescriptor) should be 16 bytes
    E1000Registers::write_tdlen(registers, (NUM_DESCRIPTORS * core::mem::size_of::<E1000TxDescriptor>()) as u32);
    E1000Registers::write_tdh(registers, 0);
    E1000Registers::write_tdt(registers, NUM_DESCRIPTORS as u32 - 1);

    descriptors
}


//passing tx:ring as slice bc is more flexible
pub fn tx_conncect_buffer_to_descriptors(tx_ring: &mut [E1000TxDescriptor], tx_buffer: &TxBuffer, registers: &E1000Registers){
    let packets = create_packets(tx_buffer);

    const HEADER_SIZE: usize = 14; // ethernet header size
    const MAX_DESCRIPTOR_SIZE: usize = 1500; //limited through ethernet frame size - jumbo frames with 9728 bytes should be supported as well
    const E1000_TXD_CMD_RS: u8 = 1 << 3;
    const E1000_TXD_CMD_EOP: u8 = 1 << 0;

    //are these variables faster than using the registers directly? - i suppose, but do not know
    let tdt = E1000Registers::read_tdt(registers) as usize;
    let mut tdh = E1000Registers::read_tdh(registers) as usize;
    for packet in packets{
        //wait for room in the ring buffer
        //wrap around necessary since tdt could be at the end of the ring buffer
        while(tdt + 1)%tx_ring.len() == tdh{
            //update tdh - card has responsibility to update tdh
            tdh = E1000Registers::read_tdh(registers) as usize;
            //this would be a good spot to make room for other threads in a multithreaded environment, i can wait for a maybe up to 1ms
        }
        //assign header to seperate descriptor
        let header = &packet[..HEADER_SIZE];
        let descriptor = &mut tx_ring[tdt];
        descriptor.buffer_addr = header.as_ptr() as u64;
        descriptor.length = header.len() as u16;
        descriptor.cmd = E1000_TXD_CMD_RS;
        descriptor.status = 0;
        //update tdt
        //tdt = (tdt + 1) % tx_ring.len();
        E1000Registers::write_tdt(registers, ((tdt + 1) % tx_ring.len()) as u32);

        //assign the payload to one or more descriptors
        let mut payload = &packet[HEADER_SIZE..];
        for chunk in payload.chunks(MAX_DESCRIPTOR_SIZE){
            //wait for room in the ring buffer
            while(tdt + 1)%tx_ring.len() == tdh{
            //update tdh - card has responsibility to update tdh
            tdh = E1000Registers::read_tdh(registers) as usize;
        }
            let descriptor = &mut tx_ring[tdt];
            descriptor.buffer_addr = chunk.as_ptr() as u64;
            descriptor.length = chunk.len() as u16;
            descriptor.cmd = E1000_TXD_CMD_RS;
            descriptor.status = 0;
            //update tdt
            //tdt = (tdt + 1) % tx_ring.len();
            E1000Registers::write_tdt(registers, ((tdt + 1) % tx_ring.len()) as u32);
        }
        //old tdt is set to EOP
        tx_ring[(tdt)%tx_ring.len()].cmd |= E1000_TXD_CMD_EOP;
    }
}

pub fn create_packets(tx_buffer: &TxBuffer) -> Vec<Vec<u8>>{
    //placeholders - inject dest and src mac per parameter later
    let destination_mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    let source_mac: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x57];
    let ethertype: [u8; 2] = [0x08, 0x00]; //0x0800 is Ethertype for IPv4
    //combine into one header
    let mut ethernet_header: Vec<u8>= Vec::new();
    ethernet_header.extend_from_slice(&destination_mac);
    ethernet_header.extend_from_slice(&source_mac);
    ethernet_header.extend_from_slice(&ethertype);

    const MAX_PACKET_SIZE: usize = 1500; //limited through ethernet frame size - jumbo frames with 9728 bytes should be supported as well


    let mut packets = Vec::new();
    for chunk in tx_buffer.data.chunks(MAX_PACKET_SIZE){
        let mut packet = Vec::new();
        //Add Header
        packet.extend_from_slice(&ethernet_header);
        //Add Payload
        packet.extend_from_slice(chunk);
//TODO: HEADER GETS RESIZED RIGHT NOW AS WELL - FIX THIS ASAP - resolved?
        //Pad last/header packet to Max size so all packets are same size - helps with debugging
        if packet.len() < MAX_PACKET_SIZE{
            //packet.extend_from_slice(&[0; MAX_PACKET_SIZE - chunk.len()]); - needs to know chunk.len() at compile time
            packet.resize(MAX_PACKET_SIZE, 0);
        }
        packets.push(packet);
    }
    packets
}

pub fn retrieve_packets(receive_ring: &mut Vec<E1000RxDescriptor>, registers: &E1000Registers, mut packets: MutexGuard<Vec<Vec<u8>>>){
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
            packets.push(packet_data.to_vec());
            //calling function still needs to sort packets between multiple programs - is that my responisbilty or the network stacks? - should be done by transport layer

            //reset status 
            descriptor.status = 0;

            //advance rdt
            E1000Registers::write_rdt(registers, ((rdt + 1) % receive_ring_len) as u32);
        }
    }
}

pub fn rx_ring_pop(receive_ring: &mut Vec<E1000RxDescriptor>, registers: &E1000Registers, mut packets:  MutexGuard<Vec<Vec<u8>>>){
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
            packets.push(packet_data.to_vec());

            //advance rdt
            E1000Registers::write_rdt(registers, (rdt + 1) % receive_ring.len() as u32);
        }
    }
    
}

