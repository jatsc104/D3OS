
//use super::pit::Timer;
use crate::device::pit::Timer;
use x86_64::VirtAddr;

//all registers defined to be 32 bit wide, should be accessed as 32 bit double words
//and are aligned at 64b bit boundary

pub struct E1000Registers{
    ctrl:u64,   //control register
    status:u64, //status register
    eecd:u64,   //eeprom/flash control register -> config data or update firmware
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

    //interrupt related registers
    icr:u64,    //interrupt cause read register
    ims:u64,    //interrupt mask set/read register
    imc:u64,    //interrupt mask clear register

    //operation related registers
    ral:u64,    //receive address low
    rah:u64,    //receive address high
}

impl Clone for E1000Registers {
    fn clone(&self) -> Self {
        Self {
            ctrl: self.ctrl,
            status: self.status,
            eecd: self.eecd,
            eerd: self.eerd,
            mdic: self.mdic,
            fcal: self.fcal,
            fcah: self.fcah,
            fct: self.fct,
            fcttv: self.fcttv,
            tctl: self.tctl,
            rctl: self.rctl,
            tdbal: self.tdbal,
            tdbah: self.tdbah,
            tdlen: self.tdlen,
            tdh: self.tdh,
            tdt: self.tdt,
            rdbal: self.rdbal,
            rdbah: self.rdbah,
            rdlen: self.rdlen,
            rdh: self.rdh,
            rdt: self.rdt,
            icr: self.icr,
            ims: self.ims,
            imc: self.imc,
            ral: self.ral,
            rah: self.rah,
        }
    }

}

impl E1000Registers{
    pub fn new(mmio_address: VirtAddr) -> Self{
        let mmio_address_u64 = mmio_address.as_u64();
        //init network card - see chapter 14 in intel doc - most over ctrl register
        //move job to e1000 driver - maybe just function call


        Self{
            ctrl: mmio_address_u64 + 0x0000,
            status: mmio_address_u64 + 0x0008,
            eecd: mmio_address_u64 + 0x0010,
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

            //interrupt related registers
            icr: mmio_address_u64 + 0x00C0,
            ims: mmio_address_u64 + 0x0D0,
            imc: mmio_address_u64 + 0x0D8,

            //operation related registers
            ral: mmio_address_u64 + 0x05400,
            rah: mmio_address_u64 + 0x05404,
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

    pub fn read_mac_address(&self) -> [u8;6] {
        const EECD_EE_REQ:u32 = 1 << 6;
        const EECD_EE_GNT:u32 = 1 << 7;
        const EECD_CS:u32 = 1 << 1;

        // Request access to the EEPROM
        self.set_eecd_bit(EECD_EE_REQ);

        // Wait for the EEPROM to grant access
        while self.read_eecd() & EECD_EE_GNT == 0 {
        }

        //enable the EEPROM
        self.set_eecd_bit(EECD_CS);

        let mut mac_address = [0u8;6];
        //mac address is stored in the first three 16 bit words of the eeprom
        for i in 0..3{
            let word = self.read_eeprom((0x0 + i)as u8);
            mac_address[i*2] = (word & 0xFF) as u8;
            mac_address[i*2 + 1] = (word >> 8) as u8;
        }

        // Disable access to the EEPROM
        self.clear_eecd_bit(EECD_CS);

        //Release request
        self.clear_eecd_bit(EECD_EE_REQ);

        mac_address
    }

    fn set_eecd_bit(&self, bit:u32){
        let mut eecd = self.read_eecd();
        eecd |= bit;
        self.write_eecd(eecd);
    }

    fn clear_eecd_bit(&self, bit:u32){
        let mut eecd = self.read_eecd();
        eecd &= !bit;
        self.write_eecd(eecd);
    }

    pub fn read_eeprom(&self, address: u8) -> u16 {

        // Write the address to the EERD register and start the read operation
        self.write_eerd((address as u32) << 8 | 0x1);
    
        // Wait for the read operation to complete
        //if run on real hardware, think about giving the card a short break before reading after write if it crashes
        while self.read_eerd() & 0x10 == 0 {
        }
    
        // Read the data from the EERD register
        let data = self.read_eerd();
    
        // The data is in the upper 16 bits of the EERD register, so shift it down
        (data >> 16) as u16
    }

    pub fn set_mac_address(&self, mac_address: &[u8;6]){
        //set mac address in ral and rah registers
        let ral = u32::from(mac_address[0])
        | (u32::from(mac_address[1]) << 8)
        | (u32::from(mac_address[2]) << 16)
        | (u32::from(mac_address[3]) << 24);
        self.write_ral0(ral);

        let rah = u32::from(mac_address[4])
        | (u32::from(mac_address[5]) << 8);
        //also set AV (Address Valid bit)
        //also set AS to 00 for normal mode - 01 would be filtering based on the source address
        let as_mask:u32 = 0b11 << 16;
        self.write_rah0((rah | (1 << 31))&(!as_mask));
    }

    fn write_ral0(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.ral as *mut u32, value);
        }
    }

    fn write_rah0(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.rah as *mut u32, value);
        }
    }

    pub fn read_eerd(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.eerd as *const u32)
        }
    }

    pub fn write_eerd(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.eerd as *mut u32, value);
        }
    }

    pub fn write_eecd(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.eecd as *mut u32, value);
        }
    }

    pub fn read_eecd(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.eecd as *const u32)
        }
    }

    pub fn read_rctl(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.rctl as *const u32)
        }
    }

    pub fn write_rctl(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.rctl as *mut u32, value);
        }
    }

    pub fn read_tctl(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.tctl as *const u32)
        }
    }

    pub fn write_tctl(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.tctl as *mut u32, value);
        }
    }

    pub fn write_ims(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.ims as *mut u32, value);
        }
    }

    pub fn read_ims(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.ims as *const u32)
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
    pub fn read_rdh(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.rdh as *const u32)
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

    pub fn read_icr(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.icr as *const u32)
        }
    }
    pub fn write_icr(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.icr as *mut u32, value);
        }
    }

    pub fn read_status(&self) -> u32{
        unsafe{
            core::ptr::read_volatile(self.status as *const u32)
        }
    }

    pub fn write_imc(&self, value: u32){
        unsafe{
            core::ptr::write_volatile(self.imc as *mut u32, value);
        }
    }


}