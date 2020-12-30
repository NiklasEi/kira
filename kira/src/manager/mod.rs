//! Provides a bridge between the main thread and the audio thread.

mod backend;

#[cfg(not(feature = "benchmarking"))]
use backend::Backend;
#[cfg(feature = "benchmarking")]
pub use backend::Backend;

use std::{hash::Hash, unreachable};

#[cfg(feature = "serde_support")]
use bimap::BiMap;
use cpal::{
	traits::{DeviceTrait, HostTrait, StreamTrait},
	Stream,
};
use flume::{Receiver, Sender};

use crate::{
	arrangement::{Arrangement, ArrangementHandle, ArrangementId},
	command::{
		sender::CommandSender, Command, GroupCommand, MetronomeCommand, MixerCommand,
		ParameterCommand, ResourceCommand, SequenceCommand,
	},
	error::{AudioError, AudioResult},
	group::{Group, GroupHandle, GroupId},
	metronome::{Metronome, MetronomeHandle, MetronomeId, MetronomeSettings},
	mixer::{SubTrackId, Track, TrackHandle, TrackIndex, TrackSettings},
	parameter::{ParameterHandle, ParameterId},
	resource::Resource,
	sequence::{Sequence, SequenceInstanceHandle, SequenceInstanceId, SequenceInstanceSettings},
	sound::{Sound, SoundHandle, SoundId},
	util::index_set_from_vec,
};

/// Settings for an [`AudioManager`](crate::manager::AudioManager).
#[derive(Debug, Clone)]
pub struct AudioManagerSettings {
	/// The number of commands that be sent to the audio thread at a time.
	///
	/// Each action you take, like starting an instance or pausing a sequence,
	/// queues up one command.
	pub num_commands: usize,
	/// The maximum number of sounds that can be loaded at a time.
	pub num_sounds: usize,
	/// The maximum number of arrangements that can be loaded at a time.
	pub num_arrangements: usize,
	/// The maximum number of parameters that can exist at a time.
	pub num_parameters: usize,
	/// The maximum number of instances of sounds that can be playing at a time.
	pub num_instances: usize,
	/// The maximum number of sequences that can be running at a time.
	pub num_sequences: usize,
	/// The maximum number of mixer tracks that can be used at a time.
	pub num_tracks: usize,
	/// The maximum number of effects that can be running at a time on a mixer track.
	pub num_effects_per_track: usize,
	/// The maximum number of groups that can be used at a time.
	pub num_groups: usize,
	/// The maximum number of audio strams that can be used at a time.
	pub num_streams: usize,
	/// The maximum number of metronomes that can be used at a time.
	pub num_metronomes: usize,
}

impl Default for AudioManagerSettings {
	fn default() -> Self {
		Self {
			num_commands: 100,
			num_sounds: 100,
			num_arrangements: 100,
			num_parameters: 100,
			num_instances: 100,
			num_sequences: 25,
			num_tracks: 100,
			num_effects_per_track: 10,
			num_groups: 100,
			num_streams: 100,
			num_metronomes: 100,
		}
	}
}

/**
Plays and manages audio.

The audio manager is responsible for all communication between the gameplay thread
and the audio thread.
*/
pub struct AudioManager {
	quit_signal_sender: Sender<bool>,
	command_sender: CommandSender,
	resources_to_unload_receiver: Receiver<Resource>,
	// holds the stream if it has been created on the main thread
	// so it can live for as long as the audio manager
	_stream: Option<Stream>,
	#[cfg(feature = "serde_support")]
	sub_track_names: BiMap<String, SubTrackId>,
	#[cfg(feature = "serde_support")]
	group_names: BiMap<String, GroupId>,
}

impl AudioManager {
	/// Creates a new audio manager and starts an audio thread.
	pub fn new(settings: AudioManagerSettings) -> AudioResult<Self> {
		let (
			quit_signal_sender,
			command_sender,
			resources_to_unload_receiver,
			command_receiver,
			unloader,
			quit_signal_receiver,
		) = Self::create_thread_channels(&settings);

		#[cfg(not(target_arch = "wasm32"))]
		let stream = {
			const WRAPPER_THREAD_SLEEP_DURATION: f64 = 1.0 / 60.0;

			let (setup_result_sender, setup_result_receiver) = flume::bounded(1);
			// set up a cpal stream on a new thread. we could do this on the main thread,
			// but that causes issues with LÖVE.
			std::thread::spawn(move || {
				match Self::setup_stream(settings, command_receiver, unloader) {
					Ok(_stream) => {
						setup_result_sender.try_send(Ok(())).unwrap();
						// wait for a quit message before ending the thread and dropping
						// the stream
						while quit_signal_receiver.try_recv().is_err() {
							std::thread::sleep(std::time::Duration::from_secs_f64(
								WRAPPER_THREAD_SLEEP_DURATION,
							));
						}
					}
					Err(error) => {
						setup_result_sender.try_send(Err(error)).unwrap();
					}
				}
			});
			// wait for the audio thread to report back a result
			loop {
				// TODO: figure out if we need to handle
				// TryRecvError::Disconnected
				if let Ok(result) = setup_result_receiver.try_recv() {
					match result {
						Ok(_) => break,
						Err(error) => return Err(error),
					}
				}
			}

			None
		};

		#[cfg(target_arch = "wasm32")]
		let stream = {
			// the quit signal is not meant to be consumed on wasm
			let _ = quit_signal_receiver;
			Some(Self::setup_stream(settings, command_receiver, unloader)?)
		};

		Ok(Self {
			quit_signal_sender,
			command_sender,
			resources_to_unload_receiver,
			_stream: stream,
			#[cfg(feature = "serde_support")]
			sub_track_names: BiMap::new(),
			#[cfg(feature = "serde_support")]
			group_names: BiMap::new(),
		})
	}

	fn create_thread_channels(
		settings: &AudioManagerSettings,
	) -> (
		Sender<bool>,
		CommandSender,
		Receiver<Resource>,
		Receiver<Command>,
		Sender<Resource>,
		Receiver<bool>,
	) {
		let (quit_signal_sender, quit_signal_receiver) = flume::bounded(1);
		let (command_sender, command_receiver) = flume::bounded(settings.num_commands);
		// TODO: add a setting or constant for max number of resources to unload
		let (unloader, resources_to_unload_receiver) = flume::bounded(10);
		(
			quit_signal_sender,
			CommandSender::new(command_sender),
			resources_to_unload_receiver,
			command_receiver,
			unloader,
			quit_signal_receiver,
		)
	}

	fn setup_stream(
		settings: AudioManagerSettings,
		command_receiver: Receiver<Command>,
		unloader: Sender<Resource>,
	) -> AudioResult<Stream> {
		let host = cpal::default_host();
		let device = host
			.default_output_device()
			.ok_or(AudioError::NoDefaultOutputDevice)?;
		let config = device.default_output_config()?.config();
		let sample_rate = config.sample_rate.0;
		let channels = config.channels;
		let mut backend = Backend::new(sample_rate, settings, command_receiver, unloader);
		let stream = device.build_output_stream(
			&config,
			move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
				for frame in data.chunks_exact_mut(channels as usize) {
					let out = backend.process();
					if channels == 1 {
						frame[0] = (out.left + out.right) / 2.0;
					} else {
						frame[0] = out.left;
						frame[1] = out.right;
					}
				}
			},
			move |_| {},
		)?;
		stream.play()?;
		Ok(stream)
	}

	#[cfg(feature = "benchmarking")]
	/// Creates an [`AudioManager`] and [`Backend`] without sending
	/// the backend to another thread.
	///
	/// This is useful for updating the backend manually for
	/// benchmarking.
	pub fn new_without_audio_thread(
		settings: AudioManagerSettings,
	) -> AudioResult<(Self, Backend)> {
		const SAMPLE_RATE: u32 = 48000;
		let (
			quit_signal_sender,
			command_sender,
			resources_to_unload_receiver,
			command_receiver,
			unloader,
			_,
		) = Self::create_thread_channels(&settings);
		let audio_manager = Self {
			quit_signal_sender,
			command_sender,
			resources_to_unload_receiver,
			_stream: None,
			#[cfg(feature = "serde_support")]
			sub_track_names: BiMap::new(),
			#[cfg(feature = "serde_support")]
			group_names: BiMap::new(),
		};
		let backend = Backend::new(SAMPLE_RATE, settings, command_receiver, unloader);
		Ok((audio_manager, backend))
	}

	/// Sends a sound to the audio thread and returns a handle to the sound.
	pub fn add_sound(&mut self, sound: Sound) -> AudioResult<SoundHandle> {
		let id = SoundId::new(&sound);
		self.command_sender
			.push(ResourceCommand::AddSound(id, sound).into())?;
		Ok(SoundHandle::new(id, self.command_sender.clone()))
	}

	/// Loads a sound from a file and returns a handle to the sound.
	///
	/// This is a shortcut for constructing the sound manually and adding it
	/// using [`AudioManager::add_sound`].
	#[cfg(any(feature = "mp3", feature = "ogg", feature = "flac", feature = "wav"))]
	pub fn load_sound<P: AsRef<std::path::Path>>(
		&mut self,
		path: P,
		settings: crate::playable::PlayableSettings,
	) -> AudioResult<SoundHandle> {
		let sound = Sound::from_file(path, settings)?;
		self.add_sound(sound)
	}

	pub fn remove_sound(&mut self, id: impl Into<SoundId>) -> AudioResult<()> {
		self.command_sender
			.push(ResourceCommand::RemoveSound(id.into()).into())
	}

	/// Sends a arrangement to the audio thread and returns a handle to the arrangement.
	pub fn add_arrangement(&mut self, arrangement: Arrangement) -> AudioResult<ArrangementHandle> {
		let id = ArrangementId::new(&arrangement);
		self.command_sender
			.push(ResourceCommand::AddArrangement(id, arrangement).into())?;
		Ok(ArrangementHandle::new(id, self.command_sender.clone()))
	}

	pub fn remove_arrangement(&mut self, id: impl Into<ArrangementId>) -> AudioResult<()> {
		self.command_sender
			.push(ResourceCommand::RemoveArrangement(id.into()).into())
	}

	/// Frees resources that are no longer in use, such as unloaded sounds
	/// or finished sequences.
	pub fn free_unused_resources(&mut self) {
		for resource in self.resources_to_unload_receiver.try_iter() {
			println!(
				"{}",
				match resource {
					Resource::Sound(_) => "Sound",
					Resource::Arrangement(_) => "Arrangement",
					Resource::SequenceInstance(_) => "SequenceInstance",
					Resource::Track(_) => "Track",
					Resource::EffectSlot(_) => "EffectSlot",
					Resource::Group(_) => "Group",
					Resource::Stream(_) => "Stream",
					Resource::Metronome(_) => "Metronome",
				}
			)
		}
	}

	pub fn add_metronome(&mut self, settings: MetronomeSettings) -> AudioResult<MetronomeHandle> {
		let id = MetronomeId::new();
		let (event_sender, event_receiver) = flume::bounded(settings.event_queue_capacity);
		let metronome = Metronome::new(settings, event_sender);
		self.command_sender
			.push(MetronomeCommand::AddMetronome(id, metronome).into())
			.map(|_| MetronomeHandle::new(id, self.command_sender.clone(), event_receiver))
	}

	pub fn remove_metronome(&mut self, id: impl Into<MetronomeId>) -> AudioResult<()> {
		self.command_sender
			.push(MetronomeCommand::RemoveMetronome(id.into()).into())
	}

	/// Starts a sequence.
	pub fn start_sequence<CustomEvent: Clone + Eq + Hash>(
		&mut self,
		sequence: Sequence<CustomEvent>,
		settings: SequenceInstanceSettings,
	) -> Result<SequenceInstanceHandle<CustomEvent>, AudioError> {
		sequence.validate()?;
		let id = SequenceInstanceId::new();
		let (instance, handle) =
			sequence.create_instance(settings, id, self.command_sender.clone());
		self.command_sender
			.push(SequenceCommand::StartSequenceInstance(id, instance).into())?;
		Ok(handle)
	}

	/// Creates a parameter with the specified starting value.
	pub fn add_parameter(&mut self, value: f64) -> AudioResult<ParameterHandle> {
		let id = ParameterId::new();
		self.command_sender
			.push(ParameterCommand::AddParameter(id, value).into())?;
		Ok(ParameterHandle::new(id, self.command_sender.clone()))
	}

	pub fn remove_parameter(&mut self, id: impl Into<ParameterId>) -> AudioResult<()> {
		self.command_sender
			.push(ParameterCommand::RemoveParameter(id.into()).into())
	}

	/// Creates a mixer sub-track.
	pub fn add_sub_track(&mut self, settings: TrackSettings) -> AudioResult<TrackHandle> {
		let id = SubTrackId::new();
		self.command_sender
			.push(MixerCommand::AddSubTrack(id, Track::new(settings)).into())?;
		Ok(TrackHandle::new(id.into(), self.command_sender.clone()))
	}

	/// Creates a mixer sub-track and assigns it a name.
	#[cfg(feature = "serde_support")]
	pub fn add_named_sub_track(
		&mut self,
		name: impl Into<String>,
		settings: TrackSettings,
	) -> AudioResult<TrackHandle> {
		let handle = self.add_sub_track(settings)?;
		match handle.index() {
			TrackIndex::Main => unreachable!(),
			TrackIndex::Sub(id) => {
				self.sub_track_names.insert(name.into(), id);
			}
		}
		Ok(handle)
	}

	/// Removes a sub-track from the mixer.
	pub fn remove_sub_track(&mut self, id: SubTrackId) -> AudioResult<()> {
		#[cfg(feature = "serde_support")]
		self.sub_track_names.remove_by_right(&id);
		self.command_sender
			.push(MixerCommand::RemoveSubTrack(id.into()).into())
	}

	/// Adds a group.
	pub fn add_group(
		&mut self,
		parent_groups: impl Into<Vec<GroupId>>,
	) -> AudioResult<GroupHandle> {
		let id = GroupId::new();
		let group = Group::new(index_set_from_vec(parent_groups.into()));
		self.command_sender
			.push(GroupCommand::AddGroup(id, group).into())?;
		Ok(GroupHandle::new(id, self.command_sender.clone()))
	}

	/// Adds a group and assigns it a name.
	#[cfg(feature = "serde_support")]
	pub fn add_named_group(
		&mut self,
		name: impl Into<String>,
		parent_groups: impl Into<Vec<GroupId>>,
	) -> AudioResult<GroupHandle> {
		let handle = self.add_group(parent_groups)?;
		self.group_names.insert(name.into(), handle.id());
		Ok(handle)
	}

	/// Removes a group.
	pub fn remove_group(&mut self, id: impl Into<GroupId>) -> AudioResult<()> {
		let id: GroupId = id.into();
		#[cfg(feature = "serde_support")]
		self.group_names.remove_by_right(&id);
		self.command_sender
			.push(GroupCommand::RemoveGroup(id).into())
	}

	#[cfg(feature = "serde_support")]
	pub fn load_playable_settings(
		&self,
		settings: crate::playable::SerializablePlayableSettings,
	) -> AudioResult<crate::playable::PlayableSettings> {
		Ok(crate::playable::PlayableSettings {
			default_track: match settings.default_track {
				Some(name) => match self.sub_track_names.get_by_left(&name) {
					Some(id) => TrackIndex::Sub(*id),
					None => return Err(AudioError::NoTrackWithName(name)),
				},
				None => TrackIndex::Main,
			},
			cooldown: settings.cooldown,
			semantic_duration: settings.semantic_duration,
			default_loop_start: settings.default_loop_start,
			groups: match settings.groups {
				Some(group_names) => {
					let mut group_ids = vec![];
					for name in group_names {
						match self.group_names.get_by_left(&name) {
							Some(id) => {
								group_ids.push(*id);
							}
							None => return Err(AudioError::NoGroupWithName(name)),
						}
					}
					group_ids
				}
				None => vec![],
			},
		})
	}
}

impl Drop for AudioManager {
	fn drop(&mut self) {
		// TODO: add proper error handling here without breaking benchmarks
		self.quit_signal_sender.send(true).ok();
	}
}
