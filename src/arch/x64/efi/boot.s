.align 16
.global stack_end

.global entry
entry:
	# Load a new higher-half stack
	movabs $stack_end, %rsp
	movq %rcx, %rdi
	movabs $boot_entry, %rax
	jmp *%rax
