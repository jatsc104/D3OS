use acpi::platform::interrupt;

use crate::pci_bus;
use pci_types::{EndpointHeader, InterruptLine};
use super::e1000_interrupt::map_irq_to_vector;
use super::e1000_register::E1000Registers;
use super::e1000_pci::{enable_device, get_e1000_device, get_interrupt_line, map_mmio_space};
use super::e1000_descriptor::{set_up_rx_desc_ring};


pub struct IntelE1000Device{
    interrupt_line: InterruptLine,
    registers: E1000Registers,
}

impl IntelE1000Device{

    pub fn new() -> Self{
        let pci_bus = pci_bus();

        let e1000_device = get_e1000_device(pci_bus);
        enable_device(e1000_device, pci_bus);
    //TODO: do rest of interrupt later
        let interrupt_line = get_interrupt_line(pci_bus, e1000_device);
        //also registers interrupt handler and configures apic
        map_irq_to_vector(interrupt_line);


        //need mmio(base)_adress for controller (register access)
        let mmio_adress = map_mmio_space(pci_bus, e1000_device);
        //let controller...
        let registers = E1000Registers::new(mmio_adress);
        registers.init_config_e1000();

        //set up descriptor rings
        set_up_rx_desc_ring(&registers);

        IntelE1000Device{
            interrupt_line: interrupt_line,
            registers: registers,
        }


    }
}