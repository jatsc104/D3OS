use core::alloc::Layout;

use alloc::alloc::alloc_zeroed;
//use acpi::platform::interrupt;
use alloc::vec::Vec;
use log::info;
use spin::Mutex;
use nolock::queues::mpmc::bounded;

//use core::sync::atomic::AtomicBool;

use crate::device::e1000_descriptor::RxBufferPacket;
use crate::device::pit::Timer;
use crate::{e1000_device, memory, pci_bus};
use super::e1000_interface::{transmit, receive_data, NetworkProtocol};
//use pci_types::{EndpointHeader, InterruptLine};
use super::e1000_interrupt::{map_irq_to_vector, enable_interrupts};
use super::e1000_register::E1000Registers;
use super::e1000_pci::{enable_device, get_e1000_device, get_interrupt_line, map_mmio_space};
use super::e1000_descriptor::{set_up_rx_desc_ring, set_up_tx_desc_ring, E1000RxDescriptor, E1000TxDescriptor, enable_receive, enable_transmit};
//use crate::alloc::rc::Rc;

//these variables are necessary because of the lack of Arc
//it can be argued that these global variables are "okay" because both need to exist for the entire lifetime of the driver, which is likely the lifetime of the whole system
//putting them in a data structure like IntelE1000Device sadly does not work, since i also need two mutable refenerences to each of them - producer and consumer
//pub static RX_NEW_DATA: AtomicBool = AtomicBool::new(false);
//pub static RECEIVED_BUFFER: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

pub const TX_NUM_DESCRIPTORS: usize = 64;
pub const RX_NUM_DESCRIPTORS: usize = 128;

static TX_RING: Mutex<Option<Vec<E1000TxDescriptor>>> = Mutex::new(None);
pub fn get_tx_ring() -> &'static Mutex<Option<Vec<E1000TxDescriptor>>>{
    &TX_RING
}


pub struct RxRingVecToPtr{
    pub ptr: *const E1000RxDescriptor,
    pub len: usize,
    pub cap: usize,
}
pub struct RxBufferVecToPtr{
    pub ptr: *const Vec<u8>,
    pub len: usize,
    pub cap: usize,
}
impl RxRingVecToPtr{
    pub fn new(rx_desc_ring: &Vec<E1000RxDescriptor>) -> Self{
        let ptr_rx_ring = rx_desc_ring.as_ptr();
        let len_rx_ring = rx_desc_ring.len();
        let cap_rx_ring = rx_desc_ring.capacity();
        RxRingVecToPtr{
            ptr: ptr_rx_ring,
            len: len_rx_ring,
            cap: cap_rx_ring,
        }
    }
}
impl RxBufferVecToPtr{
    pub fn new(received_buffer: &Vec<Vec<u8>>) -> Self{
        let ptr_rx_buffer = received_buffer.as_ptr();
        let len_rx_buffer = received_buffer.len();
        let cap_rx_buffer = received_buffer.capacity();
        RxBufferVecToPtr{
            ptr: ptr_rx_buffer,
            len: len_rx_buffer,
            cap: cap_rx_buffer,
        }
    }
}

pub struct IntelE1000Device{
    //maybe that name isnt that fitting anymore, since i only have two of five fields belonging to the card left
    //pub interrupt_line: InterruptLine,
    pub registers: E1000Registers,
    //pub received_buffer: Vec<Vec<u8>>,
    //pub rx_desc_ring: Vec<E1000RxDescriptor>,
    //pub tx_desc_ring: Vec<E1000TxDescriptor>,
    pub mac_address: [u8; 6],
    pub rx_buffer_consumer: bounded::scq::Receiver<RxBufferPacket>,
}

impl IntelE1000Device{

    pub fn new() -> Self{
        let pci_bus = pci_bus();

        let e1000_device = get_e1000_device(pci_bus);
        enable_device(e1000_device, pci_bus);
    //TODO: do rest of interrupt later
        let interrupt_line = get_interrupt_line(pci_bus, e1000_device);
        info!("Interrupt line: {}", interrupt_line);
        
        //need mmio(base)_adress for controller (register access)
        let mmio_adress = map_mmio_space(pci_bus, e1000_device);
        info!("MMIO address: {:?}", mmio_adress.as_u64());
        let registers = E1000Registers::new(mmio_adress);
        registers.init_config_e1000();
        
        
        //set up descriptor rings
        let rx_desc_ring = set_up_rx_desc_ring(&registers);
        //let tx_desc_ring = set_up_tx_desc_ring(&registers);
        initialize_tx_ring();
        set_up_tx_desc_ring(&registers, get_tx_ring());
        //get mac address
        let mac_address = registers.read_mac_address();
        E1000Registers::set_mac_address(&registers, &mac_address);
        
        //allocate memory for received_buffer
        //for now, use a bounded queue to get around having multiple mutable references to the buffer
        //mutable references would have to be synchronized, but since the producer end is passed to the interrupt handler,
        //which deadlocks if it fails to instantly obtain the spinlock, i cannot synchronize the producer end
        //RX_NUM_DESCRIPTORS * 1500 as 1500 is the MTU should be enough to hold at least one time the rx ring.
        let (rx_buffer_consumer, rx_buffer_producer) = bounded::scq::queue::<RxBufferPacket>(RX_NUM_DESCRIPTORS);
        //let received_buffer = Vec::new();

        //if possible, change the following using Rc or Arc - data has to be mutable, that is the problem
        //since this data is assigned to the interrupthandler, it should not get dropped
        //right now, prevent double instances of mut pointers to rx_ring and rx_buffer by only having them in the interrupt handler - hopefully this will suffice
        //else, think about injecting these into map_irq_to_vector
        //let rx_ring_ptr = RxRingVecToPtr::new(&rx_desc_ring);
        //let rx_buffer_ptr = RxBufferVecToPtr::new(&received_buffer);
        
        //also registers interrupt handler and configures apic
        map_irq_to_vector(interrupt_line, registers.clone(), rx_desc_ring, rx_buffer_producer);
        enable_interrupts(&registers);

        //print_tx_ring();
        
        //enable receive and transmit units
        enable_receive(&registers);
        enable_transmit(&registers);

        let ctrl_reg = registers.read_ctrl();
        info!("ctrl reg: {:b}", ctrl_reg);

        IntelE1000Device{
            //interrupt_line,
            registers,
            //received_buffer: received_buffer,
            //rx_desc_ring,
            //tx_desc_ring,
            mac_address,
            rx_buffer_consumer,
        }
        

    }
}

fn initialize_tx_ring() {
    let mut tx_ring = get_tx_ring().lock();
    //let layout = Layout::from_size_align(TX_NUM_DESCRIPTORS * core::mem::size_of::<E1000TxDescriptor>(), 16).unwrap();
    //let transmit_ring_ptr = unsafe { alloc_zeroed(layout) } as *mut E1000TxDescriptor;
    //if transmit_ring_ptr.is_null() {
    //    panic!("Failed to allocate memory for transmit ring");
    //}

    let phys_mem = memory::physical::alloc(1);
    let phys_addr = phys_mem.start.start_address().as_u64();
    //let addr_bool = phys_addr.is_aligned(16);

    let transmit_ring_ptr = phys_addr as *mut E1000TxDescriptor;

    

    //build Vec from pointer
    let mut transmit_ring = unsafe{
        Vec::from_raw_parts(transmit_ring_ptr, TX_NUM_DESCRIPTORS, TX_NUM_DESCRIPTORS)
    };

    for descriptor in &mut transmit_ring {
        *descriptor = E1000TxDescriptor::default();
    }

    //for _ in 0..TX_NUM_DESCRIPTORS {
    //    transmit_ring.push(E1000TxDescriptor::default());
    //}

    *tx_ring = Some(transmit_ring);
}

fn print_tx_ring() {
    let binding = get_tx_ring().lock();
    let tx_ring = binding.as_ref().unwrap();
    for (i, descriptor) in tx_ring.iter().enumerate() {
        info!("Descriptor {}: {:?}", i, descriptor);
    }
}

pub fn e1000_run(){
    let device = e1000_device();
    let mac = device.mac_address;
    let ethernet_header = build_ethernet_header(mac);
    let data_array: [u8; 64] = [0b01010101; 64];
    let mut data_vec = Vec::from(ethernet_header.to_bytes().to_vec());
    data_vec.extend_from_slice(&data_array);
    transmit(data_vec, NetworkProtocol::Ethernet, device);
    info!("Data sent");
    Timer::wait(5000);
    let mut rx_data = Vec::new();
    fetch_rx_data(&mut rx_data);
    fetch_rx_data(&mut rx_data);
    info!("Received data: {:?}", rx_data);
    let status = device.registers.read_status();
    info!("Status: {:b}", status);

}

fn fetch_rx_data(rx_data: &mut Vec<u8>){
    let device = e1000_device();
    let received_data = receive_data(&device);
    match received_data {
        Some(data) => {
            rx_data.extend_from_slice(&data.data[..data.length]);
        }
        None => {}
    }
}

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

fn build_ethernet_header(mac: [u8; 6]) -> EthernetHeader{
    EthernetHeader::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff], mac, 0x0800)
}