use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::thread;
use cpal::{BufferSize, Device, Stream, SampleFormat, StreamConfig, SupportedBufferSize};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use creak::Decoder;


pub struct AudioStreamManager {
    device: Device,
    stream: Stream,
    samples: Arc<Mutex<AudioBufferTracker>>,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
    buffer_size: BufferSize,
}

unsafe impl Send for AudioStreamManager {}
unsafe impl Sync for AudioStreamManager {}

struct AudioBufferTracker {
    samples: Vec<f32>,
    position: usize
}

impl AudioStreamManager {
    pub fn from_file(file_path: &str) -> Result<Self, AudioError> {
        let decoder = Decoder::open(&file_path).ok().ok_or(AudioError::FileError)?;

        let sample_rate = decoder.info().sample_rate();

        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::DeviceError)?;
        let format = StreamConfig {
            channels: device.default_output_config().ok().ok_or(AudioError::DeviceError)?.channels(),
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: BufferSize::Default
        };
        let config = device.default_output_config().ok().ok_or(AudioError::DeviceError)?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();
        let buffer_size = match config.buffer_size() {
            SupportedBufferSize::Range { min, max} => BufferSize::Fixed(*max),
            SupportedBufferSize::Unknown => BufferSize::Default
        };

        let file_channels = decoder.info().channels();
        let device_channels = format.channels;

        let mut samples: Vec<f32> = Vec::new();
        let samples_file = match decoder.into_samples() {
            Ok(samples) => samples,
            Err(_) => return Err(AudioError::FileError),
        };
        for sample in samples_file {
            samples.push(match sample {
                Ok(sample) => sample,
                Err(_) => return Err(AudioError::FileError),
            })
        };
        let audio_duration = *(&samples.len()) as f32 / sample_rate as f32;

        // Upmix / Downmix the samples to match the device's channel count, if necessary
        // right now, only mono to stereo and stereo to mono are supported
        // TODO: add support for other channel counts
        // currently returns an error if the channel count conversion is not supported
        if (file_channels == 1) && (device_channels == 2) {
            samples = upmix_mono_to_stereo(&samples);
        } else if (file_channels == 2) && (device_channels == 1) {
            samples = downmix_stereo_to_mono(samples);
        } else if file_channels != device_channels as usize {
            return Err(AudioError::ConversionError);
        }

        let samples_shared = Arc::new(Mutex::new(AudioBufferTracker {
            samples,
            position: 0
        }));
        let samples_clone = Arc::clone(&samples_shared);

        let stream = build_stream(sample_format, &format, &samples_shared, &device)?;

        Ok(AudioStreamManager {
            device,
            stream,
            sample_rate,
            channels,
            sample_format,
            buffer_size,
            samples: samples_shared,
        })
    }
    pub fn play(&mut self) -> Result<(), AudioError> {
        match self.stream.play() {
            Ok(_) => Ok(()),
            Err(_) => Err(AudioError::StreamError),
        }
    }

    pub fn pause(&mut self) -> Result<(), AudioError> {
        match self.stream.pause() {
            Ok(_) => Ok(()),
            Err(_) => Err(AudioError::StreamError),
        }
    }

    pub fn set_time(&mut self, time: f32) -> Result<(), AudioError> {
        let sample_index = self.get_sample_index_from_time(time as f32);
        match self.samples.lock() {
            Ok(mut samples) => samples.position = sample_index,
            Err(poisoned) => {
                let mut samples = poisoned.into_inner();
                samples.position = sample_index;
                // Handle the error here, e.g., log it or return an Err variant
                // TODO
            }
        }
        Ok(())
    }

    pub fn skip_time(&mut self, time_to_skip: i64) -> Result<(), AudioError> {
        let current_time = self.get_time().ok().ok_or(AudioError::StreamError)? as f32;
        let new_time = current_time + time_to_skip as f32;
        self.set_time(new_time)
    }

    fn get_sample_index_from_time(&self, time: f32) -> usize {
        (time* self.sample_rate as f32) as usize
    }

    fn get_time_from_sample_index(&self, sample_index: usize) -> f32 {
        sample_index as f32 / self.sample_rate as f32
    }

    pub fn get_time(&self) -> Result<f32, AudioError> {
        let samples = self.samples.lock().ok().ok_or(AudioError::StreamError)?;
        Ok(self.get_time_from_sample_index(samples.position))
    }

    pub fn current_sample_index(&self) -> Result<usize, AudioError> {
        let samples = self.samples.lock().ok().ok_or(AudioError::StreamError)?;
        Ok(samples.position)
    }
}

#[derive(Debug)]
pub enum AudioError {
    FileError,
    DeviceError,
    StreamError,
    ConversionError,
}

impl Display for AudioError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::ConversionError => write!(f, "ConversionError"),
            AudioError::FileError => write!(f, "FileError"),
            AudioError::DeviceError => write!(f, "DeviceError"),
            AudioError::StreamError => write!(f, "StreamError"),
        }
    }
}

impl Error for AudioError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

fn upmix_mono_to_stereo(mono: &Vec<f32>) -> Vec<f32> {
    let mut stereo = Vec::new();
    for sample in mono.iter() {
        stereo.push(*sample);
        stereo.push(*sample);
    }
    stereo
}

fn downmix_stereo_to_mono(stereo: Vec<f32>) -> Vec<f32> {
    let mut mono = Vec::new();
    for i in 0..stereo.len() / 2 {
        let left = stereo[i * 2];
        let right = stereo[i * 2 + 1];
        mono.push((left + right) / 2.0);
    }
    mono
}

fn fill_buffer<T>(
    data: &mut [T],
    samples_track: &mut AudioBufferTracker,
    convert_fn: impl Fn(f32) -> T
) -> ()
{
    let samples = &mut samples_track.samples;
    let position = &mut samples_track.position;

    for sample in data.iter_mut() {
        if *position < samples.len() {
            *sample = convert_fn(samples[*position]);
            *position += 1;
        } else {
            *sample = convert_fn(0.0); // End of samples, fill with silence
        }
    }
}

fn build_stream(
    sample_format: SampleFormat,
    audio_format: &StreamConfig,
    sample_track: &Arc<Mutex<AudioBufferTracker>>,
    device: &Device
) -> Result<cpal::Stream, AudioError>
{
    let error_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let samples_clone = Arc::clone(&sample_track);

    let stream = match sample_format {
        SampleFormat::I16 => device.build_output_stream(
            &audio_format,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| (sample * i16::MAX as f32) as i16);
            },
            error_fn,
            None
        ),
        SampleFormat::U16 => device.build_output_stream(
            &audio_format,
            move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| (sample * u16::MAX as f32) as u16);
            },
            error_fn,
            None
        ),
        SampleFormat::F32 => device.build_output_stream(
            &audio_format,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| sample);
            },
            error_fn,
            None
        ),
        SampleFormat::I8 => device.build_output_stream(
            &audio_format,
            move |data: &mut [i8], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| (sample * i8::MAX as f32) as i8);
            },
            error_fn,
            None
        ),
        SampleFormat::U8 => device.build_output_stream(
            &audio_format,
            move |data: &mut [u8], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| (sample * u8::MAX as f32) as u8);
            },
            error_fn,
            None
        ),
        SampleFormat::F64 => device.build_output_stream(
            &audio_format,
            move |data: &mut [f64], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| sample as f64);
            },
            error_fn,
            None
        ),
        SampleFormat::I32 => device.build_output_stream(
            &audio_format,
            move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| (sample * i32::MAX as f32) as i32);
            },
            error_fn,
            None
        ),
        SampleFormat::U32 => device.build_output_stream(
            &audio_format,
            move |data: &mut [u32], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| (sample * u32::MAX as f32) as u32);
            },
            error_fn,
            None
        ),
        SampleFormat::I64 => device.build_output_stream(
            &audio_format,
            move |data: &mut [i64], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| (sample * i64::MAX as f32) as i64);
            },
            error_fn,
            None
        ),
        SampleFormat::U64 => device.build_output_stream(
            &audio_format,
            move |data: &mut [u64], _: &cpal::OutputCallbackInfo| {
                let mut samples_and_pos = samples_clone.lock().unwrap();
                fill_buffer(data, &mut *samples_and_pos, |sample| (sample * u64::MAX as f32) as u64);
            },
            error_fn,
            None
        ),
        _ => return Err(AudioError::StreamError)
    };

    let stream = match stream {
        Ok(stream) => stream,
        Err(_) => return Err(AudioError::StreamError)
    };

    Ok(stream)
}
