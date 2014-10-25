.intel_syntax noprefix
.global spurious_irq
spurious_irq:
	iretq

.global interrupt_handlers
interrupt_handlers:
.fill 256 * 8

.global isr_handler
isr_handler:
	push rax
	push rbx
	push rbp
	push rcx
	push rdi
	push r8
	push r9
	push r10
	push r11
	push r12
	push r13
	push r14
	push r15

	mov ax, ds
	push rax

	mov ax, 0x10
	mov ds, ax
	
	mov rdi, rsp
	lea rcx, interrupt_handlers
	call [rcx + 8 * rsi]
	
	pop rax
	mov ds, ax

	pop r15
	pop r14
	pop r13
	pop r12
	pop r11
	pop r10
	pop r9
	pop r8
	pop rdi
	pop rcx
	pop rbp
	pop rbx
	pop rax

	pop rsi
	pop rdx

	iretq

.global isr_stubs
isr_stubs:
% 255.times do |n|
.quad isr_#{n}
% end

% 255.times do |n|
isr_#{n}:
	% if [8, 10, 11, 12, 13, 14, 17].include?(n)
		xchg [rsp], rdx
	% else
		push rdx
	% end
	push rsi
	mov rsi, #{n}
	jmp isr_handler
% end
