use alloc::vec::Vec;
use core::mem::MaybeUninit;

use alloc::boxed::Box;
use x86_64::registers;
use super::e1000_register::E1000Registers;//::{write_rdbah, write_rdbal, write_rdlen, write_rdh, write_rdt};

// Define the transmit descriptor
#[repr(C)]
struct E1000TxDescriptor {
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
struct E1000RxDescriptor {
    buffer_addr: u64,
    length: u16,
    csum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

pub fn set_up_rx_desc_ring(registers: &E1000Registers){

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






}

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



