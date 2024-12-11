use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use miniaudio::{Device, DeviceConfig, DeviceType, Format, FramesMut, Waveform, WaveformConfig, WaveformType};
use std::io::Write;
use num_traits::real::Real;
use soundprog::math::frequency::EA440ChromaticScale;
use crate::miniaudio::wait_for_enter;

pub const DEVICE_FORMAT: Format = Format::F32;
pub const DEVICE_CHANNELS: u32 = 2;
pub const DEVICE_SAMPLE_RATE: u32 = miniaudio::SAMPLE_RATE_48000;
pub const SUBBUFFER_LEN: usize = 1024;
pub const SUBBUFFER_COUNT: usize = 16;

#[test]
fn test_miniaudio_playback_simple_thread() {
    let (send, recv) =
        miniaudio::ring_buffer::<f32>(SUBBUFFER_LEN, SUBBUFFER_COUNT).expect("Failed to create ring buffer");

    let shutdown_producer = Arc::new(AtomicBool::new(true));
    let shutdown_producer_check = Arc::clone(&shutdown_producer);

    let producer_thread = std::thread::spawn(move || {
        shutdown_producer_check.store(false, std::sync::atomic::Ordering::SeqCst);

        let sine_wave_config = WaveformConfig::new(
            DEVICE_FORMAT,
            DEVICE_CHANNELS,
            DEVICE_SAMPLE_RATE,
            WaveformType::Sine,
            0.2,
            440.0
        );
        let mut sine_wave = Waveform::new(&sine_wave_config);

        let mut now_time = 0.0;
        let advance_time = 1.0 / (DEVICE_SAMPLE_RATE as f64);

        let left_waveform_freq = EA440ChromaticScale::A4.to_frequency();
        let left_waveform_period = 1.0 / left_waveform_freq;

        let right_waveform_freq = EA440ChromaticScale::C5.to_frequency();
        let right_waveform_period = 1.0 / right_waveform_freq;

        loop {
            // 周期128サンプルの矩形波を出してみる。
            // とすれば、8周期になる。
            const MUL: f32 = 0.1f32; // -6dB想定。

            // writeするだけじゃダメ。
            // write_withにして寄せる必要があるのでは。(Frameとか)
            send.write_with(SUBBUFFER_LEN, |buf| {
                // bufはおそらく1024 * 4Bytesかと。
                // bufが理想的じゃない場合もあると思うので、いったん割ってみる。
                let frame_count = buf.len() / DEVICE_CHANNELS as usize;

                for frame_start in (0..buf.len()).step_by(2) {
                    let left_local_time = now_time % left_waveform_period;
                    let left_half_period = left_waveform_period * 0.5;
                    if left_local_time < left_half_period {
                        buf[frame_start + 0] = MUL;
                    }
                    else {
                        buf[frame_start + 0] = MUL * -1.0;
                    }

                    let right_local_time = now_time % right_waveform_period;
                    let right_half_period = right_waveform_period * 0.5;
                    if right_local_time < right_half_period {
                        buf[frame_start + 1] = MUL;
                    }
                    else {
                        buf[frame_start + 1] = MUL * -1.0;
                    }

                    now_time += advance_time;
                }
            });

            // We always just try to fill the entire buffer with samples:
            //send.write_with(SUBBUFFER_LEN, |buf| {
            //    sine_wave.read_pcm_frames(&mut FramesMut::wrap(
            //        buf,
            //        DEVICE_FORMAT,
            //        DEVICE_CHANNELS,
            //    ));
            //});

            if shutdown_producer_check.load(std::sync::atomic::Ordering::Acquire) {
                break;
            }
        }
    });

    // producerスレッドが動き出すまでには止める。
    while shutdown_producer.load(std::sync::atomic::Ordering::Acquire) {
        std::thread::yield_now();
    }

    let mut device_config = DeviceConfig::new(DeviceType::Playback);
    device_config.playback_mut().set_format(DEVICE_FORMAT);
    device_config.playback_mut().set_channels(DEVICE_CHANNELS);
    device_config.set_sample_rate(DEVICE_SAMPLE_RATE);

    let mut last_sample = 0.0f32;
    device_config.set_data_callback(move |_device, output, _input| {
        let samples = output.as_samples_mut::<f32>();

        // Here we try reading at most 8 subbuffers to attempt to read enough samples to
        // fill the playback output buffer. We don't allow infinite attempts because we can't be
        // sure how long that would take.
        let mut read_count = 0;
        let mut attempts = 0;
        while read_count < samples.len() && attempts < 8 {
            read_count += recv.read(&mut samples[read_count..]);
            attempts += 1;
        }

        // If we read anything, update the last sample.
        if read_count > 0 {
            last_sample = samples[read_count - 1];
        }

        // If we're starved, just repeat the last sample on all channels:
        (&mut samples[read_count..])
            .iter_mut()
            .for_each(|s| *s = 0.0);
    });
    device_config.set_stop_callback(|_| {
        println!("Stopped");
    });

    let device = Device::new(None, &device_config).expect("Failed to open device");
    device.start().expect("Failed to start device");

    println!("Device backend: {:?}", device.context().backend());
    println!("Device Sample Rate: {}Hz", device.sample_rate());
    wait_for_enter();

    shutdown_producer.store(true, std::sync::atomic::Ordering::Release);
    producer_thread.join().expect("Producer thread panicked");
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
