//#![feature(conservative_impl_trait)]

use sample::{self, Frame, Sample};
use pitch_calc::{Letter, LetterOctave};
use std::collections::HashMap;
use bit_set::BitSet;
use core::clock::Seconds;
use num;
use std::iter::Iterator;
use frontend::audio::SoundEffect;
use std::f32;

const CHANNELS: usize = super::CHANNELS;

#[allow(unused)]
struct Signal<S, F> where S: num::Float {
	sample_rate: S,
	frames: Box<[F]>,
}

impl<S, F> Signal<S, F> where S: num::Float {
	fn len(&self) -> usize {
		self.frames.len()
	}
}

type StereoFrame = [f32; CHANNELS as usize];
type StereoSignal = Signal<f32, StereoFrame>;

// TODO: genericise this
struct Tone {
	pitch: f32,
	duration: Seconds,
	amplitude: f32,
}

#[allow(unused)]
enum Waveform {
	Sin,
	Square(f32),
	Silence,
}

impl Waveform {
	#[inline]
	fn sample(&self, amplitude: f32, duration: f32, t: f32, phase: f32) -> f32 {
		let envelope = num::clamp(1.0f32 - t / duration, 0.0f32, 1.0f32);
		let value = match self {
			&Waveform::Sin => (phase * f32::consts::PI * 2.0).sin(),
			&Waveform::Square(duty_cycle) => (phase - duty_cycle).signum(),
			_ => 0.0f32,
		};
		amplitude * envelope * value
	}

	#[allow(dead_code)]
	fn has_sample(&self, position: f32, duration: f32) -> bool {
		match self {
			&Waveform::Sin | &Waveform::Square(_) => position < duration,
			_ => false
		}
	}
}

struct Oscillator {
	tone: Tone,
	waveform: Waveform,
}

#[allow(unused)]
impl Oscillator {
	fn sin(letter_octave: LetterOctave, duration: Seconds, amplitude: f32) -> Oscillator {
		Oscillator { tone: Tone { pitch: letter_octave.hz(), duration, amplitude }, waveform: Waveform::Sin }
	}

	fn square(letter_octave: LetterOctave, duration: Seconds, amplitude: f32) -> Oscillator {
		Oscillator { tone: Tone { pitch: letter_octave.hz(), duration, amplitude }, waveform: Waveform::Square(0.5f32) }
	}

	fn silence() -> Oscillator {
		Oscillator { tone: Tone { pitch: 1.0f32, duration: Seconds::new(1.0f64), amplitude: 1.0f32 }, waveform: Waveform::Silence }
	}

	fn signal_function(self, pan: f32) -> Box<Fn(f32) -> StereoFrame> {
		let c_pan: StereoFrame = [1.0f32 - pan, pan];
		Box::new(move |t| {
			let val = self.sample(t);
			Frame::from_fn(|channel| (val * c_pan[channel]).to_sample())
		})
	}

	#[inline]
	fn sample(&self, t: f32) -> f32 {
		self.waveform.sample(self.tone.amplitude, self.tone.duration.into(), t, (t * self.tone.pitch).fract())
	}

	#[inline]
	fn duration(&self) -> Seconds {
		self.tone.duration
	}

	#[inline]
	#[allow(unused)]
	fn pitch(&self) -> f32 {
		self.tone.pitch
	}

	#[inline]
	fn has_sample(&self, t: f32) -> bool {
		self.waveform.has_sample(t, self.tone.duration.into())
	}
}

#[derive(Default, Clone)]
struct Voice {
	signal: Option<usize>,
	length: usize,
	position: usize,
}

impl Voice {
	fn new(signal_index: usize, length: usize) -> Self {
		Voice {
			signal: Some(signal_index),
			length,
			position: 0,
		}
	}

	fn remaining(&self) -> usize {
		self.length - self.position
	}

	fn advance(&mut self, l: usize) -> bool {
		self.position = usize::min(self.length, self.position + l);
		self.position >= self.length
	}
}

pub struct Multiplexer {
	#[allow(unused)]
	sample_rate: f64,
	sample_table: Vec<StereoSignal>,
	sample_map: HashMap<SoundEffect, usize>,
	voices: Vec<Voice>,
	playing_voice_index: BitSet,
	available_voice_index: Vec<usize>,
}

impl Multiplexer {
	pub fn new(sample_rate: f64, max_voices: usize) -> Multiplexer {
		let mut sample_table = Vec::new();
		let mut sample_map = HashMap::new();
		{
			let mut create_signal = |effect: SoundEffect, generator: Oscillator, pan: f32, delay: Seconds| {
				let duration = generator.tone.duration;
				let f: Box<Fn(f32) -> StereoFrame> = generator.signal_function(pan);
				let signal = Signal::new(sample_rate as f32, duration, f);
				let signal = signal.with_delay(delay, delay * 8.0, 1.0f32, 0.5f32);
				info!("Built signal for {:?} with {} samples", effect, signal.len());
				sample_table.push(signal);
				sample_map.insert(effect, sample_table.len() - 1);
			};

			create_signal(SoundEffect::Click(1), Oscillator::square(LetterOctave(Letter::G, 5), Seconds::new(0.1), 0.1f32), 0.8f32, Seconds::new(0.25));
			create_signal(SoundEffect::UserOption, Oscillator::square(LetterOctave(Letter::C, 6), Seconds::new(0.1), 0.1f32), 0.6f32, Seconds::new(0.25));
			create_signal(SoundEffect::Fertilised, Oscillator::sin(LetterOctave(Letter::C, 4), Seconds::new(0.3), 0.1f32), 0.6f32, Seconds::new(0.25));
			create_signal(SoundEffect::NewSpore, Oscillator::sin(LetterOctave(Letter::F, 5), Seconds::new(0.3), 0.1f32), 0.3f32, Seconds::new(0.33));
			create_signal(SoundEffect::NewMinion, Oscillator::sin(LetterOctave(Letter::A, 4), Seconds::new(0.5), 0.1f32), 0.55f32, Seconds::new(0.25));
			create_signal(SoundEffect::DieMinion, Oscillator::sin(LetterOctave(Letter::Eb, 3), Seconds::new(1.0), 0.2f32), 0.1f32, Seconds::new(0.5));
		}

		let voices = vec![Voice::default(); max_voices];
		let playing_voice_index = BitSet::with_capacity(max_voices);
		let available_voice_index = (0..max_voices).rev().collect();

		Multiplexer {
			sample_rate,
			sample_table,
			sample_map,
			voices,
			playing_voice_index,
			available_voice_index,
		}
	}

	fn free_voice(&mut self, voice_index: usize) {
		self.voices[voice_index].signal = None;
		self.playing_voice_index.remove(voice_index);
		self.available_voice_index.push(voice_index);
	}

	fn allocate_voice(&mut self, voice: Voice) -> Option<usize> {
		let allocated = self.available_voice_index.pop();
		if let Some(voice_index) = allocated {
			self.playing_voice_index.insert(voice_index);
			self.voices[voice_index] = voice;
		}
		allocated
	}

	pub fn audio_requested(&mut self, buffer: &mut [StereoFrame]) {
		sample::slice::equilibrium(buffer);
		let mut terminated_voices = BitSet::with_capacity(self.voices.len());
		for voice_index in &self.playing_voice_index {
			let voice = self.voices[voice_index].clone();
			if let Some(signal_index) = voice.signal {
				let frames = &self.sample_table[signal_index].frames[voice.position..];
				let len = buffer.len().min(voice.remaining());
				// TODO: how do we unroll this?
				for channel in 0..CHANNELS {
					for idx in 0..len {
						buffer[idx][channel] += frames[idx][channel];
					}
				}

				if self.voices[voice_index].advance(len) {
					// returns true on EOF
					terminated_voices.insert(voice_index);
				}
			}
		}
		for voice_index in &terminated_voices {
			self.free_voice(voice_index);
			info!("Voice {} stopped", voice_index);
		}
	}

	pub fn trigger(&mut self, effect: SoundEffect) {
		if let Some(signal_index) = self.sample_map.get(&effect).map(|t| *t) {
			let signal_length = self.sample_table[signal_index].len();
			if let Some(index) = self.allocate_voice(Voice::new(signal_index, signal_length)) {
				info!("Voice {} playing, {:?}", index, effect);
			}
		}
	}
}

#[allow(unused)]
impl<S, F> Signal<S, F> where S: num::Float + Into<f64> {
	fn new<V>(sample_rate: S, duration: Seconds, f: Box<V>) -> Signal<S, F>
		where V: Fn(S) -> F + ? Sized {
		let samples: usize = ((S::from(duration.get()).unwrap() * sample_rate).round().into()) as usize;
		let frames = (0..samples)
			.map(|i| S::from(i).unwrap() / sample_rate)
			.map(|t| f(t)).collect::<Vec<F>>();
		Signal {
			sample_rate,
			frames: frames.into_boxed_slice(),
		}
	}

	fn duration(&self) -> Seconds {
		Seconds::new(self.sample_rate.into() * self.frames.len() as f64)
	}

	fn sample_rate(&self) -> S {
		self.sample_rate
	}
}

impl<S, T> Signal<S, [T; CHANNELS]> where S: num::Float + Into<f64>, T: sample::Sample + num::Float {
	fn with_delay(self, time: Seconds, tail: Seconds, wet_dry: T, feedback: T) -> Self {
		use sample::Signal;
		use std::convert::Into;
		let wet_ratio = wet_dry;
		let dry_ratio: T = T::one() - wet_dry;
		let source_length = self.frames.len();
		let sample_rate: f64 = self.sample_rate.into();
		let delay_length = (<Seconds as Into<f64>>::into(time) * sample_rate).round() as usize;
		let tail_length = (<Seconds as Into<f64>>::into(tail) * sample_rate).round() as usize;
		let dest_length = source_length + tail_length;
		let mut delay_buffer: Vec<[T; CHANNELS]> = sample::signal::equilibrium().take(delay_length).collect();
		let mut dest_buffer: Vec<[T; CHANNELS]> = Vec::with_capacity(source_length + tail_length);

		for i in 0..dest_length {
			let tram_ptr = i % delay_length;
			let src = *self.frames.get(i).unwrap_or(&[T::zero(), T::zero()]);
			let delay_effect = delay_buffer[tram_ptr];
			let wet: [T; CHANNELS] = sample::Frame::from_fn(move |channel| { wet_ratio * src[channel] + delay_effect[channel] });
			dest_buffer.push(sample::Frame::from_fn(move |channel| { dry_ratio * src[channel] + wet[channel] }));
			delay_buffer[tram_ptr] = sample::Frame::from_fn(move |channel| { feedback * wet[CHANNELS - 1 - channel] }); // ping-pong
		}
		self::Signal {
			sample_rate: self.sample_rate,
			frames: dest_buffer.into_boxed_slice(),
		}
	}
}
