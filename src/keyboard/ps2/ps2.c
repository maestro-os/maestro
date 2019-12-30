#include <keyboard/ps2/ps2.h>

static void (*keyboard_hook)(const uint8_t) = NULL;
static int8_t leds_state = 0;

ATTR_HOT
static inline int can_read(void)
{
	return inb(PS2_STATUS) & 0b1;
}

ATTR_HOT
static inline void wait_read(void)
{
	while(!can_read())
	{
		// TODO Sleep?
	}
}

ATTR_HOT
static inline int can_write(void)
{
	return !(inb(PS2_STATUS) & 0b10);
}

ATTR_HOT
static inline void wait_write(void)
{
	while(!can_write())
	{
		// TODO Sleep?
	}
}

// TODO Timeout
ATTR_HOT
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

ATTR_HOT
static inline int test_controller(void)
{
	return (ps2_command(0xaa, CONTROLLER_TEST_PASS) == CONTROLLER_TEST_PASS);
}

ATTR_HOT
static inline int test_device(void)
{
	return (ps2_command(0xab, KEYBOARD_TEST_PASS) == KEYBOARD_TEST_PASS);
}

// TODO Timeout
ATTR_HOT
static inline int keyboard_send(const uint8_t data)
{
	uint8_t response;
	uint8_t attempts = 0;

	while(attempts++ < PS2_MAX_ATTEMPTS)
	{
		wait_write();
		outb(PS2_DATA, data);

		wait_read();
		if((response = inb(PS2_DATA)) == KEYBOARD_ACK)
			break;
	}

	return (response == KEYBOARD_ACK);
}

ATTR_HOT
static void clear_buffer(void)
{
	while(can_read())
		inb(PS2_DATA);
}

ATTR_HOT
void ps2_disable_devices(void)
{
	wait_write();
	outb(PS2_COMMAND, 0xad);
	wait_write();
	outb(PS2_COMMAND, 0xa7);
}

ATTR_HOT
int ps2_enable_keyboard(void)
{
	wait_write();
	outb(PS2_COMMAND, 0xae);

	if(!keyboard_send(0xf0) || !keyboard_send(1))
		return 0;
	if(!keyboard_send(0xf3) || !keyboard_send(0))
		return 0;
	if(!keyboard_send(0xf4))
		return 0;
	return 1;
}

ATTR_HOT
static inline uint8_t get_config_byte(void)
{
	wait_write();
	outb(PS2_COMMAND, 0x20); // TODO Use ps2_command without expected response?

	wait_read();
	return inb(PS2_DATA);
}

ATTR_HOT
static inline void set_config_byte(const uint8_t config_byte)
{
	wait_write();
	outb(PS2_COMMAND, 0x60);
	wait_write();
	outb(PS2_DATA, config_byte); // TODO Check if can write before
}

ATTR_COLD
void ps2_init(void)
{
	// TODO Check if existing using ACPI
	ps2_disable_devices();
	// TODO Discard buffer?
	// inb(PS2_DATA);
	clear_buffer();
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
	clear_buffer();
}

ATTR_COLD
void ps2_set_keyboard_hook(void (*hook)(const uint8_t))
{
	keyboard_hook = hook;
}

ATTR_HOT
void ps2_keyboard_event(void)
{
	if(keyboard_hook)
		keyboard_hook(inb(0x60));
}

ATTR_HOT
int8_t ps2_get_leds_state(void)
{
	return leds_state;
}

ATTR_HOT
void ps2_set_leds_state(const int8_t state)
{
	if(keyboard_send(0xed))
		keyboard_send(leds_state = state);
}
