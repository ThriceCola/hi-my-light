use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use btleplug::api::BDAddr;
use hi_my_light::{Channel, Clock, Frame, Power, Query};
use uuid::Uuid;

pub const TARGET_CHAR_UUID: &str = "0000fff3-0000-1000-8000-00805f9b34fb";

pub const PREFERRED_DEVICE: &str = "THUNDEROBOT L2";

#[derive(Debug, Clone)]
pub struct DeviceRow {
    pub name: String,
    pub addr: String,
    pub rssi: Option<i16>,
}

impl DeviceRow {
    pub fn is_preferred(&self) -> bool {
        self.name.to_ascii_uppercase().contains(PREFERRED_DEVICE)
    }

    pub fn sort_key(&self) -> (u8, i16, String) {
        let rank = if self.is_preferred() { 0 } else { 1 };
        let rssi = self.rssi.unwrap_or(i16::MIN);
        (rank, -rssi, self.name.to_ascii_lowercase())
    }
}

pub fn sort_devices(devices: &mut [DeviceRow]) {
    devices.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
}

#[derive(Debug, Clone)]
pub enum BleCmd {
    Scan,
    Connect(String),
    Reconnect(String),
    Disconnect,
    Write(Frame),
    Handshake,
    ShutdownOff(Option<String>),
}

#[derive(Debug, Clone)]
pub enum BleMsg {
    Ready,
    AdapterFailed(String),
    Scanning(bool),
    Devices(Vec<DeviceRow>),
    Connected { name: String, addr: String },
    Disconnected,
    Written(Frame),
    Error(String),
}

pub fn spawn_worker() -> (Sender<BleCmd>, Receiver<BleMsg>) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (msg_tx, msg_rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("hi-my-light-ble".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = msg_tx.send(BleMsg::AdapterFailed(e.to_string()));
                    return;
                }
            };
            rt.block_on(run(cmd_rx, msg_tx));
        })
        .expect("启动 BLE 线程失败");

    (cmd_tx, msg_rx)
}

async fn run(cmd_rx: Receiver<BleCmd>, msg_tx: Sender<BleMsg>) {
    let mut manager = None;
    for attempt in 0..12 {
        match hi_my_light::BleManager::new().await {
            Ok(m) => {
                manager = Some(m);
                break;
            }
            Err(e) => {
                if attempt == 11 {
                    let _ = msg_tx.send(BleMsg::AdapterFailed(e.to_string()));
                    return;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
    let Some(manager) = manager else {
        return;
    };
    let _ = msg_tx.send(BleMsg::Ready);

    let char_uuid = Uuid::parse_str(TARGET_CHAR_UUID).expect("静态 UUID");

    loop {
        let cmd = match cmd_rx.recv() {
            Ok(c) => c,
            Err(_) => break,
        };

        match cmd {
            BleCmd::Scan => {
                let _ = msg_tx.send(BleMsg::Scanning(true));
                match manager.scan_once(Duration::from_secs(3)).await {
                    Ok(devices) => {
                        let mut rows: Vec<DeviceRow> = devices
                            .into_iter()
                            .map(|d| DeviceRow {
                                name: d.display_name(),
                                addr: d.address.to_string(),
                                rssi: d.rssi,
                            })
                            .collect();
                        sort_devices(&mut rows);
                        let _ = msg_tx.send(BleMsg::Devices(rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BleMsg::Error(e.to_string()));
                    }
                }
                let _ = msg_tx.send(BleMsg::Scanning(false));
            }
            BleCmd::Reconnect(addr) => {
                let _ = msg_tx.send(BleMsg::Scanning(true));
                match manager.scan_once(Duration::from_secs(3)).await {
                    Ok(devices) => {
                        let mut rows: Vec<DeviceRow> = devices
                            .into_iter()
                            .map(|d| DeviceRow {
                                name: d.display_name(),
                                addr: d.address.to_string(),
                                rssi: d.rssi,
                            })
                            .collect();
                        sort_devices(&mut rows);
                        let _ = msg_tx.send(BleMsg::Devices(rows));
                    }
                    Err(e) => {
                        let _ = msg_tx.send(BleMsg::Error(e.to_string()));
                    }
                }
                let _ = msg_tx.send(BleMsg::Scanning(false));
                connect_addr(&manager, &char_uuid, &addr, &msg_tx).await;
            }
            BleCmd::Connect(addr) => {
                connect_addr(&manager, &char_uuid, &addr, &msg_tx).await;
            }
            BleCmd::Disconnect => {
                if let Err(e) = manager.disconnect().await {
                    let _ = msg_tx.send(BleMsg::Error(e.to_string()));
                } else {
                    let _ = msg_tx.send(BleMsg::Disconnected);
                }
            }
            BleCmd::Write(frame) => write_frame(&manager, &char_uuid, frame, &msg_tx).await,
            BleCmd::Handshake => handshake(&manager, &char_uuid, &msg_tx).await,
            BleCmd::ShutdownOff(addr) => {
                shutdown_off(&manager, &char_uuid, addr.as_deref()).await;
            }
        }
    }
}

async fn handshake(
    manager: &hi_my_light::BleManager,
    uuid: &Uuid,
    msg_tx: &Sender<BleMsg>,
) {
    let query = Query.frame();
    let clock = Clock::now().frame();
    for frame in [query, query, clock, clock] {
        write_frame(manager, uuid, frame, msg_tx).await;
        tokio::time::sleep(Duration::from_millis(55)).await;
    }
}

async fn connect_addr(
    manager: &hi_my_light::BleManager,
    char_uuid: &Uuid,
    addr: &str,
    msg_tx: &Sender<BleMsg>,
) {
    match BDAddr::from_str(addr) {
        Ok(bd) => match manager.connect(&bd).await {
            Ok(info) => match manager.find_characteristic(char_uuid).await {
                Ok(_) => {
                    let _ = msg_tx.send(BleMsg::Connected {
                        name: info.display_name(),
                        addr: info.address.to_string(),
                    });
                }
                Err(e) => {
                    let _ = manager.disconnect().await;
                    let _ = msg_tx.send(BleMsg::Error(format!("已连接但未找到灯控特征值: {e}")));
                }
            },
            Err(e) => {
                let _ = msg_tx.send(BleMsg::Error(e.to_string()));
            }
        },
        Err(_) => {
            let _ = msg_tx.send(BleMsg::Error(format!("地址无效: {addr}")));
        }
    }
}

async fn shutdown_off(manager: &hi_my_light::BleManager, uuid: &Uuid, addr: Option<&str>) {
    if !manager.is_connected().await {
        let Some(addr) = addr else {
            return;
        };
        if !connect_quiet(manager, uuid, addr).await {
            let _ = manager.scan_once(Duration::from_secs(2)).await;
            if !connect_quiet(manager, uuid, addr).await {
                return;
            }
        }
    }
    let _ = manager
        .write(uuid, Power::off(Channel::Front).frame().as_ref(), false)
        .await;
    let _ = manager
        .write(uuid, Power::off(Channel::Rear).frame().as_ref(), false)
        .await;
}

async fn connect_quiet(manager: &hi_my_light::BleManager, char_uuid: &Uuid, addr: &str) -> bool {
    let Ok(bd) = BDAddr::from_str(addr) else {
        return false;
    };
    if manager.connect(&bd).await.is_err() {
        return false;
    }
    if manager.find_characteristic(char_uuid).await.is_ok() {
        true
    } else {
        let _ = manager.disconnect().await;
        false
    }
}

async fn write_frame(
    manager: &hi_my_light::BleManager,
    uuid: &Uuid,
    frame: Frame,
    msg_tx: &Sender<BleMsg>,
) {
    match manager.write(uuid, frame.as_ref(), false).await {
        Ok(()) => {
            let _ = msg_tx.send(BleMsg::Written(frame));
        }
        Err(e) => {
            let _ = msg_tx.send(BleMsg::Error(e.to_string()));
        }
    }
}
