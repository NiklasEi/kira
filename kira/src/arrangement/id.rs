use std::{
	hash::Hash,
	sync::atomic::{AtomicUsize, Ordering},
};

use crate::mixer::TrackId;

use super::{ArrangementHandle, InternalArrangement};

static NEXT_ARRANGEMENT_INDEX: AtomicUsize = AtomicUsize::new(0);

/**
A unique identifier for an [`Arrangement`](Arrangement).

You cannot create this manually - an arrangement ID is created
when you [add an arrangement](crate::manager::AudioManager::add_arrangement)
to an [`AudioManager`](crate::manager::AudioManager).
*/
#[derive(Debug, Copy, Clone)]
pub struct ArrangementId {
	index: usize,
	duration: f64,
	default_track: TrackId,
	semantic_duration: Option<f64>,
	default_loop_start: Option<f64>,
}

impl ArrangementId {
	pub(crate) fn new(arrangement: &InternalArrangement) -> Self {
		let index = NEXT_ARRANGEMENT_INDEX.fetch_add(1, Ordering::Relaxed);
		Self {
			index,
			duration: arrangement.duration(),
			default_track: arrangement.default_track,
			semantic_duration: arrangement.semantic_duration,
			default_loop_start: arrangement.default_loop_start,
		}
	}

	/// Gets the duration of the arrangement.
	pub fn duration(&self) -> f64 {
		self.duration
	}

	/// Gets the default track that instances of this arrangement
	/// will play on.
	pub fn default_track(&self) -> TrackId {
		self.default_track
	}

	/// Gets the [semantic duration](crate::playable::PlayableSettings#structfield.semantic_duration)
	/// of the arrangement.
	pub fn semantic_duration(&self) -> Option<f64> {
		self.semantic_duration
	}

	/// Gets the default loop start point for instances of this
	/// arrangement.
	pub fn default_loop_start(&self) -> Option<f64> {
		self.default_loop_start
	}
}

impl PartialEq for ArrangementId {
	fn eq(&self, other: &Self) -> bool {
		self.index == other.index
	}
}

impl Eq for ArrangementId {}

impl Hash for ArrangementId {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.index.hash(state);
	}
}

impl From<&ArrangementHandle> for ArrangementId {
	fn from(handle: &ArrangementHandle) -> Self {
		handle.id()
	}
}
