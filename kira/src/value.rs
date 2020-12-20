use std::ops::Range;

use nanorand::{tls_rng, RNG};

use crate::{
	parameter::{Mapping, ParameterId, Parameters},
	util::{lerp, random_float_0_1},
};

/// A trait for types that can be used as [`Value`]s.
pub trait AsValue: std::fmt::Debug + Copy + From<f64> {
	/// Gets a random value of this type within a range.
	fn random_in_range(lower: Self, upper: Self, rng: &mut impl RNG) -> Self;
}

impl AsValue for f64 {
	fn random_in_range(lower: Self, upper: Self, rng: &mut impl RNG) -> Self {
		lerp(lower, upper, random_float_0_1(rng))
	}
}

/// A value that something can be set to.
#[derive(Debug, Copy, Clone)]
pub enum Value<T: AsValue> {
	/// A fixed value.
	Fixed(T),
	/// The current value of a parameter.
	Parameter(ParameterId, Mapping),
	/// A random value within a range.
	Random(T, T),
}

impl<T: AsValue> From<T> for Value<T> {
	fn from(value: T) -> Self {
		Self::Fixed(value)
	}
}

impl<T: AsValue> From<ParameterId> for Value<T> {
	fn from(id: ParameterId) -> Self {
		Self::Parameter(id, Mapping::default())
	}
}

impl<T: AsValue> From<Range<T>> for Value<T> {
	fn from(range: Range<T>) -> Self {
		Self::Random(range.start, range.end)
	}
}

/// A wrapper around [`Value`](crate::Value)s that remembers the last valid raw value.
///
/// You'll only need to use this if you're writing your own effects.
#[derive(Debug, Copy, Clone)]
pub struct CachedValue<T: AsValue> {
	value: Value<T>,
	last_value: T,
}

impl<T: AsValue> CachedValue<T> {
	/// Creates a `CachedValue` with an initial value setting
	/// and a default raw value to fall back on.
	pub fn new(value: Value<T>, default_value: T) -> Self {
		Self {
			value,
			last_value: match value {
				Value::Fixed(value) => value,
				Value::Parameter(_, _) => default_value,
				Value::Random(lower, upper) => T::random_in_range(lower, upper, &mut *tls_rng()),
			},
		}
	}

	/// Sets the value.
	pub fn set(&mut self, value: Value<T>) {
		self.value = value;
		match value {
			Value::Random(lower, upper) => {
				self.last_value = T::random_in_range(lower, upper, &mut *tls_rng());
			}
			_ => {}
		}
	}

	/// If the value is set to a parameter, updates the raw value
	/// from the parameter (if it exists).
	pub fn update(&mut self, parameters: &Parameters) {
		match self.value {
			Value::Parameter(id, mapping) => {
				if let Some(parameter) = parameters.get(id) {
					self.last_value = mapping.map(parameter.value()).into();
				}
			}
			_ => {}
		}
	}

	/// Gets the last valid raw value.
	pub fn value(&self) -> T {
		self.last_value
	}
}
