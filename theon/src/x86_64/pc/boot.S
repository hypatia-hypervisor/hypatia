// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

STACKSIZE =		8 * 4096

Cr0PE =			1 << 0
Cr0MP =			1 << 1
Cr0TS =			1 << 3
Cr0WP =			1 << 16
Cr0NW =			1 << 29
Cr0CD =			1 << 30
Cr0PG =			1 << 31

Cr4PSE =		1 << 4
Cr4PAE =		1 << 5
Cr4FSGSBASE =		1 << 16

IA32_EFER_MSR =		0xc0000080
EferSCE =		1 << 0
EferLME =		1 << 8
EferNX =		1 << 11

GdtNULL =		0
GdtCODE64 =		1 * 8
GdtCODE32 =		2 * 8
GdtDATA32 =		3 * 8

SegREAD =		1 << 41
SegWRITE =		1 << 42
SegCODE =		1 << 43
SegDATA =		0 << 43
SegALWAYS1 =		1 << 44
SegPRESENT =		1 << 47
SegLONG =		1 << 53

Seg32DEFAULT =		1 << 54
Seg32GRANULARITY =	1 << 55
Seg32LIMIT =		(0xF << 48) + 0xFFFF
Seg32DEF =		Seg32DEFAULT + Seg32GRANULARITY + Seg32LIMIT

// Serial port for debugging in early boot
Uart0 =			0x3f8
Uart0IER =		Uart0 + 1
Uart0FCR =		Uart0 + 2
Uart0LCR =		Uart0 + 3
Uart0LSR =		Uart0 + 5
Uart0DivLS =		Uart0
Uart0DivMS =		Uart0 + 1

UartDLAB =		1 << 7
Uart8N1 =		(1 << 1) + (1 << 0)
UartFIFO =		(1 << 0) + (1 << 1) + (1 << 2) + (1 << 6) + (1 << 7)
UartDiv115200Lo =	1
UartDiv115200Hi =	0
UartTXRDY =		1 << 5

// Programming Interrupt Controller constants.  So we can
// turn it off.
Pic1 =			0x20
Pic1Data =		Pic1 + 1
Pic2 =			0xA0
Pic2Data =		Pic2 + 1

FlagsID =		1 << 21
CPUIDMaxLevel =		0x80000000
CPUIDMinRequired =	0x80000001
CPUIDFeatures =		0x80000001
FeatureLM =		1 << 29
FeatureRDTSCP =		1 << 27
FeaturePG1G =		1 << 26
FeatureNX =		1 << 20
FeatureX2APIC =		1 << 21
FeatureFSGSBase =	1 << 0

IA32_APIC_BASE_MSR =	0x1B
ApicBaseEnable =	1 << 11
ApicBaseX2Enable =	1 << 10
IA32_X2APIC_APICID_MSR = 0x802

KERNZERO =		0xFFFF800000000000

Multiboot1MAGIC =	0x1BADB002
Multiboot1LoadMAGIC =	0x2BADB002
Multiboot1FLAGS =	1 << 1	// Provide memory map.

.section .text.boot
.align 8
.globl multiboot1_header
multiboot1_header:
	.int	Multiboot1MAGIC
	.int	Multiboot1FLAGS
	.int	-(Multiboot1MAGIC + Multiboot1FLAGS)

.code32
.align 16
.globl startboot
startboot:
	// Disable interrupts, clear direction flag
	cli
	cld

	// Give ourselves a stack
	movl	$(bootstack - KERNZERO + STACKSIZE), %esp

	// Save multiboot parameters (in %ebx) on stack.
	// We'll restore the physical address of the multiboot
	// information structures (in %ebx) as a parameter to
	// `main()` when we get into 64-bit mode, so push a 0
	// for the high 32-bits when we pop.
	pushl	$0
	pushl	%ebx
	pushl	%eax

	// Disable the PIC by masking all of its interrupts.
	movb	$0xFF, %al
	outb	%al, $Pic1Data
	outb	%al, $Pic2Data

	// Initialize the UART for debugging:
	// * 115200bps
	// * 8N1
	// * FIFOs enabled and reset
	// * interrupts disabled
	movw	$Uart0IER, %dx
	xorb	%al, %al
	outb	%al, %dx		// Disable interrupts.
	movw	$Uart0LCR, %dx
	movb	$UartDLAB, %al
	outb	%al, %dx		// Set latch to access divisor registers
	movw	$Uart0DivLS, %dx
	movb	$UartDiv115200Lo, %al
	outb	%al, %dx
	movw	$Uart0DivMS, %dx
	movb	$UartDiv115200Hi, %al	// Divisor is 1 => 115,200 bps.
	outb	%al, %dx
	movw	$Uart0LCR, %dx
	movb	$Uart8N1, %al
	outb	%al, %dx		// Char length 8, no parity, 1 stop bit, clear DLAB
	movw	$Uart0FCR, %dx
	movb	$UartFIFO, %al
	outb	%al, %dx		// Reset and enable FIFOs and triggers

	// Check the state of the system to see whether it can
	// support us.  We assume that if the BSP supports all
	// the required functionality, the rest of the logical
	// processors in the system do as well.
	//
	// Check multiboot magic
	popl	%eax
	cmpl	$Multiboot1LoadMAGIC, %eax
	je	1f
	movl	$(EBadMagic - KERNZERO), %edi
	jmp	earlypanic
1:
	// Check whether we support CPUID
	pushfd
	popl	%eax
	movl	%eax, %ecx
	xorl	$FlagsID, %eax
	pushl	%eax
	popfd				// Disable ID in FLAGS.
	pushfd
	popl	%eax			// If CPUID is support, %eax != %ecx
	pushl	%ecx
	popfd				// Restore original FLAGS.
	cmpl	%eax, %ecx
	jne	1f
	movl	$(ENoCPUID - KERNZERO), %edi
	jmp	earlypanic
1:
	// Check extended CPUID level support
	movl	$CPUIDMaxLevel, %eax
	xorl	%ecx, %ecx
	cpuid
	cmpl	$CPUIDMinRequired, %eax
	jge	1f
	movl	$(EOldCPUID - KERNZERO), %edi
	jmp	earlypanic
1:
	// Check CPU features
	movl	$0x80000001, %eax
	xorl	%ecx, %ecx
	cpuid
	// Check long mode
	testl	$FeatureLM, %edx
	jnz	1f
	movl	$(ENo64Bit - KERNZERO), %edi
	jmp	earlypanic
1:	// Test TSC support
	testl	$FeatureRDTSCP, %edx
	jnz	1f
	movl	$(ENoRDTSCP - KERNZERO), %edi
	jmp	earlypanic
1:	// Check gigabyte pages
	testl	$FeaturePG1G, %edx
	jnz	1f
	movl	$(ENoGigPages - KERNZERO), %edi
	jmp	earlypanic
1:	// Check NX page support
	testl	$FeatureNX, %edx
	jnz	1f
	movl	$(ENoNX - KERNZERO), %edi
	jmp	earlypanic
1:
	movl	$0x00000001, %eax
	xorl	%ecx, %ecx
	cpuid
	// Check x2APIC
	testl	$FeatureX2APIC, %ecx
	jnz	1f
	movl	$(ENoX2APIC - KERNZERO), %edi
	jmp	earlypanic
1:
	movl	$0x00000007, %eax
	xorl	%ecx, %ecx
	cpuid
	// Check (RD|WR)(FS|GS)BASE
	testl	$FeatureFSGSBase, %ebx
	jnz	1f
	movl	$(ENoFSGSBase - KERNZERO), %edi
	jmp	earlypanic
1:
	// If we've gotten this far, the system supports the
	// features that we need to run.  There's probably more
	// we should check about the hardware.

	// Check whether the GDT is in the first 16MiB of
	// physical address space for later AP startup.
	movl	$(gdt - KERNZERO), %edx
	cmpl	$((16 * (1 << 20)) - 32), %edx
	jle	1f
	movl	$(EGDTTooFarAway - KERNZERO), %edi
	jmp	earlypanic
1:
	// Enable long mode, no-execute support, and SCE
	movl	$IA32_EFER_MSR, %ecx
	movl	$(EferSCE | EferLME | EferNX), %eax
	xorl	%edx, %edx
	wrmsr

	// Set physical address extension, set FSGSBase.
	movl	%cr4, %eax
	orl	$(Cr4PAE | Cr4FSGSBASE), %eax
	andl	$~Cr4PSE, %eax
	movl	%eax, %cr4

	// Put our early double mapping in place for the jump to
	// high addresses.
	movl	$(bootpgtbl - KERNZERO), %eax
	movl	%eax, %cr3

	// Enable protected mode and paging.
	movl	%cr0, %eax
	orl	$(Cr0PE | Cr0PG | Cr0WP), %eax
	andl	$~(Cr0MP | Cr0TS | Cr0NW | Cr0CD), %eax
	movl	%eax, %cr0

	// Load the low-mapped GDT with the long code segment
	// and jump to 64-bit mode.
	lgdt	(gdtdesc - KERNZERO)
	ljmpl	$GdtCODE64, $(boot64 - KERNZERO)

// Early panic
earlypanic:
	pushl	%eax
	pushl	%ecx
	pushl	%edx
	pushl	%edi

1:	movb	(%edi), %ah
	cmpb	$0, %ah
	je	4f

	movl	$65535, %ecx
	movw	$Uart0LSR, %dx
2:	inb	%dx, %al
	testb	$UartTXRDY, %al
	jnz	3f
	pause
	loop	2b
3:
	movb	%ah, %al
	movw	$Uart0, %dx
	outb	%al, %dx

	incl	%edi
	jmp	1b
4:
	popl	%edi
	popl	%edx
	popl	%ecx
	popl	%eax
5:
	cli
	hlt
	jmp	5b

EBadMagic:	.asciz "\"earlypanic\": \"Bad Multiboot magic\""
ENoCPUID:	.asciz "\"earlypanic\": \"CPUID instruction not supported\""
EOldCPUID:	.asciz "\"earlypanic\": \"CPU instruction too old\""
ENo64Bit:	.asciz "\"earlypanic\": \"No 64-bit long mode support\""
ENoRDTSCP:	.asciz "\"earlypanic\": \"No RDTSCP support\""
ENoGigPages:	.asciz "\"earlypanic\": \"No gigabyte page support\""
ENoNX:		.asciz "\"earlypanic\": \"No non-executable page support\""
ENoX2APIC:	.asciz "\"earlypanic\": \"No x2APIC mode support\""
ENoFSGSBase:	.asciz "\"earlypanic\": \"No (RD|WR)(FS|GS)BASE support\""
EGDTTooFarAway:	.asciz "\"earlypanic\": \"GDT Descriptor is beyond 16MiB\""

.code64
.align 64
boot64:
	// Load the virtually-mapped GDT.
	movabsq	$gdtdescv, %rax
	lgdt	(%rax)

	// Zero out the segmentation registers; we won't touch
	// them from here on out.
	xorl	%eax, %eax
	movw	%ax, %ds
	movw	%ax, %es
	movw	%ax, %fs
	movw	%ax, %gs
	movw	%ax, %ss

	// Multiboot parameters are an argument to `main`.
	popq	%rdi

	// Load the %rsp with the linked address of our stack.
	movabsq	$(bootstack + STACKSIZE), %rsp

	// Transition to running at linked addresses.
	movabsq	$1f, %rax
	pushq	%rax
	ret
1:
	// We are now accessing everything relative to our
	// linked addresses.  We can remove the identity mapping
	// in low virtual memory.  We do this by reloading the
	// page table root register to point to the _early page
	// table_, which is the same as boot page table, but
	// without the identity map.  We leave the boot page
	// table as it is, as it is used to boot the APs.
	movq	$(earlypgtbl - KERNZERO), %rax
	movq	%rax, %cr3

	// Call `main` in Rust code.
	pushq	$0
	movq	$0, %rbp
	leaq	main(%rip), %rax
	pushq	%rax
	xorl	%eax, %eax
	ret
	ud2

.align 4096
.rodata

// AP startup.  APs begin in 16-bit real mode, executing at the
// beginning of the page with the same number as the SIPI vector.
// The A20 latch is already pre-disabled.

APENTRY =		0x7000
APCPUS =		0x8000 - 8
APNCPUS =		0x8000 - 16
StateRUNNING =		1

.code16
.globl apstart, eapstart
apstart:
	cli
	cld
	wbinvd

	lgdt	(APENTRY + (gdtdesc - apstart))

	movl	%cr0, %eax
	orl	$Cr0PE, %eax
	movl	%eax, %cr0

	ljmpl	$GdtCODE32, $(1f - KERNZERO)

.align 16
.skip 6
gdtdesc:
	.word	egdt - gdt - 1
	.long	(gdt - KERNZERO)

eapstart:

.code32
1:
	movw	$GdtDATA32, %ax
	movw	%ax, %ds
	movw	%ax, %ss
	movw	%ax, %es
	xorw	%ax, %ax
	movw	%ax, %fs
	movw	%ax, %gs

	movl	$IA32_EFER_MSR, %ecx
	movl	$(EferSCE | EferLME | EferNX), %eax
	xorl	%edx, %edx
	wrmsr

	movl	%cr4, %eax
	orl	$(Cr4PAE | Cr4FSGSBASE), %eax
	andl	$~Cr4PSE, %eax
	movl	%eax, %cr4

	movl	$(bootpgtbl - KERNZERO), %eax
	movl	%eax, %cr3

	mov	%cr0, %eax
	orl	$(Cr0PE | Cr0PG | Cr0WP), %eax
	andl	$~(Cr0MP | Cr0TS | Cr0NW | Cr0CD), %eax
	movl	%eax, %cr0
	ljmp	$GdtCODE64, $(1f - KERNZERO)

.code64
1:
	// Load the high-mapped GDT.
	movabsq	$gdtdescv, %rax
	lgdt	(%rax)

	xorw	%ax, %ax
	movw	%ax, %ds
	movw	%ax, %es
	movw	%ax, %fs
	movw	%ax, %gs
	movw	%ax, %ss

	// We need our x2APIC ID to proceed
	movl	$IA32_APIC_BASE_MSR, %ecx
	rdmsr
	testl	$ApicBaseEnable, %eax
	jnz	2f
	lea	EXApicDisabled(%rip), %rdi
	jmp	apearlypanic
2:
	// Turn on x2APIC mode
	xorl	%edx, %edx
	orl	$ApicBaseX2Enable, %eax
	wrmsr
	// Retrieve the x2APIC ID; stash in %rax.
	movl	$IA32_X2APIC_APICID_MSR, %ecx
	rdmsr

	// We now iterate through the list of APIC IDs
	// to find our CPU *number* and mark our state
	// as running.
	movq	APCPUS, %rsi
	movq	APNCPUS, %r8
	xorl	%ecx, %ecx
1:
	cmp	%rcx, %r8
	jne	2f
	leaq	ECpuNotFound(%rip), %rdi
	jmp	apearlypanic
2:
	movl	(%rsi), %edi
	cmp	%eax, %edi
	je	3f
	incq	%rcx
	addq	$16, %rsi
	jmp	1b
3:
	// Load our per-CPU stack from the EntryCPU structure
	// and mark state as running.  Stores within a cache
	// line are atomic on x86, and we assume the store to
	// EarlyCPU.state is aligned and thus atomic.
	movq	8(%rsi), %rsp
	movl	$StateRUNNING, 4(%rsi)

	// Processor number is the argument to apmain
	movq	%rcx, %rdi

	// Jump to high, linked addresses.
	movabsq	$1f, %rax
	pushq	%rax
	ret
	ud2
1:
	// Call apmain in Rust code.
	pushq	$0
	movq	$0, %rbp
	leaq	apmain(%rip), %rax
	push	%rax
	ret
	ud2

apearlypanic:
	// TODO(cross): We really ought to write something to
	// the UART here, but it clobbers too many registers in
	// a state where we don't have a stack.  However, %rdi
	// should at least contain a pointer to an error string
	// that we can look at with a debugger or monitor to
	// figure out what happened.  Halt in a loop, in case a
	// stray SIPI or something similar restarts us.
1:	cli
	hlt
	jmp	1b

EXApicDisabled:	.asciz "\"earlypanic\": \"LAPIC disabled\""
ECpuNotFound:	.asciz "\"earlypanic\": \"Secondary processor not in CPU list\""

.align 16
gdt:
	// 0: Null segment
	.quad	0
	// 8: Kernel 64-bit code segment
	.quad	(SegREAD | SegCODE | SegALWAYS1 | SegPRESENT | SegLONG)
	// 16: Kernel 32-bit code segment (for bootstrapping APs)
	.quad	(SegREAD | SegCODE | SegALWAYS1 | SegPRESENT | Seg32DEF)
	// 24: Kernel 32-bit data segment (for bootstrapping APs)
	.quad	(SegREAD | SegWRITE | SegALWAYS1 | SegPRESENT | Seg32DEF)
egdt:

.align 16
.skip 6
gdtdescv:
	.word	egdt - gdt - 1
	.quad	gdt

PtX =		0 << 63
PtNX =		1 << 63
PtHUGE =	1 << 7
PtW =		1 << 1
PtR =		1 << 0

.align 4096
bootpgtbl:
	.quad	(earlypt3 - KERNZERO) + (PtX | PtW | PtR)
	.space	(4096/2) - (1*8)
	.quad	(earlypt3 - KERNZERO) + (PtX | PtW | PtR)
	.space	(4096/2) - (1*8)

earlypgtbl:
	.space	(4096/2)
	.quad	(earlypt3 - KERNZERO) + (PtX | PtW | PtR)
	.space	(4096/2) - (2*8)
	.quad	(earlypgtbl - KERNZERO) + (PtNX | PtW | PtR)

.align 4096
earlypt3:
	.quad	(0<<30) + (PtHUGE | PtX | PtW | PtR)
	.quad	(1<<30) + (PtHUGE | PtX | PtW | PtR)
	.quad	(2<<30) + (PtHUGE | PtX | PtW | PtR)
	.quad	(3<<30) + (PtHUGE | PtX | PtW | PtR)
	.space	4096 - (4*8)

.bss
.align 4096

// Stack.
.globl bootstack
bootstack:
	.space	STACKSIZE
