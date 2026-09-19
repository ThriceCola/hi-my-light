mod pending;

use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use gpui::{Context, EventEmitter, SharedString};
use hi_my_light::audio::{self, Capture};
use hi_my_light::{Cct, Effect, Frame, Level, Rgb};

use crate::bridge::{self, BleCmd, BleMsg, DeviceRow};
use crate::lamp::{Front, Rear, RearLook, Scene};
use crate::session::{ClosePreference, FrontSnap, RearSnap, Session};

use pending::{Pending, Slot};

const WRITE_GAP: Duration = Duration::from_millis(55);

pub struct LampService {
    cmd_tx: Sender<BleCmd>,
    evt_rx: Receiver<BleMsg>,
    pub ready: bool,
    pub scanning: bool,
    pub devices: Vec<DeviceRow>,
    pub connected_name: Option<String>,
    pub connected_addr: Option<String>,
    pub status: SharedString,
    pub last_frame: SharedString,
    pub connecting: bool,
    pub remembered_addr: Option<String>,
    last_write: Instant,
    pending: Pending,
    retry_at: Option<Instant>,
    pub parked: bool,
    pub close_preference: ClosePreference,
    restore_after_shutdown: bool,
    shutting_down: bool,
    pub quitting: bool,
    shutdown_quit_at: Option<Instant>,
    pub autostart: bool,
    pub off_on_shutdown: bool,
    pub front: Front,
    pub rear: Rear,
    capture: Option<Capture>,
    pub audio_rms: f32,
    pub audio_loud: bool,
    pub audio_sensitivity: Level,
    quiet_since: Option<Instant>,
    audio_retry_at: Option<Instant>,
    last_scroll: Instant,
}

pub enum ServiceEvent {
    Changed,
}

impl EventEmitter<ServiceEvent> for LampService {}

impl LampService {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let session = Session::load();
        let (cmd_tx, evt_rx) = bridge::spawn_worker();
        crate::os_shutdown::bind(cmd_tx.clone());
        let front = session.front_lamp();
        let rear = session.rear_lamp();
        crate::os_shutdown::update_snapshot(
            session.last_addr.clone().filter(|a| !a.is_empty()),
            session.off_on_shutdown,
            front.on || rear.on,
        );
        let remembered = session.last_addr.clone().filter(|a| !a.is_empty());
        let reconnecting = remembered.is_some();
        if !reconnecting {
            let _ = cmd_tx.send(BleCmd::Scan);
        }

        cx.spawn(async move |this, cx| {
            loop {
                let wait = match this.update(cx, |this, cx| {
                    if crate::os_shutdown::take() {
                        this.begin_os_shutdown();
                    }
                    if this.shutdown_quit_at.is_some_and(|at| Instant::now() >= at) {
                        this.persist();
                        cx.quit();
                    }
                    let retried = this.flush_retry();
                    let audio = this.tick_audio();
                    let wrote = this.flush_pending();
                    let drained = this.drain();
                    if retried || audio || wrote || drained {
                        cx.notify();
                        cx.emit(ServiceEvent::Changed);
                    }
                    this.poll_interval()
                }) {
                    Ok(wait) => wait,
                    Err(_) => break,
                };
                cx.background_executor().timer(wait).await;
            }
        })
        .detach();

        let autostart = session.autostart;
        if autostart {
            let _ = crate::desktop::sync_autostart(true);
        }

        Self {
            cmd_tx,
            evt_rx,
            ready: false,
            scanning: !reconnecting,
            devices: Vec::new(),
            connected_name: session.last_name.clone(),
            connected_addr: None,
            status: if reconnecting {
                "正在连接…".into()
            } else {
                "启动中…".into()
            },
            last_frame: "—".into(),
            connecting: reconnecting,
            remembered_addr: remembered,
            last_write: Instant::now() - WRITE_GAP,
            pending: Pending::default(),
            retry_at: None,
            parked: false,
            close_preference: session.close_preference,
            restore_after_shutdown: session.restore_after_shutdown,
            shutting_down: false,
            quitting: false,
            shutdown_quit_at: None,
            autostart,
            off_on_shutdown: session.off_on_shutdown,
            front,
            rear,
            capture: None,
            audio_rms: 0.0,
            audio_loud: false,
            audio_sensitivity: Level::from_byte(session.rear.audio_sensitivity),
            quiet_since: None,
            audio_retry_at: None,
            last_scroll: Instant::now() - audio::TICK,
        }
    }

    fn poll_interval(&self) -> Duration {
        if self.shutting_down || self.quitting {
            Duration::from_millis(40)
        } else if self.parked {
            Duration::from_millis(750)
        } else {
            Duration::from_millis(40)
        }
    }

    pub fn begin_user_quit(&mut self, cx: &mut Context<Self>) {
        if self.quitting || self.shutting_down {
            return;
        }
        self.quitting = true;
        self.parked = true;
        self.connecting = false;
        self.retry_at = None;
        self.pending = Pending::default();
        self.stop_audio();
        self.status = "正在断开蓝牙".into();
        self.persist();
        let addr = self
            .connected_addr
            .clone()
            .or_else(|| self.remembered_addr.clone());
        self.send(BleCmd::Disconnect);
        self.shutdown_quit_at = Some(Instant::now() + Duration::from_millis(if addr.is_some() {
            2200
        } else {
            80
        }));
        if let Some(addr) = addr {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .spawn(async move {
                        hi_my_light::release_device_blocking(&addr);
                    })
                    .await;
                let _ = this.update(cx, |this, cx| {
                    this.persist();
                    cx.quit();
                });
            })
            .detach();
        }
        cx.notify();
        cx.emit(ServiceEvent::Changed);
    }

    fn begin_os_shutdown(&mut self) {
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;
        self.parked = true;
        self.connecting = false;
        self.retry_at = None;
        self.pending = Pending::default();
        self.stop_audio();

        if self.off_on_shutdown {
            self.restore_after_shutdown = true;
            self.front.extinguish();
            self.rear.extinguish();
            self.status = "系统关机，关灯".into();
            if !crate::os_shutdown::already_fired() {
                self.send(BleCmd::ShutdownOff(self.remembered_addr.clone()));
            }
            self.shutdown_quit_at = Some(Instant::now() + Duration::from_millis(4000));
        } else {
            self.status = "系统关机".into();
            self.shutdown_quit_at = Some(Instant::now() + Duration::from_millis(120));
        }
        self.persist();
    }

    pub fn park(&mut self) {
        self.parked = true;
        self.connecting = false;
        self.retry_at = None;
        self.stop_audio();
        if self.connected() {
            self.send(BleCmd::Disconnect);
        }
        self.status = "后台".into();
        self.persist();
    }

    pub fn wake(&mut self) {
        if !self.parked || self.quitting || self.shutting_down {
            return;
        }
        self.parked = false;
        if let Some(addr) = self.remembered_addr.clone() {
            self.connecting = true;
            self.status = "正在连接…".into();
            self.send(BleCmd::Reconnect(addr));
        } else {
            self.status = "已打开".into();
        }
    }

    pub fn set_close_preference(&mut self, preference: ClosePreference) {
        self.close_preference = preference;
        self.persist();
    }

    pub fn set_off_on_shutdown(&mut self, enabled: bool) {
        self.off_on_shutdown = enabled;
        self.persist();
    }

    pub fn set_autostart(&mut self, enabled: bool) {
        self.autostart = enabled;
        self.persist();
        if let Err(err) = crate::desktop::sync_autostart(enabled) {
            self.status = format!("自启动写入失败: {err}").into();
        }
    }

    pub fn connected(&self) -> bool {
        self.connected_addr.is_some()
    }

    pub fn send(&self, cmd: BleCmd) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn scan(&mut self) {
        if self.parked {
            return;
        }
        self.status = "扫描中…".into();
        self.send(BleCmd::Scan);
    }

    pub fn connect(&mut self, addr: String) {
        self.remembered_addr = Some(addr.clone());
        self.connecting = true;
        self.retry_at = None;
        self.status = "连接中…".into();
        self.send(BleCmd::Connect(addr));
        self.persist();
    }

    pub fn has_known_device(&self) -> bool {
        self.remembered_addr
            .as_ref()
            .is_some_and(|addr| !addr.is_empty())
            || self.connected()
    }

    pub fn persist(&self) {
        self.persist_session(false);
    }

    fn persist_session(&self, clear_restore: bool) {
        let disk = Session::load();
        Session {
            last_addr: self.remembered_addr.clone(),
            last_name: self.connected_name.clone(),
            close_preference: self.close_preference,
            restore_after_shutdown: hi_my_light::shutdown_once::restore_flag_to_write(
                disk.restore_after_shutdown,
                self.restore_after_shutdown,
                clear_restore,
            ),
            autostart: self.autostart,
            off_on_shutdown: self.off_on_shutdown,
            front: FrontSnap::from_lamp(&self.front),
            rear: {
                let mut rear = RearSnap::from_lamp(&self.rear);
                rear.audio_sensitivity = self.audio_sensitivity.byte();
                rear
            },
        }
        .save();
        crate::os_shutdown::update_snapshot(
            self.remembered_addr.clone(),
            self.off_on_shutdown,
            self.front.on || self.rear.on,
        );
    }

    fn flush_retry(&mut self) -> bool {
        let Some(at) = self.retry_at else {
            return false;
        };
        if Instant::now() < at {
            return false;
        }
        self.retry_at = None;
        let Some(addr) = self.remembered_addr.clone() else {
            return false;
        };
        if self.parked
            || self.shutting_down
            || self.quitting
            || self.connected()
            || !self.connecting
        {
            return false;
        }
        self.status = "重连中…".into();
        self.send(BleCmd::Reconnect(addr));
        true
    }

    fn schedule_retry(&mut self) {
        if !self.parked
            && !self.shutting_down
            && !self.quitting
            && self.connecting
            && self.remembered_addr.is_some()
            && !self.connected()
        {
            self.retry_at = Some(Instant::now() + Duration::from_secs(2));
        }
    }

    pub fn disconnect(&mut self) {
        self.send(BleCmd::Disconnect);
    }

    pub fn set_front_level(&mut self, percent: f32) {
        self.front.set_level(Level::from_percent(percent));
        self.arm_front();
        self.pending.mark(Slot::FrontLevel);
        self.flush_pending();
    }

    pub fn set_rear_level(&mut self, percent: f32) {
        self.rear.set_level(Level::from_percent(percent));
        self.arm_rear();
        self.pending.mark(Slot::RearLevel);
        self.flush_pending();
    }

    pub fn set_cct(&mut self, kelvin: f32) {
        self.front.cct = Cct::from_kelvin(kelvin.round() as u16);
        self.front.on = true;
        self.arm_front();
        self.pending.mark(Slot::FrontCct);
        self.flush_pending();
    }

    pub fn set_rgb(&mut self, rgb: Rgb) {
        self.rear.set_rgb(rgb);
        self.stop_audio();
        self.arm_rear();
        self.pending.mark(Slot::RearLook);
        self.flush_pending();
    }

    pub fn set_speed(&mut self, percent: f32) {
        self.rear.speed = Level::from_percent(percent);
        self.arm_rear();
        self.pending.mark(Slot::RearSpeed);
        self.flush_pending();
    }

    pub fn set_audio_sensitivity(&mut self, percent: f32) {
        self.audio_sensitivity = Level::from_percent(percent);
        let was_loud = self.audio_loud;
        self.gate_audio();
        if was_loud && !self.audio_loud && self.connected() {
            self.write(Rgb::BLACK.scroll_frame());
        }
    }

    fn gate_audio(&mut self) {
        let threshold = audio::loud_rms(self.audio_sensitivity.percent());
        self.audio_loud = self.audio_rms >= threshold;
        if self.audio_loud || self.audio_rms > audio::SILENCE_RMS {
            self.quiet_since = None;
        } else if self.quiet_since.is_none() {
            self.quiet_since = Some(Instant::now());
        }
    }

    fn arm_front(&mut self) {
        if let Some(frame) = self.front.arm() {
            self.write(frame);
        }
    }

    fn arm_rear(&mut self) {
        if let Some(frame) = self.rear.arm() {
            self.write(frame);
        }
    }

    pub fn flush_now(&mut self) {
        self.last_write = Instant::now() - WRITE_GAP;
        while self.flush_pending() {}
        self.persist();
    }

    fn flush_pending(&mut self) -> bool {
        if self.pending.is_empty() || !self.connected() {
            return false;
        }
        if self.last_write.elapsed() < WRITE_GAP {
            return false;
        }
        let Some(slot) = self.pending.take_slot() else {
            return false;
        };
        let frame = self.encode(slot);
        self.last_write = Instant::now();
        self.write(frame);
        true
    }

    fn encode(&self, slot: Slot) -> Frame {
        match slot {
            Slot::FrontLevel => self.front.brightness_frame(),
            Slot::FrontCct => self.front.cct_frame(),
            Slot::RearLevel => self.rear.brightness_frame(),
            Slot::RearLook => self.rear.look_frame(),
            Slot::RearSpeed => self.rear.speed_frame(),
        }
    }

    pub fn toggle_front(&mut self) {
        self.front.toggle();
        self.apply_front();
        self.persist();
    }

    pub fn toggle_rear(&mut self) {
        self.rear.toggle();
        self.apply_rear();
        self.persist();
    }

    pub fn set_scene(&mut self, scene: Scene) {
        self.front.on = true;
        self.front.level = scene.level;
        self.front.cct = scene.cct;
        self.status = scene.name.into();
        self.apply_front();
        self.persist();
    }

    pub fn set_effect(&mut self, effect: Effect) {
        self.rear.set_effect(effect);
        self.stop_audio();
        self.status = effect.name().into();
        self.apply_rear();
        self.persist();
    }

    pub fn show_rear_solid(&mut self) {
        self.rear.show_solid();
        self.stop_audio();
        self.apply_rear();
        self.persist();
    }

    pub fn show_rear_effect(&mut self) {
        self.rear.show_effect();
        self.stop_audio();
        self.status = self.rear.effect.name().into();
        self.apply_rear();
        self.persist();
    }

    pub fn show_rear_audio(&mut self) {
        self.rear.show_audio();
        self.status = "电脑音频".into();
        self.apply_rear();
        self.persist();
    }

    fn stop_audio(&mut self) {
        self.capture = None;
        self.audio_loud = false;
        self.quiet_since = None;
        self.audio_rms = 0.0;
        self.audio_retry_at = None;
    }

    fn want_capture(&self) -> bool {
        matches!(self.rear.look, RearLook::Audio)
            && self.rear.on
            && !self.parked
            && !self.quitting
            && !self.shutting_down
    }

    pub fn audio_caption(&self) -> &'static str {
        if self.audio_loud {
            "正在播放"
        } else if self
            .quiet_since
            .is_some_and(|at| at.elapsed() >= Duration::from_secs(2))
        {
            "暂停中"
        } else {
            "暂缓"
        }
    }

    fn tick_audio(&mut self) -> bool {
        if !self.want_capture() {
            let had = self.capture.is_some() || self.audio_loud;
            self.stop_audio();
            return had;
        }

        let mut dirty = false;
        if self.capture.is_none() {
            if self
                .audio_retry_at
                .is_some_and(|at| Instant::now() < at)
            {
                return false;
            }
            match Capture::start() {
                Ok(capture) => {
                    self.capture = Some(capture);
                    self.audio_retry_at = None;
                    dirty = true;
                }
                Err(err) => {
                    self.status = format!("电脑音频: {err}").into();
                    self.audio_retry_at = Some(Instant::now() + Duration::from_secs(2));
                    return true;
                }
            }
        }

        let Some(capture) = self.capture.as_ref() else {
            return dirty;
        };
        let was_loud = self.audio_loud;
        let was_rms = self.audio_rms;
        let was_caption = self.audio_caption();
        match capture.try_sample() {
            Ok(Some(sample)) => {
                self.audio_rms = sample.rms;
                self.gate_audio();
            }
            Ok(None) => {}
            Err(err) => {
                self.capture = None;
                self.status = format!("电脑音频: {err}").into();
                self.audio_retry_at = Some(Instant::now() + Duration::from_secs(1));
                return true;
            }
        }

        if self.audio_caption() != was_caption
            || self.audio_loud != was_loud
            || (self.audio_rms - was_rms).abs() > 0.0005
        {
            dirty = true;
        }

        if self.connected() {
            if self.audio_loud && self.last_scroll.elapsed() >= audio::TICK {
                self.write(audio::LOUD_RGB.scroll_frame());
                self.last_scroll = Instant::now();
            } else if was_loud && !self.audio_loud {
                self.write(Rgb::BLACK.scroll_frame());
            }
        }
        dirty
    }

    fn apply_front(&mut self) {
        for frame in self.front.wake_frames() {
            self.write(frame);
        }
    }

    fn apply_rear(&mut self) {
        for frame in self.rear.wake_frames() {
            self.write(frame);
        }
    }

    fn restore_power(&mut self) {
        for frame in self.front.wake_frames() {
            self.write(frame);
        }
        for frame in self.rear.wake_frames() {
            self.write(frame);
        }
    }

    fn write(&mut self, frame: Frame) {
        if self.shutting_down || self.quitting || !self.connected() {
            return;
        }
        self.last_frame = frame.hex().into();
        self.last_write = Instant::now();
        self.send(BleCmd::Write(frame));
    }

    fn drain(&mut self) -> bool {
        let mut dirty = false;
        while let Ok(msg) = self.evt_rx.try_recv() {
            dirty = true;
            match msg {
                BleMsg::Ready => {
                    self.ready = true;
                    if let Some(addr) = self.remembered_addr.clone() {
                        if !self.connected()
                            && !self.quitting
                            && (!self.parked || self.restore_after_shutdown)
                        {
                            self.connecting = true;
                            self.status = "正在连接…".into();
                            self.send(BleCmd::Reconnect(addr));
                        } else if self.parked {
                            self.status = "后台".into();
                        }
                    } else {
                        self.status = "就绪".into();
                    }
                }
                BleMsg::AdapterFailed(e) => {
                    self.ready = false;
                    self.status = format!("适配器不可用 {e}").into();
                }
                BleMsg::Scanning(on) => {
                    self.scanning = on;
                    if on {
                        self.status = "扫描中".into();
                    }
                }
                BleMsg::Devices(mut rows) => {
                    crate::bridge::sort_devices(&mut rows);
                    self.devices = rows;
                    self.status = format!("发现 {} 台设备", self.devices.len()).into();
                }
                BleMsg::Connected { name, addr } => {
                    self.connecting = false;
                    self.retry_at = None;
                    self.remembered_addr = Some(addr.clone());
                    self.connected_name = Some(name.clone());
                    self.connected_addr = Some(addr);
                    self.status = format!("已连接 {name}").into();
                    let restoring = self.restore_after_shutdown;
                    if restoring {
                        self.restore_after_shutdown = false;
                        self.restore_power();
                        self.persist_session(true);
                    } else {
                        self.front.sync_lit();
                        self.rear.sync_lit();
                        self.persist();
                    }
                    if self.parked || self.shutting_down || self.quitting {
                        if !restoring {
                            self.send(BleCmd::Disconnect);
                        }
                        continue;
                    }
                    self.send(BleCmd::Handshake);
                }
                BleMsg::Disconnected => {
                    self.connecting = false;
                    self.retry_at = None;
                    self.connected_addr = None;
                    self.front.extinguish();
                    self.rear.extinguish();
                    self.status = "已断开".into();
                    self.persist();
                }
                BleMsg::Written(frame) => {
                    self.last_frame = frame.hex().into();
                }
                BleMsg::Error(e) => {
                    self.status = e.into();
                    self.schedule_retry();
                }
            }
        }
        dirty
    }
}
