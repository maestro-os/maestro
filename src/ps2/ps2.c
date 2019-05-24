#include "ps2.h"

__attribute__((hot))
__attribute__((const))
static inline bool can_read(void)
{
	return inb(PS2_STATUS) & 0b1;
}

__attribute__((hot))
__attribute__((const))
static inline void wait_read(void)
{
	while(!can_read())
	{
		// TODO Sleep?
	}
}

__attribute__((hot))
__attribute__((const))
static inline bool can_write(void)
{
	return !(inb(PS2_STATUS) & 0b10);
}

__attribute__((hot))
__attribute__((const))
static inline void wait_write(void)
{
	while(!can_write())
	{
		// TODO Sleep?
	}
}

// TODO Timeout
__attribute__((hot))
static uint8_t ps2_command(const uint8_t command,
	const uint8_t expected_response)
{
	uint8_t response;
	uint8_t attempts = 0;

	while(attempts++ < PS2_MAX_ATTEMPTS)
	{
		wait_write();
		outb(PS2_COMMAND, command);

		wait_read();
		if((response = inb(PS2_DATA)) == expected_response)
			break;
	}

	return response;
}

__attribute__((hot))
__attribute__((const))
static inline bool test_controller(void)
{
	return (ps2_command(0xaa, CONTROLLER_TEST_PASS) == CONTROLLER_TEST_PASS);
}

__attribute__((hot))
__attribute__((const))
static inline bool test_device(void)
{
	return (ps2_command(0xab, KEYBOARD_TEST_PASS) == KEYBOARD_TEST_PASS);
}

// TODO Timeout
__attribute__((hot))
__attribute__((const))
static inline bool keyboard_send(const uint8_t data)
{
	uint8_t response;
	uint8_t attempts = 0;

	while(attempts++ < PS2_MAX_ATTEMPTS)
	{
		wait_write();
		outb(PS2_DATA, data);

		wait_read();
		if((response = inb(PS2_DATA)) == KEYBOARD_RESEND)
			break;
	}

	return (response == KEYBOARD_RESEND);
}

__attribute__((hot))
__attribute__((const))
void ps2_disable_devices(void)
{
	wait_write();
	outb(PS2_COMMAND, 0xad);
	wait_write();
	outb(PS2_COMMAND, 0xa7);
}

__attribute__((hot))
__attribute__((const))
bool ps2_enable_keyboard(void)
{
	wait_write();
	outb(PS2_COMMAND, 0xae);

	// TODO LEDs

	//if(!keyboard_send(0xf0) || !keyboard_send(2))
	//	return false;

	//if(!keyboard_send(0xf3) || !keyboard_send(0))
	//	return false;

	//if(!keyboard_send(0xf4))
	//	return false;

	return true;
}

__attribute__((hot))
__attribute__((const))
static inline uint8_t get_config_byte(void)
{
	wait_write();
	outb(PS2_COMMAND, 0x20); // TODO Use ps2_command without expected response?

	wait_read();
	return inb(PS2_DATA);
}

__attribute__((hot))
__attribute__((const))
static inline void set_config_byte(const uint8_t config_byte)
{
	wait_write();
	outb(PS2_COMMAND, 0x60);
	wait_write();
	outb(PS2_DATA, config_byte); // TODO Check if can write before
}

__attribute__((cold))
static void in_ps2_init(void)
{
	// TODO Check if existing using ACPI
	ps2_disable_devices();
	inb(PS2_DATA);

	set_config_byte(get_config_byte() & 0b10111100);
	printf("PS/2 Dual Channel: %s\n",
		((get_config_byte() & 0b100000) ? "no" : "yes"));

	if(!test_controller())
	{
		printf("PS/2 controller: KO D:\n");
		return;
	}

	if(!test_device())
	{
		printf("PS/2 first device: KO D:\n");
		return;
	}

	if(!ps2_enable_keyboard())
	{
		printf("Failed to enable keyboard!\n");
		return;
	}

	set_config_byte(get_config_byte() | 0b1);
}

void ps2_init(void)
{
	idt_setup_wrap(in_ps2_init);
}
