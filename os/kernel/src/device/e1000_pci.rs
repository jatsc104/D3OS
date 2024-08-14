#![allow(dead_code)]

use crate::{device::pci::PciBus, memory::PAGE_SIZE, process_manager};
use log::info;
use pci_types::{Bar, CommandRegister, EndpointHeader, InterruptLine};
use x86_64::{structures::paging::{page::PageRange, Page, PageTableFlags}, VirtAddr};



pub fn get_e1000_device(pci_bus: &PciBus)->&EndpointHeader{
    const E1000_VENDOR_ID: u16 = 0x8086;
    const E1000_DEVICE_ID: u16 = 0x100e;
    //look for device on pci bus
    let e1000_devices = pci_bus.search_by_ids(E1000_VENDOR_ID, E1000_DEVICE_ID);
    //check if device is found
    if e1000_devices.len() == 0 {
        panic!("No e1000 device found");
    }
    else if e1000_devices.len() > 1{
        panic!("Multiple e1000 devices found");
    }
    else{
        info!("E1000 device found");
        return e1000_devices[0];
    }
}

pub fn enable_device(device: &EndpointHeader, pci_bus: &PciBus){
    device.update_command(pci_bus.config_space(), |command| {
        command | CommandRegister::BUS_MASTER_ENABLE | CommandRegister::MEMORY_ENABLE
    });
    info!("E1000 enabled");
}

//ditch interrupt_pin, since e1000 only uses INTA# anyways
pub fn get_interrupt_line(pci_bus: &PciBus, e1000_device: &EndpointHeader)->InterruptLine{
    let (_,interrupt_line) = e1000_device.interrupt(pci_bus.config_space());
    interrupt_line
}

pub fn map_mmio_space(pci_bus: &PciBus, e1000_device: &EndpointHeader) -> VirtAddr{
    //get mmio_address
    let bar0 = e1000_device.bar(0, pci_bus.config_space()).unwrap();
    //map mmio space, differenciate between 32 and 64 bit and check if it is mmio(and not pmio)
    let mmio_address: u64;
    let mmio_size: u64;

    if let Bar::Memory32 { address, size, prefetchable:_} = bar0 {
        mmio_address = address as u64;
        mmio_size = size as u64;
        info!("32 bit mmio address: {:?}", mmio_address);
        info!("32 bit mmio size: {:?}", mmio_size);
    }else if let Bar::Memory64 { address, size, prefetchable:_} = bar0 {
        mmio_address = address;
        mmio_size = size;
        info!("64 bit mmio address: {:?}", mmio_address);
    }
    //e1000 can use pmio - not implementing now, maybe later
    else if let Bar::Io {..} = bar0 {
        panic!("E1000 uses mmio, not pmio");
    }
    else{
        panic!("Bar0 is neither mmio nor pmio");
    }

    //set up mmio space
    let virt_mmio_address = VirtAddr::new(mmio_address);
    let pages = mmio_size / PAGE_SIZE as u64;
    let mmio_start_page = Page::from_start_address(virt_mmio_address).expect("e1000 mmio address seems to not be page aligned");
    let kernel_address_space = process_manager().read().kernel_process().unwrap().address_space();
    kernel_address_space.map(
        PageRange {start: mmio_start_page, end: mmio_start_page + pages},
        crate::memory::MemorySpace::Kernel,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
    );
    virt_mmio_address

}