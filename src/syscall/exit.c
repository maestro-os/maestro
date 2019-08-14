#include <syscall/syscall.h>
#include <idt/idt.h>
#include <pic/pic.h>

__attribute__((noreturn))
sys_ret_t sys_exit(const sys_info_t *info)
{
	process_exit(get_running_process(), info->ebx);
	pic_EOI(0x80);
	STI();
	asm("int $0x20");
	while(1)
		;
}
