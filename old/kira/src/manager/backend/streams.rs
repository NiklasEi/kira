use crate::{
	audio_stream::{AudioStream, AudioStreamId},
	command::StreamCommand,
	manager::TrackIndex,
	resource::Resource,
};

use super::mixer::Mixer;

use flume::Sender;
use indexmap::IndexMap;

pub(crate) struct Streams {
	streams: IndexMap<AudioStreamId, (TrackIndex, Box<dyn AudioStream>)>,
}

impl Streams {
	pub fn new(capacity: usize) -> Self {
		Self {
			streams: IndexMap::with_capacity(capacity),
		}
	}

	pub fn run_command(&mut self, command: StreamCommand, unloader: &mut Sender<Resource>) {
		match command {
			StreamCommand::AddStream(stream_id, track_id, stream) => {
				self.streams.insert(stream_id, (track_id, stream));
			}
			StreamCommand::RemoveStream(stream_id) => {
				if let Some((_, stream)) = self.streams.remove(&stream_id) {
					unloader.try_send(Resource::Stream(stream)).ok();
				}
			}
		}
	}

	pub fn process(&mut self, dt: f64, mixer: &mut Mixer) {
		for (track, stream) in self.streams.values_mut() {
			mixer.add_input(*track, stream.next(dt));
		}
	}
}
