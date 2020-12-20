use crate::{
	arrangement::{Arrangement, ArrangementId},
	group::{Group, GroupId},
	instance::{
		InstanceId, InstanceSettings, PauseInstanceSettings, ResumeInstanceSettings,
		StopInstanceSettings,
	},
	mixer::{
		effect::{Effect, EffectId, EffectSettings},
		SubTrackId, TrackIndex, TrackSettings,
	},
	parameter::{ParameterId, Tween},
	pitch::Pitch,
	playable::Playable,
	sequence::{SequenceInstance, SequenceInstanceId},
	sound::{Sound, SoundId},
	tempo::Tempo,
	value::Value,
};

#[derive(Debug, Clone)]
pub(crate) enum ResourceCommand {
	AddSound(SoundId, Sound),
	RemoveSound(SoundId),
	AddArrangement(ArrangementId, Arrangement),
	RemoveArrangement(ArrangementId),
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum InstanceCommand {
	Play(
		InstanceId,
		Playable,
		Option<SequenceInstanceId>,
		InstanceSettings,
	),
	SetInstanceVolume(InstanceId, Value<f64>),
	SetInstancePitch(InstanceId, Value<Pitch>),
	SetInstancePanning(InstanceId, Value<f64>),
	SeekInstance(InstanceId, f64),
	SeekInstanceTo(InstanceId, f64),
	PauseInstance(InstanceId, PauseInstanceSettings),
	ResumeInstance(InstanceId, ResumeInstanceSettings),
	StopInstance(InstanceId, StopInstanceSettings),
	PauseInstancesOf(Playable, PauseInstanceSettings),
	ResumeInstancesOf(Playable, ResumeInstanceSettings),
	StopInstancesOf(Playable, StopInstanceSettings),
	PauseInstancesOfSequence(SequenceInstanceId, PauseInstanceSettings),
	ResumeInstancesOfSequence(SequenceInstanceId, ResumeInstanceSettings),
	StopInstancesOfSequence(SequenceInstanceId, StopInstanceSettings),
	PauseGroup(GroupId, PauseInstanceSettings),
	ResumeGroup(GroupId, ResumeInstanceSettings),
	StopGroup(GroupId, StopInstanceSettings),
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum MetronomeCommand {
	SetMetronomeTempo(Value<Tempo>),
	StartMetronome,
	PauseMetronome,
	StopMetronome,
}

pub(crate) enum SequenceCommand {
	StartSequenceInstance(SequenceInstanceId, SequenceInstance),
	MuteSequenceInstance(SequenceInstanceId),
	UnmuteSequenceInstance(SequenceInstanceId),
	PauseSequenceInstance(SequenceInstanceId),
	ResumeSequenceInstance(SequenceInstanceId),
	StopSequenceInstance(SequenceInstanceId),
	PauseGroup(GroupId),
	ResumeGroup(GroupId),
	StopGroup(GroupId),
}

#[derive(Debug)]
pub(crate) enum MixerCommand {
	AddSubTrack(SubTrackId, TrackSettings),
	RemoveSubTrack(SubTrackId),
	AddEffect(TrackIndex, EffectId, Box<dyn Effect>, EffectSettings),
	RemoveEffect(EffectId),
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum ParameterCommand {
	AddParameter(ParameterId, f64),
	RemoveParameter(ParameterId),
	SetParameter(ParameterId, f64, Option<Tween>),
}

#[derive(Debug, Clone)]
pub(crate) enum GroupCommand {
	AddGroup(GroupId, Group),
	RemoveGroup(GroupId),
}

pub(crate) enum Command {
	Resource(ResourceCommand),
	Instance(InstanceCommand),
	Metronome(MetronomeCommand),
	Sequence(SequenceCommand),
	Mixer(MixerCommand),
	Parameter(ParameterCommand),
	Group(GroupCommand),
}

impl From<ResourceCommand> for Command {
	fn from(command: ResourceCommand) -> Self {
		Self::Resource(command)
	}
}

impl From<InstanceCommand> for Command {
	fn from(command: InstanceCommand) -> Self {
		Self::Instance(command)
	}
}

impl From<MetronomeCommand> for Command {
	fn from(command: MetronomeCommand) -> Self {
		Self::Metronome(command)
	}
}

impl From<SequenceCommand> for Command {
	fn from(command: SequenceCommand) -> Self {
		Self::Sequence(command)
	}
}

impl From<MixerCommand> for Command {
	fn from(command: MixerCommand) -> Self {
		Self::Mixer(command)
	}
}

impl From<ParameterCommand> for Command {
	fn from(command: ParameterCommand) -> Self {
		Self::Parameter(command)
	}
}

impl From<GroupCommand> for Command {
	fn from(command: GroupCommand) -> Self {
		Self::Group(command)
	}
}
