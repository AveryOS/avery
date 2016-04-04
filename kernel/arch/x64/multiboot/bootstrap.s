#.intel_syntax noprefix
.code32
.align 16
.fill 0x4000
stack.32:

.align 0x1000
.global pdpt_low
pdpt_low:
.fill 0x1000

.global pdt_low
pdt_low:
.fill 0x1000

.global pt_low
pt_low:
.fill 0x1000

.global pml4t
pml4t:
.fill 0x1000

.global pdpt
pdpt:
.fill 0x1000

.global pdt
pdt:
.fill 0x1000

.global pts
pts:
.fill 0x10000

.global entry
entry:
	cli
	movl $(stack.32), %esp
	pushl %eax
	pushl %ebx
	call setup_long_mode
loop.32:
	hlt
	jmp loop.32

.code64

.global stack_end

.global bootstrap.64
bootstrap.64:
	# Load data segments
	movw $0x10, %ax
	movw %ax, %ds
	movw %ax, %es
	movw %ax, %fs
	movw %ax, %gs
	movw %ax, %ss

	# Load a new higher-half stack
	movabs $(stack_end), %rsp

	# Clear rbp for backtraces
	xor %rbp, %rbp

	# Call the higher-half entry
	mov %rcx, %rdi
	movabs $boot_entry, %rax
	call *%rax

loop.64:
	hlt
	jmp loop.64
