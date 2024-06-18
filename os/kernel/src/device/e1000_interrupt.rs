use alloc::boxed::Box;
use log::info;
use pci_types::InterruptLine;

use crate::interrupt::interrupt_handler::InterruptHandler;
use crate::interrupt::interrupt_dispatcher::{InterruptVector};
use crate::{interrupt_dispatcher, apic};

struct E1000InterruptHandler;
impl InterruptHandler for E1000InterruptHandler{
    fn trigger(&mut self){
        //interrupt handling here
        info!("Interrupt handled");
    }
}



pub fn map_irq_to_vector(interrupt_line: InterruptLine){
    //add 32 because first 32 are reserved for cpu exceptions
    let interrupt_vector = InterruptVector::try_from(interrupt_line as u8 + 32).unwrap();
    let handler = Box::new(E1000InterruptHandler{});
    interrupt_dispatcher().assign(interrupt_vector, handler);
    apic().allow(interrupt_vector);
}