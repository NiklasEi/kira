use std::f32::consts::PI;

use criterion::{criterion_group, criterion_main, Criterion};
use kira::{instance::InstanceSettings, manager::AudioManager, sound::Sound, StereoSample};

fn create_test_sound(num_samples: usize) -> Sound {
	const SAMPLE_RATE: u32 = 48000;
	let mut sine_samples = vec![];
	let mut phase = 0.0;
	for _ in 0..num_samples {
		sine_samples.push(StereoSample::from_mono((phase * 2.0 * PI).sin()));
		phase += 440.0 / SAMPLE_RATE as f32;
	}
	Sound::new(SAMPLE_RATE, sine_samples, Default::default())
}

fn instances_benchmark(c: &mut Criterion) {
	let (mut audio_manager, mut backend) =
		AudioManager::<()>::new_without_audio_thread(Default::default()).unwrap();
	let sound_id = audio_manager.add_sound(create_test_sound(48000)).unwrap();
	backend.process();
	for _ in 0..100 {
		audio_manager
			.play_sound(sound_id, InstanceSettings::new().loop_region(..))
			.unwrap();
	}
	c.bench_function("instances", |b| b.iter(|| backend.process()));
}

criterion_group!(benches, instances_benchmark);
criterion_main!(benches);
