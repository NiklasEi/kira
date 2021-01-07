mod handle;

pub use handle::TrackHandle;

use std::sync::atomic::{AtomicUsize, Ordering};

use indexmap::IndexMap;

use crate::{frame::Frame, parameter::Parameters};

use super::{
	effect::{Effect, EffectId, EffectSettings},
	effect_slot::EffectSlot,
};

static NEXT_SUB_TRACK_INDEX: AtomicUsize = AtomicUsize::new(0);

/**
A unique identifier for a sub-track.

You cannot create this manually - a `SubTrackId` is created
when you create a sub-track with an [`AudioManager`](crate::manager::AudioManager).
*/
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SubTrackId {
	index: usize,
}

impl SubTrackId {
	pub(crate) fn new() -> Self {
		let index = NEXT_SUB_TRACK_INDEX.fetch_add(1, Ordering::Relaxed);
		Self { index }
	}
}

/// Represents a mixer track.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TrackId {
	/// The main track.
	///
	/// All sub-tracks are sent to the main track as input,
	/// and the output of the main track is what you hear.
	Main,
	/// A sub-track.
	///
	/// Sub-tracks are useful for adjusting the volumes of
	/// and applying effects to certain kinds of sounds.
	/// For example, in a game, you may have one sub-track
	/// for sound effects and another for music.
	Sub(SubTrackId),
}

impl Default for TrackId {
	fn default() -> Self {
		Self::Main
	}
}

impl From<SubTrackId> for TrackId {
	fn from(id: SubTrackId) -> Self {
		Self::Sub(id)
	}
}

impl From<&TrackHandle> for TrackId {
	fn from(handle: &TrackHandle) -> Self {
		handle.id()
	}
}

#[derive(Debug, Clone)]
pub enum TrackLabel {
	Id(TrackId),
	Name(String),
}

impl Default for TrackLabel {
	fn default() -> Self {
		Self::Id(TrackId::default())
	}
}

impl From<TrackId> for TrackLabel {
	fn from(id: TrackId) -> Self {
		Self::Id(id)
	}
}

impl From<SubTrackId> for TrackLabel {
	fn from(id: SubTrackId) -> Self {
		Self::Id(TrackId::Sub(id))
	}
}

impl From<&TrackHandle> for TrackLabel {
	fn from(handle: &TrackHandle) -> Self {
		Self::Id(handle.id())
	}
}

impl From<String> for TrackLabel {
	fn from(name: String) -> Self {
		Self::Name(name)
	}
}

impl From<&str> for TrackLabel {
	fn from(name: &str) -> Self {
		Self::Name(name.into())
	}
}

/// Settings for a mixer track.
#[derive(Debug, Copy, Clone)]
pub struct TrackSettings {
	/// The volume of the track.
	pub volume: f64,
}

impl Default for TrackSettings {
	fn default() -> Self {
		Self { volume: 1.0 }
	}
}

#[derive(Debug)]
pub(crate) struct Track {
	volume: f64,
	effect_slots: IndexMap<EffectId, EffectSlot>,
	input: Frame,
}

impl Track {
	pub fn new(settings: TrackSettings) -> Self {
		Self {
			volume: settings.volume,
			effect_slots: IndexMap::new(),
			input: Frame::from_mono(0.0),
		}
	}

	pub fn add_effect(&mut self, id: EffectId, effect: Box<dyn Effect>, settings: EffectSettings) {
		self.effect_slots
			.insert(id, EffectSlot::new(effect, settings));
	}

	pub fn remove_effect(&mut self, id: EffectId) -> Option<EffectSlot> {
		self.effect_slots.remove(&id)
	}

	pub fn add_input(&mut self, input: Frame) {
		self.input += input;
	}

	pub fn process(&mut self, dt: f64, parameters: &Parameters) -> Frame {
		let mut input = self.input;
		self.input = Frame::from_mono(0.0);
		for (_, effect_slot) in &mut self.effect_slots {
			input = effect_slot.process(dt, input, parameters);
		}
		input * (self.volume as f32)
	}
}
