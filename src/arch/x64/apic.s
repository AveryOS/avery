.intel_syntax noprefix

.global apic_registers
apic_registers:

.global apic_calibrate_ticks

.global apic_calibrate_pit_handler
apic_calibrate_pit_handler:
	push rax

	# Increase tick count
	#lock inc qword ptr [apic_calibrate_ticks]

	# EOI to APIC
	mov rax, qword ptr [apic_registers]
	mov dword ptr [rax + 0xB0], 0

	pop rax
	iretq
