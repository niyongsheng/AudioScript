//! Windows 麦克风回退：设备原生格式超过两声道时 flexaudio 会直接拒绝打开，
//! 这里按原生格式采集、下混成单声道并重采样到 ASR 需要的 16 kHz。
//!
//! 回调只做下混并写入无锁环形缓冲，不分配、不加锁、不做 IO；消费侧在工作线程完成重采样与混音。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use flexaudio::{OutputFormat, SourceKind, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Indexing, Resampler as _, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

use super::{
    mix_mic_and_system, open_started, AudioError, AudioSourceKind, CaptureHandle, DeviceSelection,
    CHUNK_SAMPLES, POLL_IDLE,
};
use crate::asr::TARGET_SAMPLE_RATE;
use crate::i18n::{tr, Key};

const RING_SECONDS: usize = 2;

const OVERFLOW_LOG_INTERVAL: Duration = Duration::from_secs(5);

pub(super) fn probe(kind: AudioSourceKind, selection: &DeviceSelection) -> Option<String> {
    let wanted = match kind {
        AudioSourceKind::Mic => selection.device.as_deref(),
        AudioSourceKind::Mix => selection.mix_mic.as_deref(),
        AudioSourceKind::Loopback => return None,
    };

    let host = cpal::default_host();
    let device = find_device(&host, wanted)?;
    let config = device.default_input_config().ok()?;
    if config.channels() <= 2 {
        return None;
    }

    device.name().ok()
}

pub(super) fn start<S>(
    kind: AudioSourceKind,
    selection: &DeviceSelection,
    mic_name: String,
    sink: S,
) -> Result<CaptureHandle, AudioError>
where
    S: FnMut(&[f32]) + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let mut mic_selector = if kind == AudioSourceKind::Mix {
        selection.mix_mic.clone()
    } else {
        selection.device.clone()
    };
    let system_name = (kind == AudioSourceKind::Mix).then(|| {
        super::resolve_device_name(AudioSourceKind::Loopback, selection.mix_system.as_deref())
    });
    let device_name = match &system_name {
        Some(system) => format!("{mic_name} + {system}"),
        None => mic_name,
    };
    let system_device = selection.mix_system.clone();

    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel::<Result<u32, AudioError>>(1);
    let stop_for_worker = Arc::clone(&stop);
    let thread = std::thread::Builder::new()
        .name("audio-worker".into())
        .spawn(move || {
            if kind == AudioSourceKind::Mix {
                run_mix(
                    mic_selector.take(),
                    system_device,
                    stop_for_worker,
                    sink,
                    &ready_tx,
                );
            } else {
                run_mic(mic_selector.take(), stop_for_worker, sink, &ready_tx);
            }
        })
        .map_err(|e| AudioError::Start(e.to_string()))?;

    match ready_rx.recv() {
        Ok(Ok(sample_rate)) => Ok(CaptureHandle {
            stop,
            thread: Some(thread),
            device_sample_rate: sample_rate,
            device_name,
        }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => {
            let _ = thread.join();
            Err(AudioError::Start("麦克风采集线程提前退出".into()))
        }
    }
}

fn run_mic<S>(
    mic_selector: Option<String>,
    stop: Arc<AtomicBool>,
    mut sink: S,
    ready: &std::sync::mpsc::SyncSender<Result<u32, AudioError>>,
) where
    S: FnMut(&[f32]) + Send,
{
    if let Err(e) = mic_pipeline(mic_selector.as_deref(), &stop, &mut sink, ready) {
        tracing::error!("麦克风采集终止：{e}");
        let _ = ready.send(Err(e));
    }
    tracing::info!("采集工作线程退出");
}

fn mic_pipeline<S>(
    mic_selector: Option<&str>,
    stop: &AtomicBool,
    sink: &mut S,
    ready: &std::sync::mpsc::SyncSender<Result<u32, AudioError>>,
) -> Result<(), AudioError>
where
    S: FnMut(&[f32]) + Send,
{
    let mut capture = open_mic(mic_selector)?;
    let mut resampler = make_resampler(capture.native_rate)?;
    let _ = ready.send(Ok(capture.native_rate));

    let mut pending: VecDeque<f32> = VecDeque::with_capacity(CHUNK_SAMPLES * 4);
    let mut raw: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES);
    let mut last_overflow = Instant::now();

    while !stop.load(Ordering::SeqCst) {
        let had_samples = drain_samples(&mut capture, &mut raw, &mut resampler, &mut pending)?;
        report_overflow(&capture.overflow, &mut last_overflow);
        if let Some(error) = take_error(&capture.errors) {
            return Err(AudioError::Start(format!("麦克风采集中断：{error}")));
        }
        while pending.len() >= CHUNK_SAMPLES {
            let chunk: Vec<f32> = pending.drain(..CHUNK_SAMPLES).collect();
            sink(&chunk);
        }
        trim(&mut pending);
        if !had_samples {
            std::thread::sleep(POLL_IDLE);
        }
    }

    if let Some(rs) = &mut resampler {
        if let Ok(tail) = rs.flush() {
            pending.extend(tail);
        }
    }
    if !pending.is_empty() {
        let chunk: Vec<f32> = pending.drain(..).collect();
        sink(&chunk);
    }
    Ok(())
}

fn run_mix<S>(
    mic_selector: Option<String>,
    system_device: Option<String>,
    stop: Arc<AtomicBool>,
    mut sink: S,
    ready: &std::sync::mpsc::SyncSender<Result<u32, AudioError>>,
) where
    S: FnMut(&[f32]) + Send,
{
    if let Err(e) = mix_pipeline(
        mic_selector.as_deref(),
        system_device.as_deref(),
        &stop,
        &mut sink,
        ready,
    ) {
        tracing::error!("混音采集终止：{e}");
        let _ = ready.send(Err(e));
    }
    tracing::info!("采集工作线程退出");
}

fn mix_pipeline<S>(
    mic_selector: Option<&str>,
    system_device: Option<&str>,
    stop: &AtomicBool,
    sink: &mut S,
    ready: &std::sync::mpsc::SyncSender<Result<u32, AudioError>>,
) -> Result<(), AudioError>
where
    S: FnMut(&[f32]) + Send,
{
    let mut capture = open_mic(mic_selector)?;

    let system_config = StreamConfig {
        kind: SourceKind::SystemLoopback,
        device_id: system_device.map(str::to_owned),
        exclude_self: true,
        output: OutputFormat {
            sample_rate: TARGET_SAMPLE_RATE,
            channels: 1,
        },
        ..Default::default()
    };
    let mut system = open_started(&system_config)?;
    let system_rate = system.native_format().0;

    let mut resampler = make_resampler(capture.native_rate)?;
    let _ = ready.send(Ok(system_rate));

    let mut mic_buf: VecDeque<f32> = VecDeque::with_capacity(CHUNK_SAMPLES * 4);
    let mut system_buf: VecDeque<f32> = VecDeque::with_capacity(CHUNK_SAMPLES * 4);
    let mut raw: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES);
    let mut last_overflow = Instant::now();

    while !stop.load(Ordering::SeqCst) {
        let mut had_samples = false;
        while let Some(chunk) = system.poll_chunk() {
            system_buf.extend(chunk.data);
            had_samples = true;
        }
        if let Some(error) = system.terminal_error() {
            return Err(AudioError::Start(format!("系统声音采集中断：{error}")));
        }

        had_samples |= drain_samples(&mut capture, &mut raw, &mut resampler, &mut mic_buf)?;
        report_overflow(&capture.overflow, &mut last_overflow);
        if let Some(error) = take_error(&capture.errors) {
            return Err(AudioError::Start(format!("麦克风采集中断：{error}")));
        }

        while system_buf.len() >= CHUNK_SAMPLES || mic_buf.len() >= CHUNK_SAMPLES {
            let mixed = mix_mic_and_system(&mut mic_buf, &mut system_buf, CHUNK_SAMPLES);
            sink(&mixed);
        }
        trim(&mut mic_buf);
        trim(&mut system_buf);
        if !had_samples {
            std::thread::sleep(POLL_IDLE);
        }
    }

    while let Some(chunk) = system.poll_chunk() {
        system_buf.extend(chunk.data);
    }
    if let Some(rs) = &mut resampler {
        if let Ok(tail) = rs.flush() {
            mic_buf.extend(tail);
        }
    }
    while !mic_buf.is_empty() || !system_buf.is_empty() {
        let frames = CHUNK_SAMPLES.min(mic_buf.len().max(system_buf.len()));
        let mixed = mix_mic_and_system(&mut mic_buf, &mut system_buf, frames);
        sink(&mixed);
    }

    system.stop();
    Ok(())
}

struct MicCapture {
    native_rate: u32,
    _stream: cpal::Stream,
    samples: Consumer<f32>,
    overflow: Arc<AtomicU64>,
    errors: Arc<Mutex<Option<String>>>,
}

fn open_mic(selector: Option<&str>) -> Result<MicCapture, AudioError> {
    let host = cpal::default_host();
    let device = find_device(&host, selector).ok_or_else(|| {
        AudioError::DeviceNotFound(
            selector
                .map(str::to_owned)
                .unwrap_or_else(|| tr(Key::AudioDefaultDevice).to_string()),
        )
    })?;
    let supported = device
        .default_input_config()
        .map_err(|e| AudioError::Open(format!("读取麦克风格式失败：{e}")))?;
    let native_rate = supported.sample_rate().0;

    let (producer, consumer) = RingBuffer::<f32>::new(native_rate as usize * RING_SECONDS);
    let overflow = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(Mutex::new(None));
    let stream = build_stream(
        &device,
        &supported,
        producer,
        Arc::clone(&overflow),
        Arc::clone(&errors),
    )?;
    stream
        .play()
        .map_err(|e| AudioError::Start(format!("启动麦克风采集失败：{e}")))?;

    let device_name = device.name().unwrap_or_default();
    tracing::info!(
        device = %device_name,
        native_rate,
        native_channels = supported.channels(),
        target_rate = TARGET_SAMPLE_RATE,
        "多声道麦克风回退已启用"
    );

    Ok(MicCapture {
        native_rate,
        _stream: stream,
        samples: consumer,
        overflow,
        errors,
    })
}

fn build_stream(
    device: &cpal::Device,
    supported: &cpal::SupportedStreamConfig,
    mut producer: Producer<f32>,
    overflow: Arc<AtomicU64>,
    errors: Arc<Mutex<Option<String>>>,
) -> Result<cpal::Stream, AudioError> {
    let channels = usize::from(supported.channels());
    let config: cpal::StreamConfig = supported.clone().into();

    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                push_mono(&mut producer, data.iter().copied(), channels, &overflow)
            },
            error_callback(errors),
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                push_mono(
                    &mut producer,
                    data.iter().map(|s| f32::from(*s) / 32768.0),
                    channels,
                    &overflow,
                )
            },
            error_callback(errors),
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| {
                push_mono(
                    &mut producer,
                    data.iter().map(|s| (f32::from(*s) - 32768.0) / 32768.0),
                    channels,
                    &overflow,
                )
            },
            error_callback(errors),
            None,
        ),
        SampleFormat::I32 => device.build_input_stream(
            &config,
            move |data: &[i32], _| {
                push_mono(
                    &mut producer,
                    data.iter().map(|s| *s as f32 / 2_147_483_648.0),
                    channels,
                    &overflow,
                )
            },
            error_callback(errors),
            None,
        ),
        other => {
            return Err(AudioError::Start(format!(
                "不支持的麦克风采样格式：{other:?}"
            )))
        }
    };

    stream.map_err(|e| AudioError::Open(format!("打开麦克风采集失败：{e}")))
}

fn error_callback(
    errors: Arc<Mutex<Option<String>>>,
) -> impl FnMut(cpal::StreamError) + Send + 'static {
    move |err| {
        tracing::warn!("麦克风采集错误：{err}");
        if let Ok(mut slot) = errors.try_lock() {
            if slot.is_none() {
                *slot = Some(err.to_string());
            }
        }
    }
}

fn push_mono<I: Iterator<Item = f32>>(
    producer: &mut Producer<f32>,
    samples: I,
    channels: usize,
    overflow: &AtomicU64,
) {
    if channels == 0 {
        return;
    }
    let mut sum = 0.0;
    let mut count = 0usize;
    for sample in samples {
        sum += sample;
        count += 1;
        if count == channels {
            if producer.push(sum / channels as f32).is_err() {
                overflow.fetch_add(1, Ordering::Relaxed);
            }
            sum = 0.0;
            count = 0;
        }
    }
}

fn drain_samples(
    capture: &mut MicCapture,
    raw: &mut Vec<f32>,
    resampler: &mut Option<MonoResampler>,
    out: &mut VecDeque<f32>,
) -> Result<bool, AudioError> {
    let mut received = false;
    loop {
        raw.clear();
        while raw.len() < CHUNK_SAMPLES {
            match capture.samples.pop() {
                Ok(sample) => raw.push(sample),
                Err(_) => break,
            }
        }
        if raw.is_empty() {
            return Ok(received);
        }
        received = true;
        match resampler {
            Some(rs) => {
                let samples = rs
                    .push(raw)
                    .map_err(|e| AudioError::Start(format!("麦克风重采样失败：{e}")))?;
                out.extend(samples);
            }
            None => out.extend(raw.iter().copied()),
        }
        if raw.len() < CHUNK_SAMPLES {
            return Ok(received);
        }
    }
}

fn report_overflow(overflow: &AtomicU64, last: &mut Instant) {
    let dropped = overflow.swap(0, Ordering::Relaxed);
    if dropped == 0 || last.elapsed() < OVERFLOW_LOG_INTERVAL {
        return;
    }
    *last = Instant::now();
    tracing::warn!(dropped, "麦克风采集积压，已丢弃采样点");
}

fn find_device(host: &cpal::Host, wanted: Option<&str>) -> Option<cpal::Device> {
    match wanted {
        None => host.default_input_device(),
        Some(name) => {
            let mut matches = host
                .input_devices()
                .ok()?
                .filter(|d| d.name().is_ok_and(|n| n == name));
            let first = matches.next()?;
            matches.next().is_none().then_some(first)
        }
    }
}

fn make_resampler(native_rate: u32) -> Result<Option<MonoResampler>, AudioError> {
    if native_rate == TARGET_SAMPLE_RATE {
        return Ok(None);
    }
    MonoResampler::new(native_rate)
        .map(Some)
        .map_err(|e| AudioError::Start(format!("麦克风重采样器初始化失败：{e}")))
}

fn take_error(errors: &Arc<Mutex<Option<String>>>) -> Option<String> {
    errors.lock().unwrap_or_else(|e| e.into_inner()).take()
}

fn trim(buf: &mut VecDeque<f32>) {
    let max = TARGET_SAMPLE_RATE as usize * 2;
    if buf.len() > max {
        let excess = buf.len() - max;
        buf.drain(..excess);
    }
}

struct MonoResampler {
    inner: Async<f32>,
    chunk_in_frames: usize,
    max_out_frames: usize,
    in_accum: Vec<f32>,
    out_scratch: Vec<f32>,
}

impl MonoResampler {
    fn new(in_rate: u32) -> Result<Self, String> {
        let ratio = f64::from(TARGET_SAMPLE_RATE) / f64::from(in_rate);
        let chunk_in_frames = (in_rate as usize / 50).max(64);
        let params = SincInterpolationParameters {
            sinc_len: 128,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 128,
            window: WindowFunction::BlackmanHarris2,
        };
        let inner =
            Async::<f32>::new_sinc(ratio, 1.0, &params, chunk_in_frames, 1, FixedAsync::Input)
                .map_err(|e| e.to_string())?;
        let max_out_frames = inner.output_frames_max();
        Ok(Self {
            inner,
            chunk_in_frames,
            max_out_frames,
            in_accum: Vec::with_capacity(chunk_in_frames * 4),
            out_scratch: vec![0.0; max_out_frames],
        })
    }

    fn push(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        self.in_accum.extend_from_slice(samples);
        let mut out = Vec::new();
        while self.in_accum.len() >= self.chunk_in_frames {
            self.process(self.chunk_in_frames, None, &mut out)?;
        }
        Ok(out)
    }

    fn flush(&mut self) -> Result<Vec<f32>, String> {
        let mut out = Vec::new();
        let remaining = self.in_accum.len();
        if remaining == 0 {
            return Ok(out);
        }
        self.in_accum.resize(self.chunk_in_frames, 0.0);
        self.process(self.chunk_in_frames, Some(remaining), &mut out)?;
        Ok(out)
    }

    fn process(
        &mut self,
        frames: usize,
        partial_len: Option<usize>,
        out: &mut Vec<f32>,
    ) -> Result<(), String> {
        let in_adapter = InterleavedSlice::new(&self.in_accum[..frames], 1, frames)
            .map_err(|e| e.to_string())?;
        let mut out_adapter =
            InterleavedSlice::new_mut(&mut self.out_scratch, 1, self.max_out_frames)
                .map_err(|e| e.to_string())?;
        let indexing = Indexing {
            input_offset: 0,
            output_offset: 0,
            partial_len,
            active_channels_mask: None,
        };
        let (_used, written) = self
            .inner
            .process_into_buffer(&in_adapter, &mut out_adapter, Some(&indexing))
            .map_err(|e| e.to_string())?;
        out.extend_from_slice(&self.out_scratch[..written]);
        self.in_accum.drain(..frames);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(consumer: &mut Consumer<f32>) -> Vec<f32> {
        let mut out = Vec::new();
        while let Ok(sample) = consumer.pop() {
            out.push(sample);
        }
        out
    }

    #[test]
    fn multichannel_frames_are_averaged_into_the_ring() {
        let (mut producer, mut consumer) = RingBuffer::<f32>::new(8);
        let overflow = AtomicU64::new(0);
        push_mono(
            &mut producer,
            [0.2, 0.4, 0.6, 0.8, -0.2, -0.4, -0.6, -0.8].into_iter(),
            4,
            &overflow,
        );
        let out = drain(&mut consumer);
        assert_eq!(out.len(), 2);
        assert!((out[0] - 0.5).abs() < 1e-6);
        assert!((out[1] + 0.5).abs() < 1e-6);
        assert_eq!(overflow.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn downmix_ignores_a_trailing_partial_frame() {
        let (mut producer, mut consumer) = RingBuffer::<f32>::new(4);
        let overflow = AtomicU64::new(0);
        push_mono(&mut producer, [0.5, 0.5, 0.5].into_iter(), 2, &overflow);
        assert_eq!(drain(&mut consumer), vec![0.5]);
    }

    #[test]
    fn full_ring_drops_samples_and_counts_them() {
        let (mut producer, _consumer) = RingBuffer::<f32>::new(2);
        let overflow = AtomicU64::new(0);
        push_mono(
            &mut producer,
            [0.1, 0.2, 0.3, 0.4].into_iter(),
            1,
            &overflow,
        );
        assert_eq!(overflow.load(Ordering::Relaxed), 2);
    }
}
