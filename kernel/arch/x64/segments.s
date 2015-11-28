.global load_segments
load_segments:
	pushq %rsi
	pushq $new_code
	lretq
	
new_code:
	movw %di, %ax
	movw %ax, %ss
	movw %ax, %ds
	movw %ax, %es
	movw %ax, %fs
	movw %ax, %gs

	ret
