//! The termios structure defines the IO settings for a terminal.

/// Termcap flags.
pub type TCFlag = u32;
/// Type representing a character.
pub type CC = u8;

/// Size of the array for control characters.
const NCCS: usize = 19;

pub const VINTR: usize = 0;
pub const VQUIT: usize = 1;
pub const VERASE: usize = 2;
pub const VKILL: usize = 3;
pub const VEOF: usize = 4;
pub const VTIME: usize = 5;
pub const VMIN: usize = 6;
pub const VSWTC: usize = 7;
pub const VSTART: usize = 8;
pub const VSTOP: usize = 9;
pub const VSUSP: usize = 10;
pub const VEOL: usize = 11;
pub const VREPRINT: usize = 12;
pub const VDISCARD: usize = 13;
pub const VWERASE: usize = 14;
pub const VLNEXT: usize = 15;
pub const VEOL2: usize = 16;

pub const IGNBRK: TCFlag = 0o000001;
pub const BRKINT: TCFlag = 0o000002;
pub const IGNPAR: TCFlag = 0o000004;
pub const PARMRK: TCFlag = 0o000010;
pub const INPCK: TCFlag = 0o000020;
pub const ISTRIP: TCFlag = 0o000040;
pub const INLCR: TCFlag = 0o000100;
pub const IGNCR: TCFlag = 0o000200;
pub const ICRNL: TCFlag = 0o000400;
pub const IUCLC: TCFlag = 0o001000;
pub const IXON: TCFlag = 0o002000;
pub const IXANY: TCFlag = 0o004000;
pub const IXOFF: TCFlag = 0o010000;
pub const IMAXBEL: TCFlag = 0o020000;
pub const IUTF8: TCFlag = 0o040000;

pub const OPOST: TCFlag = 0o000001;
pub const OLCUC: TCFlag = 0o000002;
pub const ONLCR: TCFlag = 0o000004;
pub const OCRNL: TCFlag = 0o000010;
pub const ONOCR: TCFlag = 0o000020;
pub const ONLRET: TCFlag = 0o000040;
pub const OFILL: TCFlag = 0o000100;
pub const OFDEL: TCFlag = 0o000200;
pub const NLDLY: TCFlag = 0o000400;
pub const NL0: TCFlag = 0o000000;
pub const NL1: TCFlag = 0o000400;
pub const CRDLY: TCFlag = 0o003000;
pub const CR0: TCFlag = 0o000000;
pub const CR1: TCFlag = 0o001000;
pub const CR2: TCFlag = 0o002000;
pub const CR3: TCFlag = 0o003000;
pub const TABDLY: TCFlag = 0o014000;
pub const TAB0: TCFlag = 0o000000;
pub const TAB1: TCFlag = 0o004000;
pub const TAB2: TCFlag = 0o010000;
pub const TAB3: TCFlag = 0o014000;
pub const BSDLY: TCFlag = 0o020000;
pub const BS0: TCFlag = 0o000000;
pub const BS1: TCFlag = 0o020000;
pub const FFDLY: TCFlag = 0o100000;
pub const FF0: TCFlag = 0o000000;
pub const FF1: TCFlag = 0o100000;

pub const VTDLY: TCFlag = 0o040000;
pub const VT0: TCFlag = 0o000000;
pub const VT1: TCFlag = 0o040000;

pub const B0: TCFlag = 0o000000;
pub const B50: TCFlag = 0o000001;
pub const B75: TCFlag = 0o000002;
pub const B110: TCFlag = 0o000003;
pub const B134: TCFlag = 0o000004;
pub const B150: TCFlag = 0o000005;
pub const B200: TCFlag = 0o000006;
pub const B300: TCFlag = 0o000007;
pub const B600: TCFlag = 0o000010;
pub const B1200: TCFlag = 0o000011;
pub const B1800: TCFlag = 0o000012;
pub const B2400: TCFlag = 0o000013;
pub const B4800: TCFlag = 0o000014;
pub const B9600: TCFlag = 0o000015;
pub const B19200: TCFlag = 0o000016;
pub const B38400: TCFlag = 0o000017;

pub const B57600: TCFlag = 0o010001;
pub const B115200: TCFlag = 0o010002;
pub const B230400: TCFlag = 0o010003;
pub const B460800: TCFlag = 0o010004;
pub const B500000: TCFlag = 0o010005;
pub const B576000: TCFlag = 0o010006;
pub const B921600: TCFlag = 0o010007;
pub const B1000000: TCFlag = 0o010010;
pub const B1152000: TCFlag = 0o010011;
pub const B1500000: TCFlag = 0o010012;
pub const B2000000: TCFlag = 0o010013;
pub const B2500000: TCFlag = 0o010014;
pub const B3000000: TCFlag = 0o010015;
pub const B3500000: TCFlag = 0o010016;
pub const B4000000: TCFlag = 0o010017;

pub const CSIZE: TCFlag = 0o000060;
pub const CS5: TCFlag = 0o000000;
pub const CS6: TCFlag = 0o000020;
pub const CS7: TCFlag = 0o000040;
pub const CS8: TCFlag = 0o000060;
pub const CSTOPB: TCFlag = 0o000100;
pub const CREAD: TCFlag = 0o000200;
pub const PARENB: TCFlag = 0o000400;
pub const PARODD: TCFlag = 0o001000;
pub const HUPCL: TCFlag = 0o002000;
pub const CLOCAL: TCFlag = 0o004000;

pub const ISIG: TCFlag = 0o000001;
pub const ICANON: TCFlag = 0o000002;
pub const ECHO: TCFlag = 0o000010;
pub const ECHOE: TCFlag = 0o000020;
pub const ECHOK: TCFlag = 0o000040;
pub const ECHONL: TCFlag = 0o000100;
pub const NOFLSH: TCFlag = 0o000200;
pub const TOSTOP: TCFlag = 0o000400;
pub const IEXTEN: TCFlag = 0o100000;

pub const TCOOFF: TCFlag = 0;
pub const TCOON: TCFlag = 1;
pub const TCIOFF: TCFlag = 2;
pub const TCION: TCFlag = 3;

pub const TCIFLUSH: TCFlag = 0;
pub const TCOFLUSH: TCFlag = 1;
pub const TCIOFLUSH: TCFlag = 2;

pub const TCSANOW: TCFlag = 0;
pub const TCSADRAIN: TCFlag = 1;
pub const TCSAFLUSH: TCFlag = 2;

pub const EXTA: TCFlag = 0o000016;
pub const EXTB: TCFlag = 0o000017;
pub const CBAUD: TCFlag = 0o010017;
pub const CBAUDEX: TCFlag = 0o010000;
pub const CIBAUD: TCFlag = 0o02003600000;
pub const CMSPAR: TCFlag = 0o10000000000;
pub const CRTSCTS: TCFlag = 0o20000000000;

pub const XCASE: TCFlag = 0o000004;
pub const ECHOCTL: TCFlag = 0o001000;
pub const ECHOPRT: TCFlag = 0o002000;
pub const ECHOKE: TCFlag = 0o004000;
pub const FLUSHO: TCFlag = 0o010000;
pub const PENDIN: TCFlag = 0o040000;
pub const EXTPROC: TCFlag = 0o200000;

pub const XTABS: TCFlag = 0o014000;

/// Terminal IO settings.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Termios {
	/// Input modes
	pub c_iflag: TCFlag,
	/// Output modes
	pub c_oflag: TCFlag,
	/// Control modes
	pub c_cflag: TCFlag,
	/// Local modes
	pub c_lflag: TCFlag,
	/// Line discipline
	pub c_line: CC,
	/// Special characters
	pub c_cc: [CC; NCCS],
	/// Input baud rate
	pub __c_ispeed: u32,
	/// Output baud rate
	pub __c_ospeed: u32,
}

impl Default for Termios {
	fn default() -> Self {
		let mut t = Self {
			c_iflag: ICRNL | IUCLC | IXANY | IMAXBEL,
			c_oflag: OPOST | ONLCR,
			c_cflag: CS8,
			c_lflag: ISIG | ICANON | ECHO | ECHOE | ECHOK,
			c_line: 0,
			c_cc: [0; NCCS],
			__c_ispeed: 0,
			__c_ospeed: 0,
		};

		// Filling special characters
		t.c_cc[VINTR] = 0o03;
		t.c_cc[VQUIT] = 0o34;
		t.c_cc[VERASE] = 0o177;
		t.c_cc[VKILL] = 0o25;
		t.c_cc[VEOF] = 0o4;
		t.c_cc[VMIN] = 1;
		t.c_cc[VSTART] = 0o21;
		t.c_cc[VSTOP] = 0o23;
		t.c_cc[VSUSP] = 0o32;
		t.c_cc[VREPRINT] = 0o22;
		t.c_cc[VDISCARD] = 0o17;
		t.c_cc[VWERASE] = 0o27;
		t.c_cc[VLNEXT] = 0o26;

		t
	}
}
