
//use super::pit::Timer;
use crate::device::pit::Timer;
use x86_64::VirtAddr;

//all registers defined to be 32 bit wide, should be accessed as 32 bit double words
//and are aligned at 64b bit boundary

pub struct E1000Registers{
    ctrl:u64,   //control register
    status:u64, //status register
    //eecd:u32, //eeprom/flash control register -> config data or update firmware
    eerd:u64,   //eeprom read
    //ctrl_ext:u32,
    mdic:u64,   //config or read PHY
    fcal:u64,   //flow control address low
    fcah:u64,   //flow control address high
    fct:u64,    //flow control type
    //vet:u32,
    fcttv:u64,  //flow control transmit timer value
    tctl:u64,   //transmit control
    rctl:u64,   //receive control

    tdbal:u64,  //transmit descriptor base address low
    tdbah:u64,  //transmit descriptor base address high
    tdlen:u64,  //transmit descriptor length
    tdh:u64,
    tdt:u64,
    //head and tail also available - table 13-2
    rdbal:u64,  //receive descriptor base address low
    rdbah:u64,  //receive descriptor base address high
    rdlen:u64,  //receive descriptor length
    rdh:u64,    //receive descriptor head
    rdt:u64,    //receive descriptor tail
    //head and tail also available - table 13-2
}

impl E1000Registers{
    pub fn new(mmio_address: VirtAddr) -> Self{
        let mmio_address_u64 = mmio_address.as_u64();
        //init network card - see chapter 14 in intel doc - most over ctrl register
        //move job to e1000 driver - maybe just function call


        Self{
            ctrl: mmio_address_u64 + 0x0000,
            status: mmio_address_u64 + 0x0008,
            //let eecd = mmio_address_u64 + 0x0010;
            eerd: mmio_address_u64 + 0x0014,
            //let ctrl_ext = mmio_address_u64 + 0x0018;
            mdic: mmio_address_u64 + 0x0020,
            tctl: mmio_address_u64 + 0x0400,
            rctl: mmio_address_u64 + 0x0100,
            tdbal: mmio_address_u64 + 0x3800,
            tdbah: mmio_address_u64 + 0x3804,
            tdlen: mmio_address_u64 + 0x3808,
            tdh: mmio_address_u64 + 0x3810,
            tdt: mmio_address_u64 + 0x3818,
            rdbal: mmio_address_u64 + 0x2800,
            rdbah: mmio_address_u64 + 0x2804,
            rdlen: mmio_address_u64 + 0x2808,
            rdh: mmio_address_u64 + 0x2810,
            rdt: mmio_address_u64 + 0x2818,

            //just for setting them zero to enable auto-negotiation
            fcal: mmio_address_u64 + 0x0028,
            fcah: mmio_address_u64 + 0x002C,
            fct: mmio_address_u64 + 0x0030,
            fcttv: mmio_address_u64 + 0x0170,
        }
    }

    pub fn init_config_e1000(&self){

        //enable auto-negotiation in eeprom, so card starts auto-negotiation immediately after reset - intel doc 8.5
        //auto negotiation determines duplex resolution and flow control configuration
        //maybe dont touch the eeprom, unless absolutely necessary, since it is kind of, eh, permanent..
        //auto-negotiation should be enabled after reset, refer to intel doc 5.6.7/5-5



        const CTRL_RST: u32 = 1 << 26;
        self.write_ctrl(CTRL_RST);
        //wait for reset to complete
        //timer for 1 ms
        Timer::wait(1);
        //does this work?
        while self.read_ctrl() & CTRL_RST != 0{
            //wait
        }
        //config transmit and receive units


        //set up transmit and reveive descriptor rings

        //enable interrupts


        //set fcal register to 0
        unsafe{
            core::ptr::write_volatile(self.fcal as *mut u32, 0);
        }
        //set fcah register to 0
        unsafe{
            core::ptr::write_volatile(self.fcah as *mut u32, 0);
        }
        //set fct register to 0
        unsafe{
            core::ptr::write_volatile(self.fct as *mut u32, 0);
        }
        //set fcttv register to 0
        unsafe{
            core::ptr::write_volatile(self.fcttv as *mut u32, 0);
        }


    }

    //not sure wether deref pointers in struct would be better, but this is more explicit
    pub fn read_ctrl(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.ctrl as *const u32)
        }
    }
    pub fn write_ctrl(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.ctrl as *mut u32, value);
        }
    }

    pub fn write_rdbal(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.rdbal as *mut u32, value);
        }
    }
    pub fn write_rdbah(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.rdbah as *mut u32, value);
        }
    }
    pub fn write_rdlen(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.rdlen as *mut u32, value);
        }
    }
    pub fn write_rdh(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.rdh as *mut u32, value);
        }
    }
    pub fn write_rdt(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.rdt as *mut u32, value);
        }
    }
    pub fn read_rdt(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.rdt as *const u32)
        }
    }

    pub fn write_tdbal(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.tdbal as *mut u32, value);
        }
    }
    pub fn write_tdbah(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.tdbah as *mut u32, value);
        }
    }
    pub fn write_tdlen(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.tdlen as *mut u32, value);
        }
    }

    pub fn write_tdh(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.tdh as *mut u32, value);
        }
    }
    pub fn read_tdh(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.tdh as *const u32)
        }
    }
    pub fn write_tdt(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.tdt as *mut u32, value);
        }
    }
    pub fn read_tdt(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.tdt as *const u32)
        }
    }


}