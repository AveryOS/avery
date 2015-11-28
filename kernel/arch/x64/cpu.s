.intel_syntax noprefix
.code16

.global ap_bootstrap
ap_bootstrap:
	cli
	jmp 0:reset_cs
reset_cs:

	xor eax, eax
	mov ds, ax

	lidt [idt]

spin:
	cmp dword ptr [allow_start], 0 # should be `qword ptr` binutils bugs
	jne enter_long_mode
	pause
	jmp spin

enter_long_mode:
	# Enable PAE
	mov eax, cr4
	bts eax, 5
	mov cr4, eax

	# Setup PML4
	mov eax, dword ptr [pml4]
	mov cr3, eax

	# Set long mode and NX enable bits
	mov ecx, 0xC0000080
	rdmsr
	bts eax, 8
	bts eax, 11
	wrmsr

	# Turn on paging and protected mode
	mov eax, cr0
	bts eax, 0
	bts eax, 31
	mov cr0, eax

	lgdt [gdt_pointer]

	jmp 8:long_mode

.code64
.global ap_entry

long_mode:
	mov ax, 0x10
	mov ds, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	mov ss, ax

	mov rax, [apic_regs]
	mov eax, [rax + 0x20]
	shr eax, 24

	mov rdx, [cpu_apic_offset]
	mov rbx, [cpus]

find_cpu:
	cmp [rbx + rdx], al
	je found_cpu
	add rbx, [cpu_size]
	jmp find_cpu

found_cpu:
	mov rdx, [cpu_stack_offset]
	mov rsp, [rbx + rdx]
	mov rdi, rbx

	jmp ap_entry

	#movabs rax, ap_entry binutils doesn't like this
	#jmp rax


gdt:
.quad 0
.quad 0x0020980000000000
.quad 0x0000920000000000

gdt_pointer:
.word 23
.quad gdt

idt:
.word 0
.byte 0

.global ap_bootstrap_info
ap_bootstrap_info:
pml4:
.long 0
allow_start:
.quad 0
apic_regs:
.quad 0
cpu_count:
.quad 0
cpu_size:
.quad 0
cpu_apic_offset:
.quad 0
cpu_stack_offset:
.quad 0
cpus:
.quad 0
