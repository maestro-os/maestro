#include "ps2.h"

// TODO Timeout
__attribute__((hot))
static uint8_t ps2_command(const uint8_t command,
	const uint8_t expected_response)
{
	uint8_t response;
	uint8_t attempts = 0;

	while(attempts++ < PS2_MAX_ATTEMPTS)
	{
		outb(PS2_COMMAND, command);

		while(!(inb(PS2_STATUS) & 0b1))
		{
			// TODO Sleep?
		}

		if((response = inb(PS2_DATA)) == expected_response)
			break;
	}

	return response;
}

__attribute__((hot))
__attribute__((const))
static inline bool test_controller()
{
	return (ps2_command(0xaa, CONTROLLER_TEST_PASS) == CONTROLLER_TEST_PASS
		&& inb(PS2_DATA) == 0x55);
}

__attribute__((hot))
__attribute__((const))
static inline bool test_device()
{
	return (ps2_command(0xab, KEYBOARD_TEST_PASS) == KEYBOARD_TEST_PASS
		&& inb(PS2_DATA) == 0x00);
}

__attribute__((hot))
__attribute__((const))
void ps2_disable_devices()
{
	outb(PS2_COMMAND, 0xad);
	outb(PS2_COMMAND, 0xa7);
}

__attribute__((hot))
__attribute__((const))
bool ps2_enable_keyboard(void)
{
	outb(PS2_COMMAND, 0xae);
	// TODO?

	return true;
}

__attribute__((hot))
__attribute__((const))
static inline uint8_t get_config_byte()
{
	outb(PS2_COMMAND, 0x20); // TODO Use ps2_command without expected response?
	return inb(PS2_DATA);
}

__attribute__((hot))
__attribute__((const))
static inline void set_config_byte(const uint8_t config_byte)
{
	outb(PS2_COMMAND, 0x60);
	outb(PS2_DATA, config_byte); // TODO Check if can write before
}

__attribute__((cold))
static void in_ps2_init()
{
	// TODO Check if existing using ACPI
	ps2_disable_devices();
	inb(PS2_DATA);

	// TODO Correct?
	set_config_byte(get_config_byte() & 0b10111100);
	printf("PS/2, Dual Channel: %s\n",
		(get_config_byte() & 0b10000 ? "no" : "yes"));

	if(!test_controller())
	{
		printf("PS/2 controller: KO D:\n");
		return;
	}

	if(!ps2_enable_keyboard())
	{
		printf("Failed to enable keyboard!\n");
		return;
	}

	if(!test_device())
	{
		printf("PS/2 first device: KO D:\n");
		return;
	}
}

void ps2_init()
{
	idt_setup_wrap(in_ps2_init);
}
