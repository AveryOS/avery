.align 16
.global stack
.comm stack, 0x8000

.global entry
entry:
	# Load a new higher-half stack
	movabs $(stack + 0x8000), %rsp
	movq %rcx, %rdi
	movabs $boot_entry, %rax
	jmp *%rax
