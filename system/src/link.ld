/*
 * Copyright 2021  The Hypatia Authors
 * All rights reserved
 *
 * Use of this source code is governed by an MIT-style
 * license that can be found in the LICENSE file or at
 * https://opensource.org/licenses/MIT.
 */

ENTRY(start)

SECTIONS {
	. = 0xFFFFF00000000000;

	.text . :
	{
		*(.text*)
	}
	. = ALIGN(4096);
	PROVIDE(etext = .);

	.rodata . :
	{
		*(.rodata*)
	}
	. = ALIGN(4096);
	PROVIDE(erodata = .);

	.got . :
	{
		*(.got*)
	}
	. = ALIGN(4096);

	.data . :
	{
		*(.data*)
	}
	. = ALIGN(4096);
	PROVIDE(edata = .);

	.bss . :
	{
		*(.bss*)
		*(COMMON)
	}
	. = ALIGN(4096);
	PROVIDE(end = .);
}
