//! The termios structure defines the IO settings for a terminal.

/// Termcap flags.
pub type TCFlag = u32;
/// TODO doc
pub type CC = u8;

/// Size of the array for control characters.
const NCCS: usize = 19;

const VINTR: TCFlag = 0;
const VQUIT: TCFlag = 1;
const VERASE: TCFlag = 2;
const VKILL: TCFlag = 3;
const VEOF: TCFlag = 4;
const VTIME: TCFlag = 5;
const VMIN: TCFlag = 6;
const VSWTC: TCFlag = 7;
const VSTART: TCFlag = 8;
const VSTOP: TCFlag = 9;
const VSUSP: TCFlag = 10;
const VEOL: TCFlag = 11;
const VREPRINT: TCFlag = 12;
const VDISCARD: TCFlag = 13;
const VWERASE: TCFlag = 14;
const VLNEXT: TCFlag = 15;
const VEOL2: TCFlag = 16;

const IGNBRK: TCFlag = 0o000001;
const BRKINT: TCFlag = 0o000002;
const IGNPAR: TCFlag = 0o000004;
const PARMRK: TCFlag = 0o000010;
const INPCK: TCFlag = 0o000020;
const ISTRIP: TCFlag = 0o000040;
const INLCR: TCFlag = 0o000100;
const IGNCR: TCFlag = 0o000200;
const ICRNL: TCFlag = 0o000400;
const IUCLC: TCFlag = 0o001000;
const IXON: TCFlag = 0o002000;
const IXANY: TCFlag = 0o004000;
const IXOFF: TCFlag = 0o010000;
const IMAXBEL: TCFlag = 0o020000;
const IUTF8: TCFlag = 0o040000;

const OPOST: TCFlag = 0o000001;
const OLCUC: TCFlag = 0o000002;
const ONLCR: TCFlag = 0o000004;
const OCRNL: TCFlag = 0o000010;
const ONOCR: TCFlag = 0o000020;
const ONLRET: TCFlag = 0o000040;
const OFILL: TCFlag = 0o000100;
const OFDEL: TCFlag = 0o000200;
const NLDLY: TCFlag = 0o000400;
const NL0: TCFlag = 0o000000;
const NL1: TCFlag = 0o000400;
const CRDLY: TCFlag = 0o003000;
const CR0: TCFlag = 0o000000;
const CR1: TCFlag = 0o001000;
const CR2: TCFlag = 0o002000;
const CR3: TCFlag = 0o003000;
const TABDLY: TCFlag = 0o014000;
const TAB0: TCFlag = 0o000000;
const TAB1: TCFlag = 0o004000;
const TAB2: TCFlag = 0o010000;
const TAB3: TCFlag = 0o014000;
const BSDLY: TCFlag = 0o020000;
const BS0: TCFlag = 0o000000;
const BS1: TCFlag = 0o020000;
const FFDLY: TCFlag = 0o100000;
const FF0: TCFlag = 0o000000;
const FF1: TCFlag = 0o100000;

const VTDLY: TCFlag = 0o040000;
const VT0: TCFlag = 0o000000;
const VT1: TCFlag = 0o040000;

const B0: TCFlag = 0o000000;
const B50: TCFlag = 0o000001;
const B75: TCFlag = 0o000002;
const B110: TCFlag = 0o000003;
const B134: TCFlag = 0o000004;
const B150: TCFlag = 0o000005;
const B200: TCFlag = 0o000006;
const B300: TCFlag = 0o000007;
const B600: TCFlag = 0o000010;
const B1200: TCFlag = 0o000011;
const B1800: TCFlag = 0o000012;
const B2400: TCFlag = 0o000013;
const B4800: TCFlag = 0o000014;
const B9600: TCFlag = 0o000015;
const B19200: TCFlag = 0o000016;
const B38400: TCFlag = 0o000017;

const B57600: TCFlag = 0o010001;
const B115200: TCFlag = 0o010002;
const B230400: TCFlag = 0o010003;
const B460800: TCFlag = 0o010004;
const B500000: TCFlag = 0o010005;
const B576000: TCFlag = 0o010006;
const B921600: TCFlag = 0o010007;
const B1000000: TCFlag = 0o010010;
const B1152000: TCFlag = 0o010011;
const B1500000: TCFlag = 0o010012;
const B2000000: TCFlag = 0o010013;
const B2500000: TCFlag = 0o010014;
const B3000000: TCFlag = 0o010015;
const B3500000: TCFlag = 0o010016;
const B4000000: TCFlag = 0o010017;

const CSIZE: TCFlag = 0o000060;
const CS5: TCFlag = 0o000000;
const CS6: TCFlag = 0o000020;
const CS7: TCFlag = 0o000040;
const CS8: TCFlag = 0o000060;
const CSTOPB: TCFlag = 0o000100;
const CREAD: TCFlag = 0o000200;
const PARENB: TCFlag = 0o000400;
const PARODD: TCFlag = 0o001000;
const HUPCL: TCFlag = 0o002000;
const CLOCAL: TCFlag = 0o004000;

const ISIG: TCFlag = 0o000001;
const ICANON: TCFlag = 0o000002;
const ECHO: TCFlag = 0o000010;
const ECHOE: TCFlag = 0o000020;
const ECHOK: TCFlag = 0o000040;
const ECHONL: TCFlag = 0o000100;
const NOFLSH: TCFlag = 0o000200;
const TOSTOP: TCFlag = 0o000400;
const IEXTEN: TCFlag = 0o100000;

const TCOOFF: TCFlag = 0;
const TCOON: TCFlag = 1;
const TCIOFF: TCFlag = 2;
const TCION: TCFlag = 3;

const TCIFLUSH: TCFlag = 0;
const TCOFLUSH: TCFlag = 1;
const TCIOFLUSH: TCFlag = 2;

const TCSANOW: TCFlag = 0;
const TCSADRAIN: TCFlag = 1;
const TCSAFLUSH: TCFlag = 2;

const EXTA: TCFlag = 0o000016;
const EXTB: TCFlag = 0o000017;
const CBAUD: TCFlag = 0o010017;
const CBAUDEX: TCFlag = 0o010000;
const CIBAUD: TCFlag = 0o02003600000;
const CMSPAR: TCFlag = 0o10000000000;
const CRTSCTS: TCFlag = 0o20000000000;

const XCASE: TCFlag = 0o000004;
const ECHOCTL: TCFlag = 0o001000;
const ECHOPRT: TCFlag = 0o002000;
const ECHOKE: TCFlag = 0o004000;
const FLUSHO: TCFlag = 0o010000;
const PENDIN: TCFlag = 0o040000;
const EXTPROC: TCFlag = 0o200000;

const XTABS: TCFlag = 0o014000;

/// Terminal IO settings.
#[repr(C)]
#[derive(Clone)]
pub struct Termios {
	/// Input modes
	pub c_iflag: TCFlag,
	/// Output modes
	pub c_oflag: TCFlag,
	/// Control modes
	pub c_cflag: TCFlag,
	/// Local modes
	pub c_lflag: TCFlag,
	/// Special characters
	pub c_cc: [CC; NCCS],
}

impl Default for Termios {
	fn default() -> Self {
		Self {
			c_iflag: 0, // TODO
			c_oflag: 0, // TODO
			c_cflag: 0, // TODO
			c_lflag: 0, // TODO
			c_cc: [0; NCCS], // TODO
		}
	}
}

impl Termios {
	/// Tells whether the terminal is in canonical mode.
	pub fn is_canonical_mode(&self) -> bool {
		self.c_iflag & ICANON != 0
	}
}
