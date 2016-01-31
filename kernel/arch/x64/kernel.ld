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

% ['ranges', 'abbrev', 'info', 'line', 'str', 'loc'].each do |s|
	debug_#{s}_start = .;
	.debug_#{s} 0 : AT(debug_#{s}_start - high_offset) { *(.debug_#{s}) }
	. += debug_#{s}_start;
	debug_#{s}_end = .;
% end

	. = ALIGN(page_size);

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
	}

	. = ALIGN(page_size);
	kernel_end = .;
	kernel_size = (kernel_end - kernel_start) / page_size;

	/DISCARD/ : {
		*(*)
	}
}
