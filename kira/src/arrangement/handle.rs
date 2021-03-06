//! An interface for controlling arrangements.

use crate::{
	command::{
		producer::{CommandError, CommandProducer},
		InstanceCommand,
	},
	instance::{
		handle::InstanceHandle, Instance, InstanceId, InstanceSettings, PauseInstanceSettings,
		ResumeInstanceSettings, StopInstanceSettings,
	},
	mixer::TrackIndex,
};

use super::{Arrangement, ArrangementId};

/// Allows you to control an arrangement.
#[derive(Debug, Clone)]
pub struct ArrangementHandle {
	id: ArrangementId,
	duration: f64,
	default_track: TrackIndex,
	semantic_duration: Option<f64>,
	default_loop_start: Option<f64>,
	command_producer: CommandProducer,
}

impl ArrangementHandle {
	pub(crate) fn new(arrangement: &Arrangement, command_producer: CommandProducer) -> Self {
		Self {
			id: arrangement.id(),
			duration: arrangement.duration(),
			default_track: arrangement.default_track(),
			semantic_duration: arrangement.semantic_duration(),
			default_loop_start: arrangement.default_loop_start(),
			command_producer,
		}
	}

	/// Returns the ID of the arrangement.
	pub fn id(&self) -> ArrangementId {
		self.id
	}

	/// Returns the duration of the arrangement (in seconds).
	pub fn duration(&self) -> f64 {
		self.duration
	}

	/// Returns the default track instances of this
	/// arrangement will play on.
	pub fn default_track(&self) -> TrackIndex {
		self.default_track
	}

	/// Returns the "musical length" of this arrangement (if there
	/// is one).
	pub fn semantic_duration(&self) -> Option<f64> {
		self.semantic_duration
	}

	/// Returns the default time (in seconds) instances
	/// of this arrangement will loop back to when they reach
	/// the end.
	pub fn default_loop_start(&self) -> Option<f64> {
		self.default_loop_start
	}

	/// Plays the arrangement.
	pub fn play(&mut self, settings: InstanceSettings) -> Result<InstanceHandle, CommandError> {
		let id = settings.id.unwrap_or(InstanceId::new());
		let instance = Instance::new(
			self.id.into(),
			self.duration,
			None,
			settings.into_internal(self.duration, self.default_loop_start, self.default_track),
		);
		let handle = InstanceHandle::new(
			id,
			instance.public_state(),
			instance.public_position(),
			self.command_producer.clone(),
		);
		self.command_producer
			.push(InstanceCommand::Play(id, instance).into())?;
		Ok(handle)
	}

	/// Pauses all instances of this arrangement.
	pub fn pause(&mut self, settings: PauseInstanceSettings) -> Result<(), CommandError> {
		self.command_producer
			.push(InstanceCommand::PauseInstancesOf(self.id.into(), settings).into())
	}

	/// Resumes all instances of this arrangement.
	pub fn resume(&mut self, settings: ResumeInstanceSettings) -> Result<(), CommandError> {
		self.command_producer
			.push(InstanceCommand::ResumeInstancesOf(self.id.into(), settings).into())
	}

	/// Stops all instances of this arrangement.
	pub fn stop(&mut self, settings: StopInstanceSettings) -> Result<(), CommandError> {
		self.command_producer
			.push(InstanceCommand::StopInstancesOf(self.id.into(), settings).into())
	}
}
