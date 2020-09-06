use super::vga;
use super::util;

/*
 * This module handles TTYs.
 */

// TODO Spinlock
// TODO Sanity checks
// TODO Implement streams and termcaps

/*
 * The number of TTYs.
 */
const TTYS_COUNT: usize = 8;
/*
 * The number of history lines for one TTY.
 */
const HISTORY_LINES: vga::Pos = 128;
/*
 * The number of characters a TTY can store.
 */
const HISTORY_SIZE: usize = (vga::WIDTH as usize) * (HISTORY_LINES as usize);

/*
 * An empty character.
 */
const EMPTY_CHAR: vga::Char = (vga::DEFAULT_COLOR as vga::Char) << 8;

/*
 * The size of a tabulation in space-equivalent.
 */
const TAB_SIZE: usize = 4;

/*
 * TODO
 */
const ANSI_ESCAPE: char = 0x1b as char;

/*
 * The frequency of the bell in Hz.
 */
const BELL_FREQUENCY: u32 = 2000;
/*
 * The duraction of the bell in ms.
 */
const BELL_DURATION: u32 = 500;

/*
 * Returns the position of the cursor in the history array from x and y position.
 */
const fn history_pos(x: vga::Pos, y: vga::Pos) -> usize {
	(y * vga::WIDTH + x) as usize
}

/*
 * Returns the position of a tab character for the given cursor X position.
 */
const fn get_tab_size(cursor_x: vga::Pos) -> usize {
	TAB_SIZE - ((cursor_x as usize) % TAB_SIZE)
}

/*
 * Structure representing a TTY.
 */
#[derive(Copy)]
#[derive(Clone)]
struct TTY
{
	/* The X position of the cursor in the history */
	cursor_x: vga::Pos,
	/* The Y position of the cursor in the history */
	cursor_y: vga::Pos,
	/* The Y position of the screen in the history */
	screen_y: vga::Pos,

	/* The current color for the text to be written */
	current_color: vga::Color,

	/* The content of the TTY's history */
	history: [vga::Char; HISTORY_SIZE],

	/* The number of prompted characters */
	prompted_chars: usize,
	/* Tells whether TTY updates are enabled or not */
	update: bool,

	/* The spinlock for the TTY */
	spinlock: util::Spinlock,
}

/*
 * The array of every TTYs.
 */
static mut TTYS: &'static mut [TTY; TTYS_COUNT] = &mut[TTY {
		cursor_x: 0,
		cursor_y: 0,
		screen_y: 0,
		current_color: vga::DEFAULT_COLOR,
		history: [0; HISTORY_SIZE],
		prompted_chars: 0,
		update: true,
		spinlock: util::Spinlock::new(),
	}; TTYS_COUNT];
/*
 * The current TTY's id.
 */
static mut CURRENT_TTY: usize = 0;

/*
 * Initializes every TTYs.
 */
pub fn init() {
	for i in 0..TTYS_COUNT {
		unsafe {
			TTYS[i].clear();
		}
	}
	TTY::switch(0);
}

impl TTY {
	/*
	 * Switches to TTY with id `tty`.
	 */
	pub fn switch(tty: usize) {
		if tty >= TTYS_COUNT {
			return;
		}
		unsafe {
			CURRENT_TTY = tty;
		}

		let mut t = unsafe { TTYS[tty] };
		vga::enable_cursor();
		vga::move_cursor(t.cursor_x, t.cursor_y);
		t.update();
	}

	/*
	 * Returns a reference to the current TTY.
	 */
	pub fn current() -> &'static mut Self {
		unsafe {
			&mut TTYS[CURRENT_TTY]
		}
	}

	/*
	 * Updates the TTY to the screen.
	 */
	pub fn update(&mut self) {
		if !self.update {
			return;
		}
		// TODO Screen update
	}

	/*
	 * Reinitializes TTY's current attributes.
	 */
	pub fn reset_attrs(&mut self) {
		self.current_color = vga::DEFAULT_COLOR;
		// TODO
	}

	/*
	 * Sets the current foreground color `color` for TTY.
	 */
	pub fn set_fgcolor(&mut self, color: vga::Color) {
		self.current_color &= !(0xff as vga::Color);
		self.current_color |= color;
	}

	/*
	 * Sets the current background color `color` for TTY.
	 */
	pub fn set_bgcolor(&mut self, color: vga::Color) {
		self.current_color &= !((0xff << 4) as vga::Color);
		self.current_color |= color << 4;
	}

	/*
	 * Clears the TTY's history.
	 */
	pub fn clear(&mut self) {
		self.cursor_x = 0;
		self.cursor_y = 0;
		self.screen_y = 0;
		for i in 0..self.history.len() {
			self.history[i] = 0;
		}
		self.update();
	}

	fn fix_pos(&mut self) {
		if self.cursor_x < 0 {
			let p = -self.cursor_x;
			self.cursor_x = vga::WIDTH - (p % vga::WIDTH);
			self.cursor_y += p / vga::WIDTH - 1;
		}

		if self.cursor_x >= vga::WIDTH {
			let p = self.cursor_x;
			self.cursor_x = p % vga::WIDTH;
			self.cursor_y += p / vga::WIDTH;
		}

		if self.cursor_y < self.screen_y {
			self.screen_y = self.cursor_y;
		}

		if self.cursor_y >= self.screen_y + vga::HEIGHT {
			self.screen_y = self.cursor_y - vga::HEIGHT + 1;
		}

		if self.cursor_y >= HISTORY_LINES {
			self.cursor_y = HISTORY_LINES - 1;
		}

		if self.screen_y < 0 {
			self.screen_y = 0;
		}

		if self.screen_y + vga::HEIGHT > HISTORY_LINES {
			let diff = ((self.screen_y + vga::HEIGHT - HISTORY_LINES) * vga::WIDTH) as usize;
			let size = self.history.len() - diff;
			for i in 0..size {
				self.history[i] = self.history[diff + i];
			}
			for i in size..diff {
				self.history[i] = 0;
			}
			self.screen_y = HISTORY_LINES - vga::HEIGHT;
		}
	}

	fn cursor_forward(&mut self, x: usize, y: usize)
	{
		self.cursor_x += x as vga::Pos;
		self.cursor_y += y as vga::Pos;
		self.fix_pos();
	}

	fn cursor_backward(&mut self, x: usize, y: usize)
	{
		self.cursor_x -= x as vga::Pos;
		self.cursor_y -= y as vga::Pos;
		self.fix_pos();
	}

	fn newline(&mut self)
	{
		self.cursor_x = 0;
		self.cursor_y += 1;
		self.fix_pos();
	}

	pub fn putchar(&mut self, c: char) {
		match c {
			'\x08' => {
				// TODO Bell beep
			},
			'\t' => {
				self.cursor_forward(get_tab_size(self.cursor_x), 0);
			},
			'\n' => {
				self.newline();
			},
			'\r' => {
				self.cursor_x = 0;
			},
			_ => {
				let tty_char = (c as vga::Char) | ((self.current_color as vga::Char) << 8);
				self.history[history_pos(self.cursor_x, self.cursor_y)] = tty_char;
				self.cursor_forward(1, 0);
			}
		}
		self.update();
	}

	/*
	 * Writes string `buffer` to TTY.
	 */
	pub fn write(&mut self, buffer: &str) {
		for i in 0..buffer.len() {
			let c = buffer.as_bytes()[i] as char;
			if c != ANSI_ESCAPE {
				self.putchar(c);
			} else {
				// TODO Handle ANSI
			}
			self.update();
		}
	}

	/*
	 * Erases `count` characters in TTY.
	 */
	pub fn erase(&mut self, mut count: usize) {
		count = util::max(count, self.prompted_chars);
		if count == 0 {
			return;
		}
		self.cursor_backward(count, 0);

		let begin = history_pos(self.cursor_x, self.cursor_y);
		for i in begin..(begin + count) {
			self.history[i] = EMPTY_CHAR;
		}
		self.update();
		self.prompted_chars -= count;
	}

	/*
	 * Handles keyboard insert input for keycode `code`.
	 */
	/*pub fn input_hook(&mut self, code: ps2::key_code) {
		// TODO
	}*/

	/*
	 * Handles keyboard control input for keycode `code`.
	 */
	/*pub fn ctrl_hook(&mut self, code: ps2::key_code) {
		// TODO
	}*/

	/*
	 * Handles keyboard erase input for keycode.
	 */
	pub fn erase_hook(&mut self) {
		self.erase(1);
	}
}
