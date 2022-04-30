use core::arch::asm;
use seq_macro::seq;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Frame {
    // Pushed by software.
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    // It is arguable whether we should care about
    // these registers.  x86 segmentation (aside from
    // FS and GS) isn't used once we're in long mode,
    // and we doesn't support real or compatibility
    // mode in the hypervisor, so these are effectively
    // unused.
    //
    // Regardless, they exist, so we save and restore
    // them.  Some kernels do this, some do not.  Note
    // that %fs and %gs are special.
    ds: u64, // Really these are u16s, but
    es: u64, // we waste a few bytes to keep
    fs: u64, // the stack aligned.  Thank
    gs: u64, // you, x86 segmentation.

    vector: u64,

    // Sometimes pushed by hardware.
    pub error: u64,

    // Pushed by hardware.
    pub rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

const TRAPFRAME_VECTOR_OFFSET: usize = 19 * core::mem::size_of::<u64>();
const TRAPFRAME_CS_OFFSET: usize = 22 * core::mem::size_of::<u64>();

#[repr(transparent)]
pub struct Stub(usize);

macro_rules! gen_stub {
    ($vecnum:literal) => {
        concat!(r#".balign 8; pushq $0; callq {trap}; .byte "#, $vecnum, ";")
    };
    ($vecnum:literal, err) => {
        concat!(r#".balign 8; callq {trap}; .byte "#, $vecnum, ";")
    };
}

macro_rules! gen_trap_stub {
    // These cases include hardware-generated error words
    // on the trap frame
    (8) => {
        gen_stub!(8, err)
    };
    (10) => {
        gen_stub!(10, err)
    };
    (11) => {
        gen_stub!(11, err)
    };
    (12) => {
        gen_stub!(12, err)
    };
    (13) => {
        gen_stub!(13, err)
    };
    (14) => {
        gen_stub!(14, err)
    };
    (17) => {
        gen_stub!(17, err)
    };
    // No hardware error
    ($num:literal) => {
        gen_stub!($num)
    };
}

pub fn stubs() -> &'static [Stub; 256] {
    unsafe { &*(trap_stubs as usize as *const [Stub; 256]) }
}

/// # Safety
///
/// Container for thunks.
#[allow(dead_code)]
#[link_section = ".trap"]
#[naked]
pub unsafe extern "C" fn trap_stubs() -> ! {
    asm!(
        seq!(N in 0..=255 {
            concat!( #( gen_trap_stub!(N), )* )
        }),
        trap = sym trap, options(att_syntax, noreturn));
}

/// # Safety
///
/// Common trap handler.  Called from interrupt/exception stub.
#[link_section = ".trap"]
#[naked]
pub unsafe extern "C" fn trap() -> ! {
    asm!(r#"
        // Allocate space to save registers.
        subq $((4 + 15) * 8), %rsp
        // Save the x86 segmentation registers.
        movq $0, 18*8(%rsp);
        movw %gs, 18*8(%rsp);
        movq $0, 17*8(%rsp);
        movw %fs, 17*8(%rsp);
        movq $0, 16*8(%rsp);
        movw %es, 16*8(%rsp);
        movq $0, 15*8(%rsp);
        movw %ds, 15*8(%rsp);
        // Save the general purpose registers.
        movq %r15, 14*8(%rsp);
        movq %r14, 13*8(%rsp);
        movq %r13, 12*8(%rsp);
        movq %r12, 11*8(%rsp);
        movq %r11, 10*8(%rsp);
        movq %r10, 9*8(%rsp);
        movq %r9, 8*8(%rsp);
        movq %r8, 7*8(%rsp);
        movq %rbp, 6*8(%rsp);
        movq %rdi, 5*8(%rsp);
        movq %rsi, 4*8(%rsp);
        movq %rdx, 3*8(%rsp);
        movq %rcx, 2*8(%rsp);
        movq %rbx, 1*8(%rsp);
        movq %rax, 0*8(%rsp);
        // Fix up the vector number.  We got into `trap` via a
        // CALL, so hardware pushed the address after after the
        // CALLQ instruction onto the stack, and the byte at that
        // location is the vector number from the stub.  Load
        // the "return" address from the stack, then MOVZBQ what
        // that points to into %rdi, and store back in the save
        // area.
        //
        // The vector number is an argument to the dispatch
        // function, along with the address of the register save
        // area at the top of the stack.
        movq {vector_offset}(%rsp), %rdi;
        movzbq (%rdi), %rdi;
        movq %rdi, {vector_offset}(%rsp);
        movq %rsp, %rsi;
        // If we're already in kernel mode, don't swap %gs.
        cmpq ${ktext_sel}, {cs_offset}(%rsp);
        je 1f;
        swapgs;
        1:
        callq {dispatch};
        // If we're returning to kernel mode, don't swap %gs.
        cmpq ${ktext_sel}, {cs_offset}(%rsp);
        je 1f;
        swapgs;
        1:
        // Restore the general purpose registers.
        movq 0*8(%rsp), %rax;
        movq 1*8(%rsp), %rbx;
        movq 2*8(%rsp), %rcx;
        movq 3*8(%rsp), %rdx;
        movq 4*8(%rsp), %rsi;
        movq 5*8(%rsp), %rdi;
        movq 6*8(%rsp), %rbp;
        movq 7*8(%rsp), %r8;
        movq 8*8(%rsp), %r9;
        movq 9*8(%rsp), %r10;
        movq 10*8(%rsp), %r11;
        movq 11*8(%rsp), %r12;
        movq 12*8(%rsp), %r13;
        movq 13*8(%rsp), %r14;
        movq 14*8(%rsp), %r15;
        // Restore the segmentation registers.
        movw 15*8(%rsp), %ds;
        movw 16*8(%rsp), %es;
        // %gs is restored via swapgs above.  The system never changes
        // it, so we don't bother restoring it here.  %fs is special.
        // We do save and restore it, for TLS if anyone ever uses green
        // threads.
        movw 17*8(%rsp), %fs;
        // movw 18*8(%rsp), %gs;
        // Pop registers, alignment word and error.
        addq $((2 + 4 + 15) * 8), %rsp;
        // Go back to whence you came.
        iretq
        "#,
        ktext_sel = const 8,
        cs_offset = const TRAPFRAME_CS_OFFSET,
        vector_offset = const TRAPFRAME_VECTOR_OFFSET,
        dispatch = sym dispatch,
        options(att_syntax, noreturn));
}

fn dispatch(_vector: u8, _trap_frame: &mut Frame) -> u32 {
    0
}
