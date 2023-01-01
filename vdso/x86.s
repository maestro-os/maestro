.section .text

.global __kernel_vsyscall
.global __kernel_rt_sigreturn
.global __kernel_sigreturn
.global __vdso_clock_gettime
.global __vdso_gettimeofday
.global __vdso_time

__kernel_vsyscall:
	int $0x80
	ret

__kernel_rt_sigreturn:
	# TODO
	ud2

__kernel_sigreturn:
	# TODO
	ud2

__vdso_clock_gettime:
	# TODO
	ud2

__vdso_gettimeofday:
	# TODO
	ud2

__vdso_time:
	# TODO
	ud2
