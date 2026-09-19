//! PipeWire 上的 Pulse 兼容层：录默认扬声器的 monitor，不是麦克风。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use libpulse_binding::def::BufferAttr;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;

use super::{Error, Sample, TICK};

pub const SUPPORTED: bool = true;

const RATE: u32 = 48_000;
const CHANNELS: u8 = 2;
const CHECK_SINK: Duration = Duration::from_secs(2);

pub struct Capture {
    stop: Arc<AtomicBool>,
    dead: Arc<AtomicBool>,
    latest: Arc<Mutex<Option<Sample>>>,
}

impl Capture {
    pub fn start() -> Result<Self, Error> {
        let monitor = default_monitor()?;
        let stop = Arc::new(AtomicBool::new(false));
        let dead = Arc::new(AtomicBool::new(false));
        let latest = Arc::new(Mutex::new(None));
        let flag = stop.clone();
        let died = dead.clone();
        let slot = latest.clone();
        thread::Builder::new()
            .name("hi-my-light-audio".into())
            .spawn(move || {
                run(flag, slot, monitor);
                died.store(true, Ordering::Relaxed);
            })
            .map_err(|err| Error::Backend(err.to_string()))?;
        Ok(Self { stop, dead, latest })
    }

    pub fn try_sample(&self) -> Result<Option<Sample>, Error> {
        if self.dead.load(Ordering::Relaxed) {
            return Err(Error::Backend("音频线程已退出".into()));
        }
        Ok(*self.latest.lock().expect("audio slot"))
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn run(stop: Arc<AtomicBool>, latest: Arc<Mutex<Option<Sample>>>, mut monitor: String) {
    let mut stream = match open(&monitor) {
        Ok(stream) => stream,
        Err(_) => return,
    };
    let mut buf = vec![0u8; chunk_bytes()];
    let mut checked = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        if checked.elapsed() >= CHECK_SINK {
            checked = Instant::now();
            if let Ok(next) = default_monitor() {
                if next != monitor {
                    if let Ok(next_stream) = open(&next) {
                        stream = next_stream;
                        monitor = next;
                    }
                }
            }
        }

        if stream.read(&mut buf).is_err() {
            thread::sleep(Duration::from_millis(40));
            match default_monitor().and_then(|name| {
                monitor = name.clone();
                open(&name)
            }) {
                Ok(next) => stream = next,
                Err(_) => {
                    store(&latest, Sample::default());
                    thread::sleep(TICK);
                }
            }
            continue;
        }

        let rms = rms_bytes(&buf);
        store(&latest, Sample { rms, loud: false });
    }
}

fn store(latest: &Mutex<Option<Sample>>, sample: Sample) {
    *latest.lock().expect("audio slot") = Some(sample);
}

fn open(monitor: &str) -> Result<Simple, Error> {
    let spec = Spec {
        format: Format::F32le,
        channels: CHANNELS,
        rate: RATE,
    };
    if !spec.is_valid() {
        return Err(Error::Backend("采样格式无效".into()));
    }
    let fragsize = chunk_bytes() as u32;
    let attr = BufferAttr {
        maxlength: fragsize * 4,
        tlength: u32::MAX,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize,
    };
    Simple::new(
        None,
        "hi-my-light",
        Direction::Record,
        Some(monitor),
        "pc-monitor",
        &spec,
        None,
        Some(&attr),
    )
    .map_err(|err| Error::Backend(format!("打开 {monitor}: {err}")))
}

fn chunk_bytes() -> usize {
    let frames = (RATE as u64 * TICK.as_millis() as u64 / 1000) as usize;
    frames * CHANNELS as usize * 4
}

fn rms_bytes(bytes: &[u8]) -> f32 {
    let mut n = 0u32;
    let mut sum = 0.0f32;
    for chunk in bytes.chunks_exact(4) {
        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        sum += sample * sample;
        n += 1;
    }
    if n == 0 {
        0.0
    } else {
        (sum / n as f32).sqrt()
    }
}

fn default_monitor() -> Result<String, Error> {
    use libpulse_binding::context::{Context, FlagSet, State};
    use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
    use libpulse_binding::operation::State as OpState;

    let mut mainloop = Mainloop::new().ok_or_else(|| Error::Backend("无法创建 Pulse 主循环".into()))?;
    let mut context = Context::new(&mainloop, "hi-my-light-sink")
        .ok_or_else(|| Error::Backend("无法创建 Pulse 上下文".into()))?;
    context
        .connect(None, FlagSet::NOFLAGS, None)
        .map_err(|err| Error::Backend(err.to_string().unwrap_or_else(|| "Pulse 连接失败".into())))?;

    loop {
        match mainloop.iterate(false) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                return Err(Error::Backend("Pulse 主循环失败".into()));
            }
            IterateResult::Success(_) => {}
        }
        match context.get_state() {
            State::Ready => break,
            State::Failed | State::Terminated => {
                return Err(Error::Backend("连不上 PipeWire/Pulse".into()));
            }
            _ => {}
        }
    }

    let (tx, rx) = mpsc::channel();
    let op = context.introspect().get_server_info(move |info| {
        let name = info.default_sink_name.as_ref().map(|n| n.to_string());
        let _ = tx.send(name);
    });

    let deadline = Instant::now() + Duration::from_secs(2);
    while op.get_state() != OpState::Done {
        if Instant::now() > deadline {
            return Err(Error::Backend("读默认输出超时".into()));
        }
        match mainloop.iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                return Err(Error::Backend("读默认输出失败".into()));
            }
            IterateResult::Success(_) => {}
        }
    }

    let sink = rx
        .recv()
        .map_err(|_| Error::Backend("读默认输出失败".into()))?
        .ok_or_else(|| Error::Backend("没有默认扬声器".into()))?;
    if sink.is_empty() {
        return Err(Error::Backend("没有默认扬声器".into()));
    }
    Ok(format!("{sink}.monitor"))
}

#[cfg(test)]
mod live {
    use super::*;
    use std::time::Duration;

    #[test]
    fn capture_hears_playback() {
        let cap = Capture::start().expect("capture");
        let mut max = 0.0f32;
        for _ in 0..20 {
            thread::sleep(Duration::from_millis(80));
            if let Ok(Some(sample)) = cap.try_sample() {
                max = max.max(sample.rms);
            }
        }
        assert!(
            max > 0.01,
            "capture rms={max:.5}; monitor should have PCM"
        );
    }
}


