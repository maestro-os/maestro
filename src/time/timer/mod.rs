//! This module implements timers.

use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::math::rational::Rational;

pub mod pit;

/// A timer is an which executes an action at a given frequency.
pub trait Timer {
	/// Returns the name of the timer.
	fn get_name(&self) -> &'static str;

	/// Returns the maximum frequency of the timer in hertz.
	fn get_max_frequency(&self) -> Rational;
	/// Returns the current frequency of the timer in hertz.
	fn get_curr_frequency(&self) -> Rational;

	/// Sets the current frequency of the timer in hertz. The timer is approximating the given
	/// frequency to the closest supported. To get the exact frequency, one should use
	/// `get_curr_frequency` after setting it.
	/// If the given frequency is negative, the behaviour is undefined.
	fn set_curr_frequency(&mut self, frequency: Rational);

	/// Defines the callback which is called each times the timer sends a signal.
	fn set_callback(&mut self, callback: Box<dyn FnMut()>);
}

/// Trait representing an object that can be ticked by a timer.
pub trait Tickable {
	/// The function to be called when the object is ticked.
	fn tick(&mut self);
}

/// The frequency divider allows to take as input the signal of a timer and to divide it to target
/// a lower frequency.
pub struct FrequencyDivider<O: Fn()> {
	/// The number of signals to count before sending one signal to the output.
	count: u64,
	/// The number of signals counted so far.
	i: u64,

	/// The output callback.
	output: O,
}

impl<O: Fn()> FrequencyDivider<O> {
	/// Creates a new instance.
	pub fn new(count: u64, output: O) -> Self {
		Self {
			count,
			i: 0,

			output,
		}
	}

	/// Returns `b` in the formula `a / b = c`, where `a` is the input frequency and `c` is the
	/// output frequency.
	pub fn get_count(&self) -> u64 {
		self.count
	}

	/// Function to call to receive an input signal.
	pub fn input(&mut self) {
		self.i += 1;

		if self.i >= self.count {
			(self.output)();
			self.i = 0;
		}
	}
}

/// A structure wrapping a tickable object registered into a TimerManager.
struct TickableWrapper {
	/// The object to be ticked.
	tickable: Box<dyn Tickable>,

	/// `frequency` is the object's ticking frequency.
	frequency: Rational,
	/// `once` tells whether the object has to be ticked only once. If true, the object will be
	/// removed right after being ticked.
	once: bool,
}

/// A structure managing a timer, allowing to link the timer to its tickable objects.
pub struct TimerManager {
	/// The timer bound to the current manager.
	timer: Box<dyn Timer>,

	/// The list of tickable objects bound to the current manager.
	tickable: Vec<TickableWrapper>,
}

impl TimerManager {
	/// Creates a new instance with the given timer. The structure will redefine the timer's
	/// callback.
	pub fn new<T: 'static + Timer>(mut timer: T) -> Result<Self, Errno> {
		timer.set_callback(Box::new(|| {
			// TODO
		})?);

		Ok(Self {
			timer: Box::new(timer)?,

			tickable: Vec::new(),
		})
	}

	/// Registers the given object to be ticked.
	/// `tickable` is the object to be ticked.
	/// `frequency` is the object's ticking frequency.
	/// `once` tells whether the object has to be ticked only once. If true, the object will be
	/// removed right after being ticked.
	pub fn register_tickable<T: 'static + Tickable>(&mut self, tickable: T, frequency: Rational,
		once: bool) -> Result<(), Errno> {
		let wrapper = TickableWrapper {
			tickable: Box::new(tickable)?,

			frequency,
			once,
		};
		// TODO Update timer frequency accordingly

		self.tickable.push(wrapper)
	}

	// TODO Add functions to unregister tickables
}
