#![feature(ptr_from_ref)]
#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![no_std]

extern crate spin; // we need a mutex in devices::cga_print
extern crate rlibc;
extern crate tinyrlibc;
extern crate alloc;

use alloc::format;
use alloc::string::ToString;
use core::mem::size_of;
use core::panic::PanicInfo;
use chrono::DateTime;
use multiboot2::{BootInformation, BootInformationHeader, Tag};
use multiboot2::MemoryAreaType::{Available};
use x86_64::instructions::interrupts;

// insert other modules
#[macro_use]
mod device;
mod kernel;
mod library;
mod consts;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let terminal = kernel::get_device_service().get_terminal();
    if terminal.is_locked() {
        unsafe { terminal.force_unlock(); };
    }

    println!("Panic: {}", info);
    loop {}
}

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[no_mangle]
pub unsafe extern fn startup(mbi: u64) {
    // Get multiboot information
    let multiboot = BootInformation::load(mbi as *const BootInformationHeader).unwrap();

    // Initialize memory management
    let memory_info = multiboot.memory_map_tag().unwrap();
    let mut heap_area = memory_info.memory_areas().get(0).unwrap();

    for area in memory_info.memory_areas() {
        if area.typ() == Available && area.size() > heap_area.size() {
            heap_area = area;
        }
    }

    kernel::get_memory_service().init(heap_area.start_address() as usize, heap_area.end_address() as usize);

    // Initialize ACPI tables
    let rsdp_addr: usize = if let Some(rsdp_tag) = multiboot.rsdp_v2_tag() {
        core::ptr::from_ref(rsdp_tag) as usize + size_of::<Tag>()
    } else if let Some(rsdp_tag) = multiboot.rsdp_v1_tag() {
        core::ptr::from_ref(rsdp_tag) as usize + size_of::<Tag>()
    } else {
        panic!("ACPI not available!");
    };

    kernel::get_device_service().init_acpi_tables(rsdp_addr);

    // Initialize interrupts
    kernel::get_interrupt_service().init();
    interrupts::enable();

    // Initialize timer;
    kernel::get_device_service().init_timer();
    kernel::get_device_service().get_timer().plugin();

    // Initialize keyboard
    kernel::get_device_service().init_keyboard();
    kernel::get_device_service().get_ps2().plugin_keyboard();

    // Initialize terminal
    let fb_info = multiboot.framebuffer_tag().unwrap().unwrap();
    kernel::get_device_service().init_terminal(fb_info.address() as * mut u8, fb_info.pitch(), fb_info.width(), fb_info.height(), fb_info.bpp());

    let version = format!("v{} ({})", built_info::PKG_VERSION, built_info::PROFILE);
    let date = match DateTime::parse_from_rfc2822(built_info::BUILT_TIME_UTC) {
        Ok(date_time) => date_time.format("%Y-%m-%d %H:%M:%S").to_string(),
        Err(_) => "Unknown".to_string()
    };
    let git_ref = match built_info::GIT_HEAD_REF {
        Some(str) => str,
        None => "Unknown"
    };
    let git_commit = match built_info::GIT_COMMIT_HASH_SHORT {
        Some(str) => str,
        None => "Unknown"
    };
    let bootloader_name = match multiboot.boot_loader_name_tag() {
        Some(tag) => if tag.name().is_ok() { tag.name().unwrap() } else { "Unknown" },
        None => "Unknown"
    };

    println!(include_str!("banner.txt"), version, date, git_ref, git_commit, bootloader_name);
    println!("Boot time: {} ms", kernel::get_device_service().get_timer().get_systime_ms());

    let terminal = kernel::get_device_service().get_terminal();
    loop {
        match terminal.lock().read_byte() {
            -1 => panic!("Terminal input stream closed!"),
            _ => {}
        }
    }
}
