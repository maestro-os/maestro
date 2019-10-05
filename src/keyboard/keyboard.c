#include <keyboard/keyboard.h>

static key_state_t key_states[KEYS_COUNT];
static bool capslock_state = false;

void (*input_hook)(const key_code_t) = NULL;
void (*ctrl_hook)(const key_code_t) = NULL;
void (*erase_hook)() = NULL;

__attribute__((hot))
static void type_key(const key_code_t code)
{
	if(code == KEY_BACKSPACE)
	{
		if(erase_hook) erase_hook();
		return;
	}
	else if(code == KEY_CAPSLOCK)
		capslock_state = !capslock_state;
	// TODO Arrows, etc...
	if(keyboard_get_char(code, keyboard_is_shift_enabled()))
	{
		if(input_hook)
			input_hook(code);
	}
	else if(ctrl_hook)
		ctrl_hook(code);
}
__attribute__((hot))
static void handle_extra_key(const key_code_t code)
{
	if(code < 0x90)
	{
		key_states[code - 0x10 + 0x60] = KEY_STATE_PRESSED;
		type_key(code - 0x1);
	}
	else
		key_states[code - 0x90 + 0x60] = KEY_STATE_RELEASED;
}

__attribute__((hot))
static void handle_normal_key(const key_code_t code)
{
	if(code < 0x80)
	{
		key_states[code - 0x1] = KEY_STATE_PRESSED;
		type_key(code - 0x1);
	}
	else
		key_states[code - 0x81] = KEY_STATE_RELEASED;
}

__attribute__((hot))
static void keyboard_handler(const key_code_t code)
{
	static bool extra_keys = false;

	if(!extra_keys && code == EXTRA_KEYS_CODE)
	{
		extra_keys = true;
		return;
	}

	if(extra_keys)
		handle_extra_key(code);
	else
		handle_normal_key(code);

	extra_keys = false;
}

__attribute__((cold))
void keyboard_init(void)
{
	ps2_set_keyboard_hook(keyboard_handler);
	bzero(key_states, sizeof(key_states));
}

__attribute__((hot))
key_state_t keyboard_get_state(const key_code_t key)
{
	return key_states[key];
}

__attribute__((hot))
bool keyboard_is_ctrl_enabled(void)
{
	return keyboard_get_state(KEY_LEFT_CTRL)
		|| keyboard_get_state(KEY_RIGHT_CTRL);
}

__attribute__((hot))
bool keyboard_is_shift_enabled(void)
{
	const bool shift = keyboard_get_state(KEY_LEFT_SHIFT)
			|| keyboard_get_state(KEY_RIGHT_SHIFT);
	return (capslock_state ? !shift : shift);
}

__attribute__((hot))
bool keyboard_is_capslock_enabled(void)
{
	return capslock_state;
}

__attribute__((hot))
char keyboard_get_char(const key_code_t code, const bool shift)
{
	switch(code)
	{
		case KEY_1: return (shift ? '!' : '1');
		case KEY_2: return (shift ? '@' : '2');
		case KEY_3: return (shift ? '#' : '3');
		case KEY_4: return (shift ? '$' : '4');
		case KEY_5: return (shift ? '%' : '5');
		case KEY_6: return (shift ? '^' : '6');
		case KEY_7: return (shift ? '&' : '7');
		case KEY_8: return (shift ? '*' : '8');
		case KEY_9: return (shift ? '(' : '9');
		case KEY_0: return (shift ? ')' : '0');
		case KEY_MINUS: return (shift ? '_' : '-');
		case KEY_EQUAL: return (shift ? '+' : '=');
		case KEY_TAB: return '\t';
		case KEY_Q: return (shift ? 'Q' : 'q');
		case KEY_W: return (shift ? 'W' : 'w');
		case KEY_E: return (shift ? 'E' : 'e');
		case KEY_R: return (shift ? 'R' : 'r');
		case KEY_T: return (shift ? 'T' : 't');
		case KEY_Y: return (shift ? 'Y' : 'y');
		case KEY_U: return (shift ? 'U' : 'u');
		case KEY_I: return (shift ? 'I' : 'i');
		case KEY_O: return (shift ? 'O' : 'o');
		case KEY_P: return (shift ? 'P' : 'p');
		case KEY_LEFT_BRACKET: return (shift ? '{' : '[');
		case KEY_RIGHT_BRACKET: return (shift ? '}' : ']');
		case KEY_ENTER: return '\n';
		case KEY_A: return (shift ? 'A' : 'a');
		case KEY_S: return (shift ? 'S' : 's');
		case KEY_D: return (shift ? 'D' : 'd');
		case KEY_F: return (shift ? 'F' : 'f');
		case KEY_G: return (shift ? 'G' : 'g');
		case KEY_H: return (shift ? 'H' : 'h');
		case KEY_J: return (shift ? 'J' : 'j');
		case KEY_K: return (shift ? 'K' : 'k');
		case KEY_L: return (shift ? 'L' : 'l');
		case KEY_SEMICOLON: return (shift ? ':' : ';');
		case KEY_SINGLE_QUOTE: return (shift ? '"' : '\'');
		case KEY_BACKTICK: return (shift ? '~' : '`');
		case KEY_BACKSLASH: return (shift ? '|' : '\\');
		case KEY_Z: return (shift ? 'Z' : 'z');
		case KEY_X: return (shift ? 'X' : 'x');
		case KEY_C: return (shift ? 'C' : 'c');
		case KEY_V: return (shift ? 'V' : 'v');
		case KEY_B: return (shift ? 'B' : 'b');
		case KEY_N: return (shift ? 'N' : 'n');
		case KEY_M: return (shift ? 'M' : 'm');
		case KEY_COMMA: return (shift ? '<' : ',');
		case KEY_DOT: return (shift ? '>' : '.');
		case KEY_SLASH: return (shift ? '?' : '/');
		case KEY_KEYPAD_STAR: return '*';
		case KEY_SPACE: return ' ';
		case KEY_KEYPAD_7: return (shift ? '\0' : '7');
		case KEY_KEYPAD_8: return (shift ? '\0' : '8');
		case KEY_KEYPAD_9: return (shift ? '\0' : '9');
		case KEY_KEYPAD_MINUS: return (shift ? '\0' : '-');
		case KEY_KEYPAD_4: return (shift ? '\0' : '4');
		case KEY_KEYPAD_5: return (shift ? '\0' : '5');
		case KEY_KEYPAD_6: return (shift ? '\0' : '6');
		case KEY_KEYPAD_PLUS: return (shift ? '\0' : '+');
		case KEY_KEYPAD_1: return (shift ? '\0' : '1');
		case KEY_KEYPAD_2: return (shift ? '\0' : '2');
		case KEY_KEYPAD_3: return (shift ? '\0' : '3');
		case KEY_KEYPAD_0: return (shift ? '\0' : '0');
		case KEY_KEYPAD_DOT: return (shift ? '\0' : '.');

		default: return '\0';
	}
}

__attribute__((cold))
void keyboard_set_input_hook(void (*hook)(const key_code_t))
{
	input_hook = hook;
}

__attribute__((cold))
void keyboard_set_ctrl_hook(void (*hook)(const key_code_t))
{
	ctrl_hook = hook;
}

__attribute__((cold))
void keyboard_set_erase_hook(void (*hook)(void))
{
	erase_hook = hook;
}
