use alloc::boxed::Box;
use alloc::vec::Vec;
use log::info;
use pci_types::InterruptLine;

use core::sync::atomic::{AtomicBool, Ordering};

use crate::device::e1000_descriptor::{retrieve_packets, rx_ring_pop, E1000RxDescriptor};
use crate::interrupt::interrupt_handler::InterruptHandler;
use crate::interrupt::interrupt_dispatcher::{InterruptVector};
use crate::{apic, interrupt_dispatcher};
use crate::device::e1000_register::E1000Registers;
use crate::device::e1000_driver::{IntelE1000Device, RxBufferVecToPtr, RxRingVecToPtr};
use crate::device::e1000_driver::{RX_NEW_DATA, RECEIVED_BUFFER};
//use crate::alloc::rc::Rc;

struct E1000InterruptHandler{
    registers: E1000Registers,
    rx_ring: Vec<E1000RxDescriptor>,
    rx_buffer: Vec<Vec<u8>>,
}

//seperate impl block needed, since new is not part of InterruptHandler trait
impl E1000InterruptHandler{
    fn new(registers: E1000Registers, rx_desc_ring: Vec<E1000RxDescriptor>, received_buffer: Vec<Vec<u8>>) -> Self{
        E1000InterruptHandler{
            registers,
            rx_ring: rx_desc_ring,
            rx_buffer: received_buffer,
        }
    }
}

impl InterruptHandler for E1000InterruptHandler{
    fn trigger(&mut self){
        //interrupt handling here

        //icr has to be cleared in the end, otherwise interrupt will be triggered again
        //nvm, reading icr should clear it - setting bits to 1 clears them, setting bits to 0 does nothing

        //read interrupt cause register
        let interrupt_cause = self.registers.read_icr();

        //clear transmit related interrupts for now - Transmit Descriptor Written Back (bit 0) and Transmit Queue Empty (bit 1)
        const ICR_TXDW: u32 = 1 << 0;
        const ICR_TXQE: u32 = 1 << 1;
        const ICR_LSC: u32 = 1 << 2;
        const STATUS_LU: u32 = 1 << 1;
        const ICR_RXDMT0: u32 = 1 << 4;
        //bit 5 is reserved
        const ICR_RXO: u32 = 1 << 6;
        const ICR_RXT0: u32 = 1 << 7;
        //bit 8 is reserved
        //clearing unnecessary - see above
        //self.registers.write_icr(interrupt_cause & !(ICR_TXDW) & !(ICR_TXQE));

        //link status change
        if interrupt_cause & ICR_LSC != 0{
            info!("Link status change");
            let status = self.registers.read_status();

            //log status
            info!("Link Status Change: {}", status);

            //check if link is up - be careful, register should be set, but doesnt get initialized by default
            if (status & STATUS_LU) != 0{
                info!("Link is up");
                //handled
            }
            else{
                info!("Link is down");
                //TODO: pause network activity, try to reestablish connection
            }
        }

        if interrupt_cause & (ICR_RXDMT0 | ICR_RXO) != 0{
            info!("Receive Descriptor Minimum Threshold Reached or Receive Overrun");

            //retrieve_packets(&mut self.rx_ring, &self.registers, &mut self.rx_buffer);
            let packets = RECEIVED_BUFFER.lock();
            retrieve_packets(&mut self.rx_ring, &self.registers, packets);
            //more relaxed forms of ordering could lead to race conditions - i think
            RX_NEW_DATA.store(true, Ordering::SeqCst);

        info!("Interrupt handled");
        }

        if interrupt_cause & (ICR_RXT0) != 0{
            //single packet received - data races between interrupt and new packet possible? maybe two packets received at the "same" time?
            //data race technically handled by RXDMT0 and RXO, regular clearing of packets might be a good idea
            //pop a sinlgle packet from rx_desc_ring. or actually loop starting at the tail up to head-1

            //rx_ring_pop(&mut self.rx_ring, &self.registers, &mut self.rx_buffer);
            let mut packets = RECEIVED_BUFFER.lock();
            rx_ring_pop(&mut self.rx_ring, &self.registers, packets);
            RX_NEW_DATA.store(true, Ordering::SeqCst);

        }

        //UNHANDLED INTERRUPTS

        //MDAC - MDI/O access complete - phy and ethernet controller connected
        //RXCFG - used when forcing link - the latter not done by me
        //PHYINT - phy interrupt
        //GPI_SDP6 - general purpose interrupt
        //GPI_SDP7 - general purpose interrupt
        //GPI - general purpose interrupt
        //TXD_LOW - transmit descriptor low threshold - should be implemented
        //SRPD - small receive packet detected

    }
}


pub fn map_irq_to_vector(interrupt_line: InterruptLine, registers: E1000Registers, rx_desc_ring: Vec<E1000RxDescriptor>, received_buffer: Vec<Vec<u8>>){
    //add 32 because first 32 are reserved for cpu exceptions
    let interrupt_vector = InterruptVector::try_from(interrupt_line as u8 + 32).unwrap();
    let handler = Box::new(E1000InterruptHandler::new(registers, rx_desc_ring, received_buffer));
    interrupt_dispatcher().assign(interrupt_vector, handler);
    apic().allow(interrupt_vector);
}

pub fn enable_interrupts(registers: &E1000Registers) {
    const TXDW: u32 = 1 << 0;
    const TXQE: u32 = 1 << 1;
    const LSC: u32 = 1 << 2;
    //RXSEQ: Receive Sequence Error
    const RXDMT0: u32 = 1 << 4;
    const RXO: u32 = 1 << 6;
    const RXT0: u32 = 1 << 7;
    const MDAC: u32 = 1 << 9;
    const PHYINT: u32 = 1 << 12;
    const GPI1: u32 = 1 << 13;
    const GPI2: u32 = 1 << 14;
    const TXD_LOW: u32 = 1 << 15;
    const SRPD: u32 = 1 << 16;

    let interrupts_to_enable =  TXDW
                                | TXQE
                                | LSC
                                | RXDMT0
                                | RXO
                                | RXT0
                                | MDAC
                                | PHYINT
                                | GPI1
                                | GPI2
                                | TXD_LOW
                                | SRPD;
    
    registers.write_ims(interrupts_to_enable);
}