use crate::{
	command::{sender::CommandSender, InstanceCommand},
	instance::{
		Instance, InstanceHandle, InstanceId, InstanceSettings, PauseInstanceSettings,
		ResumeInstanceSettings, StopInstanceSettings,
	},
	mixer::TrackId,
	AudioResult,
};

use super::SoundId;

#[derive(Clone)]
pub struct SoundHandle {
	id: SoundId,
	command_sender: CommandSender,
}

impl SoundHandle {
	pub(crate) fn new(id: SoundId, command_sender: CommandSender) -> Self {
		Self { id, command_sender }
	}

	pub fn id(&self) -> SoundId {
		self.id
	}

	pub fn duration(&self) -> f64 {
		self.id.duration()
	}

	pub fn default_track(&self) -> TrackId {
		self.id.default_track()
	}

	pub fn semantic_duration(&self) -> Option<f64> {
		self.id.semantic_duration()
	}

	pub fn default_loop_start(&self) -> Option<f64> {
		self.id.default_loop_start()
	}

	pub fn play(&mut self, settings: InstanceSettings) -> AudioResult<InstanceHandle> {
		let instance_id = InstanceId::new();
		let instance = Instance::new(self.id.into(), None, settings);
		let handle = InstanceHandle::new(
			instance_id,
			instance.public_state(),
			self.command_sender.clone(),
		);
		self.command_sender
			.push(InstanceCommand::Play(instance_id, instance).into())
			.map(|()| handle)
	}

	pub fn pause(&mut self, settings: PauseInstanceSettings) -> AudioResult<()> {
		self.command_sender
			.push(InstanceCommand::PauseInstancesOf(self.id.into(), settings).into())
	}

	pub fn resume(&mut self, settings: ResumeInstanceSettings) -> AudioResult<()> {
		self.command_sender
			.push(InstanceCommand::ResumeInstancesOf(self.id.into(), settings).into())
	}

	pub fn stop(&mut self, settings: StopInstanceSettings) -> AudioResult<()> {
		self.command_sender
			.push(InstanceCommand::StopInstancesOf(self.id.into(), settings).into())
	}
}
