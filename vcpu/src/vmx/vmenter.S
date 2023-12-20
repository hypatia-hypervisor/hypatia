// Copyright 2023  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

.code64

.globl vmenter, vmexit
vmenter:
	// Save callee-save registers.
	subq $(6*8), %rsp
	movq %rbp, 0*8(%rsp)
	movq %rbx, 1*8(%rsp)
	movq %r12, 2*8(%rsp)
	movq %r13, 3*8(%rsp)
	movq %r14, 4*8(%rsp)
	movq %r15, 5*8(%rsp)

	// Save system stack pointer in the VM context
	// structure and reload the stack pointer to
	// point to the given context.  Since the stack
	// pointer is banked in the VMCS, it will be
	// automatically saved and restored.
	movq %rsp, 15*8(%rdi)
	movq %rdi, %rsp

	// Restore guest registers.
	movq 0*8(%rsp), %rax
	movq 1*8(%rsp), %rbx
	movq 2*8(%rsp), %rcx
	movq 3*8(%rsp), %rdx
	movq 4*8(%rsp), %rsi
	movq 5*8(%rsp), %rdi
	movq 6*8(%rsp), %rbp
	movq 7*8(%rsp), %r8
	movq 8*8(%rsp), %r9
	movq 9*8(%rsp), %r10
	movq 10*8(%rsp), %r11
	movq 11*8(%rsp), %r12
	movq 12*8(%rsp), %r13
	movq 13*8(%rsp), %r14
	movq 14*8(%rsp), %r15

	// Check whether we need to execute a `VMENTER` or
	// `VMRESUME` instruction depending on our current
	// state.
	cmpq $0, 16*8(%rsp)
	jne 1f
	vmlaunch
	jmp vmexit
1:	vmresume

vmexit:
	// Coming out of the guest context, the guest %rsp is
	// banked in the VMCS and the host %rsp value set to
	// point to the VM context before VM entry is restored.
	movq %rax, 0*8(%rsp)
	movq %rbx, 1*8(%rsp)
	movq %rcx, 2*8(%rsp)
	movq %rdx, 3*8(%rsp)
	movq %rsi, 4*8(%rsp)
	movq %rdi, 5*8(%rsp)
	movq %rbp, 6*8(%rsp)
	movq %r8, 7*8(%rsp)
	movq %r9, 8*8(%rsp)
	movq %r10, 9*8(%rsp)
	movq %r11, 10*8(%rsp)
	movq %r12, 11*8(%rsp)
	movq %r13, 12*8(%rsp)
	movq %r14, 13*8(%rsp)
	movq %r15, 14*8(%rsp)

	// Unconditionally set the state to 1 to represent that
	// we have entered the guest context for this VMCS;
	// subsequent vm entries should thus `VMRESUME`.
	movq	$1, 16*8(%rsp);

	// Restore system stack pointer.
	movq 15*8(%rsp), %rsp

	// Restore callee-save registers.
	movq 0*8(%rsp), %rbp
	movq 1*8(%rsp), %rbx
	movq 2*8(%rsp), %r12
	movq 3*8(%rsp), %r13
	movq 4*8(%rsp), %r14
	movq 5*8(%rsp), %r15

	// Restore the stack pointer and return.
	addq $(6*8), %rsp
	ret
