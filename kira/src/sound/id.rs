use std::{
	hash::Hash,
	sync::atomic::{AtomicUsize, Ordering},
};

use crate::mixer::{TrackId, TrackLabel};

use super::{Sound, SoundHandle};

static NEXT_SOUND_INDEX: AtomicUsize = AtomicUsize::new(0);

/// A unique identifier for a [`Sound`](crate::sound::Sound).
///
/// You cannot create this manually - a sound ID is returned
/// when you add a sound to an [`AudioManager`](crate::manager::AudioManager).
#[derive(Debug, Copy, Clone)]
pub struct SoundId {
	index: usize,
	duration: f64,
	default_track: TrackId,
	semantic_duration: Option<f64>,
	default_loop_start: Option<f64>,
}

impl SoundId {
	pub(crate) fn new(sound: &Sound) -> Self {
		let index = NEXT_SOUND_INDEX.fetch_add(1, Ordering::Relaxed);
		Self {
			index,
			duration: sound.duration(),
			default_track: match sound.default_track() {
				TrackLabel::Id(id) => *id,
				TrackLabel::Name(_) => panic!(
					"Default track must be converted from name to ID before creating a SoundId.  This is an internal bug in Kira."
				),
			},
			semantic_duration: sound.semantic_duration(),
			default_loop_start: sound.default_loop_start(),
		}
	}

	/// Gets the duration of the sound.
	pub fn duration(&self) -> f64 {
		self.duration
	}

	/// Gets the default track that instances of this sound
	/// will play on.
	pub fn default_track(&self) -> TrackId {
		self.default_track
	}

	/// Gets the [semantic duration](crate::playable::PlayableSettings#structfield.semantic_duration)
	/// of the sound.
	pub fn semantic_duration(&self) -> Option<f64> {
		self.semantic_duration
	}

	/// Gets the default loop start point for instances of this
	/// sound.
	pub fn default_loop_start(&self) -> Option<f64> {
		self.default_loop_start
	}
}

impl PartialEq for SoundId {
	fn eq(&self, other: &Self) -> bool {
		self.index == other.index
	}
}

impl Eq for SoundId {}

impl Hash for SoundId {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.index.hash(state);
	}
}

impl From<&SoundHandle> for SoundId {
	fn from(handle: &SoundHandle) -> Self {
		handle.id()
	}
}
