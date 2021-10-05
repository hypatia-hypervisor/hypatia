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
    // and rxv64 doesn't support real or compatibility
    // mode, so these are effectively unused.
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

const TRAPFRAME_VECTOR_OFFSET: usize = 0x98;
const TRAPFRAME_CS_OFFSET: usize = 0xB0;

macro_rules! gen_stub {
    ($vecnum:expr) => {
        concat!(r#".balign 8; pushq $0; callq {trap}; .byte "#, stringify!($vecnum), ";")
    };
    ($vecnum:expr, err) => {
        concat!(r#".balign 8; callq {trap}; .byte "#, stringify!($vecnum), ";")
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
    ($num:expr) => {
        gen_stub!($num)
    };
}

#[allow(dead_code)]
#[link_section = ".trap"]
#[naked]
pub unsafe extern "C" fn stubs() -> ! {
    asm!(
        seq!(N in 0..=255 {
            concat!( #( gen_trap_stub!(N), )* )
        }),
        trap = sym trap, options(att_syntax, noreturn));
}

#[link_section = ".trap"]
#[naked]
pub unsafe extern "C" fn trap() -> ! {
    asm!(r#"
        // Save the x86 segmentation registers.
        subq $32, %rsp
        movq $0, (%rsp);
        movw %ds, (%rsp);
        movq $0, 8(%rsp);
        movw %es, 8(%rsp);
        movq $0, 16(%rsp);
        movw %fs, 16(%rsp);
        movq $0, 24(%rsp);
        movw %gs, 24(%rsp);
        pushq %r15;
        pushq %r14;
        pushq %r13;
        pushq %r12;
        pushq %r11;
        pushq %r10;
        pushq %r9;
        pushq %r8;
        pushq %rbp;
        pushq %rdi;
        pushq %rsi;
        pushq %rdx;
        pushq %rcx;
        pushq %rbx;
        pushq %rax;
        movq {vector_offset}(%rsp), %rdi;
	movzbq (%rdi), %rdi;
	movq %rdi, {vector_offset}(%rsp);
        movq %rsp, %rsi;
        cmpq ${ktext_sel}, {cs_offset}(%rsp);
        je 1f;
        swapgs;
        1:
        callq {dispatch};
        cmpq ${ktext_sel}, {cs_offset}(%rsp);
        je 1f;
        swapgs;
        1:
        popq %rax;
        popq %rbx;
        popq %rcx;
        popq %rdx;
        popq %rsi;
        popq %rdi;
        popq %rbp;
        popq %r8;
        popq %r9;
        popq %r10;
        popq %r11;
        popq %r12;
        popq %r13;
        popq %r14;
        popq %r15;
        // If necessary, %gs is restored via swapgs above.
        // %fs is special.  We ought to save it and restore
        // it, should userspace ever use green threads.
        //movw 24(%rsp), %gs;
        movw 16(%rsp), %fs;
        movw 8(%rsp), %es;
        movw (%rsp), %ds;
        addq $32, %rsp;
        // Pop alignment word and error.
        addq $16, %rsp;
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
