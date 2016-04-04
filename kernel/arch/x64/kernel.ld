OUTPUT_FORMAT(elf64-x86-64)
ENTRY(entry)

high_start = 0xFFFFFFFF80000000;
page_size = 0x1000;

SECTIONS
{
% if multiboot
	. = 0;

	.multiboot ALIGN(4) : { *(.multiboot) }

	. = 0x100000;

	.bootstrap ALIGN(4) :
	{
		build/kernel/arch/x64/multiboot/bootstrap.s.o
		build/bootstrap.o(*)
	}
% else
	. = 0x100000;
% end

	. = ALIGN(page_size);
	low_end = .;
	high_offset = high_start - low_end;

	. = high_start;

	kernel_start = .;

	ap_bootstrap_start = .;
	ap_bootstrap_mapped = 0x1000;

	.ap_bootstrap ap_bootstrap_mapped : AT(ap_bootstrap_start - high_offset)
	{
		build/kernel/arch/x64/cpu.s.o(*)
		. = ALIGN(page_size); /* Align .text */
	}

	. = ap_bootstrap_start + (. - ap_bootstrap_mapped);

	ap_bootstrap_end = .;

	.text : AT(ADDR(.text) - high_offset)
	{
		*(.text*)
		*(.gnu.linkonce.t*)

		. = ALIGN(page_size); /* Align .rodata */
	}

	rodata_start = .;

	.rodata : AT(ADDR(.rodata) - high_offset)
	{
		*(.rodata*)
		*(.gnu.linkonce.r*)

		raw_bitmap_font = .;
		*(.font)

		user_image_start = .;
		build/user/hello.o(*)
		user_image_end = .;

		. = ALIGN(page_size); /* Align .data */
	}

	.data : AT(ADDR(.data) - high_offset)
	{
		data_start = .;
		*(.data*)
		*(.gnu.linkonce.d*)
	}

	.bss ALIGN(page_size) : AT(ADDR(.bss) - high_offset)
	{
		*(.bss*)
		*(.gnu.linkonce.b*)
		. = ALIGN(page_size); /* Align kernel_end */
	}

	kernel_end = .;
	kernel_size = (kernel_end - kernel_start) / page_size;
}
