pub mod io;
pub mod cpu {
    pub fn pause() {
        unsafe {
            core::arch::x86_64::_mm_pause();
        }
    }
}
