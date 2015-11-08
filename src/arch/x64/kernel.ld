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
		build/src/arch/x64/multiboot/bootstrap.s.o
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
		build/src/arch/x64/cpu.S.o(*)
	}

	. = ap_bootstrap_start + (. - ap_bootstrap_mapped);

	ap_bootstrap_end = .;

	. = ALIGN(page_size);

	.text : AT(ADDR(.text) - high_offset)
	{
		*(.text*)
		*(.gnu.linkonce.t*)
	}

	. = ALIGN(page_size);
	rodata_start = .;

	.rodata : AT(ADDR(.rodata) - high_offset)
	{
		*(.rodata*)
		*(.gnu.linkonce.r*)

		raw_bitmap_font = .;
		*(.font)
	}

	debug_ranges_start = .;
	.debug_ranges 0 : AT(debug_ranges_start - high_offset) { *(.debug_ranges) }
	. += debug_ranges_start;
	debug_ranges_end = .;

	debug_abbrev_start = .;
	.debug_abbrev 0 : AT(debug_abbrev_start - high_offset) { *(.debug_abbrev) }
	. += debug_abbrev_start;
	debug_abbrev_end = .;

	debug_info_start = .;
	.debug_info 0 : AT(debug_info_start - high_offset) { *(.debug_info) }
	. += debug_info_start;
	debug_info_end = .;

	debug_line_start = .;
	.debug_line 0 : AT(debug_line_start - high_offset) { *(.debug_line) }
	. += debug_line_start;
	debug_line_end = .;

	debug_str_start = .;
	.debug_str 0 : AT(debug_str_start - high_offset) { *(.debug_str) }
 	. += debug_str_start;
	debug_str_end = .;

	. = ALIGN(page_size);
	data_start = .;

	.data : AT(ADDR(.data) - high_offset)
	{
		*(.data*)
		*(.gnu.linkonce.d*)
	}

	.bss ALIGN(page_size) : AT(ADDR(.bss) - high_offset)
	{
		*(.bss*)
		*(.gnu.linkonce.b*)
	}

	. = ALIGN(page_size);
	kernel_end = .;
	kernel_size = (kernel_end - kernel_start) / page_size;

	/DISCARD/ : {
		*(*)
	}
}
