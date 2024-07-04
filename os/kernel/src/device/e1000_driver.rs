use acpi::platform::interrupt;
use alloc::vec::Vec;
use log::info;
use spin::Mutex;

use core::sync::atomic::{AtomicBool, Ordering};

use crate::device::pit::Timer;
use crate::pci_bus;
use pci_types::{EndpointHeader, InterruptLine};
use super::e1000_interrupt::{map_irq_to_vector, enable_interrupts};
use super::e1000_register::E1000Registers;
use super::e1000_pci::{enable_device, get_e1000_device, get_interrupt_line, map_mmio_space};
use super::e1000_descriptor::{set_up_rx_desc_ring, set_up_tx_desc_ring, E1000RxDescriptor, E1000TxDescriptor, enable_receive};
//use crate::alloc::rc::Rc;

//these variables are necessary because of the lack of Arc
//it can be argued that these global variables are "okay" because both need to exist for the entire lifetime of the driver, which is likely the lifetime of the whole system
//putting them in a data structure like IntelE1000Device sadly does not work, since i also need two mutable refenerences to each of them - producer and consumer
pub static RX_NEW_DATA: AtomicBool = AtomicBool::new(false);
pub static RECEIVED_BUFFER: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());


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
    pub tx_desc_ring: Vec<E1000TxDescriptor>,
    pub mac_address: [u8; 6],
}

impl IntelE1000Device{

    pub fn new() -> Self{
        let pci_bus = pci_bus();

        let e1000_device = get_e1000_device(pci_bus);
        enable_device(e1000_device, pci_bus);
    //TODO: do rest of interrupt later
        let interrupt_line = get_interrupt_line(pci_bus, e1000_device);
        info!("Interrupt line: {}", interrupt_line);
        Timer::wait(1000);
        
        //need mmio(base)_adress for controller (register access)
        let mmio_adress = map_mmio_space(pci_bus, e1000_device);
        info!("MMIO address: {:?}", mmio_adress.as_u64());
        Timer::wait(1000);
        //let controller...
        let registers = E1000Registers::new(mmio_adress);
        registers.init_config_e1000();
        
        
        //set up descriptor rings
        let rx_desc_ring = set_up_rx_desc_ring(&registers);
        let tx_desc_ring = set_up_tx_desc_ring(&registers);

        //get mac address
        let mac_address = registers.read_mac_address();
        
        //allocate memory for received_buffer
        let received_buffer = Vec::new();

        //if possible, change the following using Rc or Arc - data has to be mutable, that is the problem
        //since this data is assigned to the interrupthandler, it should not get dropped
        //right now, prevent double instances of mut pointers to rx_ring and rx_buffer by only having them in the interrupt handler - hopefully this will suffice
        //else, think about injecting these into map_irq_to_vector
        //talk to fabian about this
        let rx_ring_ptr = RxRingVecToPtr::new(&rx_desc_ring);
        //let rx_buffer_ptr = RxBufferVecToPtr::new(&received_buffer);
        
        //also registers interrupt handler and configures apic
        map_irq_to_vector(interrupt_line, registers.clone(), rx_desc_ring, received_buffer);
        enable_interrupts(&registers);
        
        enable_receive(&registers);

        IntelE1000Device{
            //interrupt_line,
            registers,
            //received_buffer: received_buffer,
            //rx_desc_ring,
            tx_desc_ring,
            mac_address,
        }
        

    }
}