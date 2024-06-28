use alloc::boxed::Box;
use log::info;
use pci_types::InterruptLine;
use x86_64::registers;

use crate::interrupt::interrupt_handler::InterruptHandler;
use crate::interrupt::interrupt_dispatcher::{InterruptVector};
use crate::{apic, interrupt, interrupt_dispatcher};
use crate::device::e1000_register::E1000Registers;

struct E1000InterruptHandler{
    registers: E1000Registers,
}

//seperate impl block needed, since new is not part of InterruptHandler trait
impl E1000InterruptHandler{
    fn new(registers:E1000Registers) -> Self{
        E1000InterruptHandler{
            registers,
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





        info!("Interrupt handled");
    }
}



pub fn map_irq_to_vector(interrupt_line: InterruptLine, registers: E1000Registers){
    //add 32 because first 32 are reserved for cpu exceptions
    let interrupt_vector = InterruptVector::try_from(interrupt_line as u8 + 32).unwrap();
    let handler = Box::new(E1000InterruptHandler::new(registers));
    interrupt_dispatcher().assign(interrupt_vector, handler);
    apic().allow(interrupt_vector);
}