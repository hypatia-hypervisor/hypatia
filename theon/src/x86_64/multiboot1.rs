use multiboot::information::{MemoryManagement, Multiboot, PAddr};

struct MM;

unsafe fn phys_to_slice(phys_addr: PAddr, len: usize) -> Option<&'static [u8]> {
    const THEON_ZERO: PAddr = 0xffff_8000_0000_0000;
    let p = (phys_addr + THEON_ZERO) as usize as *const u8;
    Some(core::slice::from_raw_parts(p, len))
}

impl MemoryManagement for MM {
    unsafe fn paddr_to_slice(&self, phys_addr: PAddr, len: usize) -> Option<&'static [u8]> {
        phys_to_slice(phys_addr, len)
    }

    unsafe fn allocate(&mut self, _len: usize) -> Option<(PAddr, &mut [u8])> {
        None
    }

    unsafe fn deallocate(&mut self, addr: PAddr) {
        if addr != 0 {
            unimplemented!();
        }
    }
}

extern "C" {
    static end: [u64; 0];
}

pub fn init(mbinfo_phys: u64) {
    uart::panic_println!("mbinfo: {:08x}", mbinfo_phys);
    let mut mm = MM {};
    let mb = unsafe { Multiboot::from_ptr(mbinfo_phys as PAddr, &mut mm).unwrap() };
    uart::panic_println!("end = {:016x}", unsafe { end.as_ptr() as usize });
    for module in mb.modules().unwrap() {
        uart::panic_println!("{:x?}", module);
        if Some("bin.a") == module.string {
            let len = (module.end - module.start) as usize;
            let bytes = unsafe { phys_to_slice(module.start, len).unwrap() };
            let archive = goblin::archive::Archive::parse(bytes).unwrap();
            uart::panic_println!("archive: {:#x?}", archive);
        }
    }
    for module in mb.memory_regions().unwrap() {
        uart::panic_println!("{:x?}", module);
    }
}
