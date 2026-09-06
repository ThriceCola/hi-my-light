use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use btleplug::api::{
    Central, Manager as BtManager, Peripheral as BtPeripheral, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};

use crate::device::{CharacteristicInfo, DeviceInfo};
use crate::error::BleError;

// ---------------------------------------------------------------------------
// 事件
// ---------------------------------------------------------------------------

/// [`BleManager`] 发出的事件，前端可以订阅接收。
#[derive(Debug, Clone)]
pub enum BleEvent {
    /// 扫描过程中发现新设备。
    DeviceDiscovered(DeviceInfo),
    /// 已有设备的属性发生变化。
    DeviceUpdated(DeviceInfo),
    /// 设备不再可见。
    DeviceLost(btleplug::api::BDAddr),
    /// 成功连接到设备。
    Connected(DeviceInfo),
    /// 与设备断开连接。
    Disconnected,
    /// 扫描开始。
    ScanStarted,
    /// 扫描停止。
    ScanStopped,
    /// 正常流程之外发生的错误。
    Error(BleError),
}

// ---------------------------------------------------------------------------
// BleManager – 核心后端
// ---------------------------------------------------------------------------

/// 管理 BLE 适配器生命周期、扫描、连接和通信的后端。
/// 设计为可供任何前端（CLI、TUI、GUI 等）使用。
///
/// 所有公开方法都是 `&self` —— 内部可变性通过
/// `tokio::sync::Mutex` 处理，因此管理器可以在任务间共享。
pub struct BleManager {
    adapter: Adapter,
    event_tx: broadcast::Sender<BleEvent>,
    connected: Mutex<Option<Peripheral>>,
}

impl BleManager {
    // -- 生命周期 --------------------------------------------------------

    /// 通过打开第一个可用的 BLE 适配器创建一个新的 `BleManager`。
    ///
    /// 如果没有可用的蓝牙适配器，则返回 `BleError::NoAdapter`。
    pub async fn new() -> Result<Self, BleError> {
        let manager = Manager::new()
            .await
            .map_err(|e| BleError::Internal(format!("创建 BLE Manager 失败: {}", e)))?;
        let adapters = manager
            .adapters()
            .await
            .map_err(|e| BleError::Internal(format!("获取 BLE 适配器失败: {}", e)))?;
        let adapter = adapters.into_iter().next().ok_or(BleError::NoAdapter)?;

        let (event_tx, _) = broadcast::channel(256);

        Ok(Self {
            adapter,
            event_tx,
            connected: Mutex::new(None),
        })
    }

    /// 订阅 BLE 事件。
    ///
    /// 每个订阅者都会收到每个事件的克隆副本。
    pub fn events(&self) -> broadcast::Receiver<BleEvent> {
        self.event_tx.subscribe()
    }

    /// 获取底层适配器的引用（供高级用法使用）。
    pub fn adapter(&self) -> &Adapter {
        &self.adapter
    }

    // -- 扫描 ---------------------------------------------------------

    /// 开始扫描附近的 BLE 设备。
    ///
    /// 使用默认扫描过滤器。设备被发现后会通过
    /// [`BleEvent::DeviceDiscovered`] 发出事件。
    pub async fn start_scan(&self) -> Result<(), BleError> {
        self.adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| BleError::ScanFailed(e.to_string()))?;
        let _ = self.event_tx.send(BleEvent::ScanStarted);
        Ok(())
    }

    /// 停止正在进行的扫描。
    pub async fn stop_scan(&self) -> Result<(), BleError> {
        self.adapter
            .stop_scan()
            .await
            .map_err(|e| BleError::ScanFailed(e.to_string()))?;
        let _ = self.event_tx.send(BleEvent::ScanStopped);
        Ok(())
    }

    /// 返回当前发现的外设的快照。
    ///
    /// 重复调用此方法（或订阅事件）以获取最新列表。
    pub async fn discovered_devices(&self) -> Result<Vec<DeviceInfo>, BleError> {
        let peripherals = self
            .adapter
            .peripherals()
            .await
            .map_err(|e| BleError::ScanFailed(e.to_string()))?;

        let mut devices = Vec::with_capacity(peripherals.len());
        for p in &peripherals {
            if let Some(props) = p
                .properties()
                .await
                .map_err(|e| BleError::ScanFailed(e.to_string()))?
            {
                devices.push(DeviceInfo {
                    address: props.address,
                    name: props.local_name,
                    rssi: props.rssi,
                });
            }
        }
        Ok(devices)
    }

    // -- 连接 -------------------------------------------------------

    /// 通过蓝牙地址连接到指定的外设。
    ///
    /// 如果已连接到其他设备，则返回错误。
    /// 连接成功后，管理器会自动执行服务发现。
    pub async fn connect(&self, address: &btleplug::api::BDAddr) -> Result<DeviceInfo, BleError> {
        // 保护：每次只允许一个连接。
        {
            let guard = self.connected.lock().await;
            if guard.is_some() {
                return Err(BleError::AlreadyConnected);
            }
        }

        // 查找目标外设。
        let peripherals = self
            .adapter
            .peripherals()
            .await
            .map_err(|e| BleError::ConnectFailed(e.to_string()))?;

        let mut target: Option<(Peripheral, DeviceInfo)> = None;
        for p in peripherals.into_iter() {
            if let Ok(Some(props)) = p.properties().await {
                if props.address == *address {
                    let info = DeviceInfo {
                        address: props.address,
                        name: props.local_name,
                        rssi: props.rssi,
                    };
                    target = Some((p, info));
                    break;
                }
            }
        }

        let (peripheral, info) =
            target.ok_or_else(|| BleError::PeripheralNotFound(address.to_string()))?;

        // 连接并发现服务。
        peripheral
            .connect()
            .await
            .map_err(|e| BleError::ConnectFailed(e.to_string()))?;

        peripheral
            .discover_services()
            .await
            .map_err(|e| BleError::ServiceDiscoveryFailed(e.to_string()))?;

        // 存储已连接的外设。
        *self.connected.lock().await = Some(peripheral);
        let _ = self.event_tx.send(BleEvent::Connected(info.clone()));

        Ok(info)
    }

    /// 断开与当前连接设备的连接。
    ///
    /// 如果未连接，则不执行任何操作。
    pub async fn disconnect(&self) -> Result<(), BleError> {
        let mut guard = self.connected.lock().await;
        if let Some(p) = guard.take() {
            p.disconnect()
                .await
                .map_err(|e| BleError::DisconnectFailed(e.to_string()))?;
            let _ = self.event_tx.send(BleEvent::Disconnected);
        }
        Ok(())
    }

    /// 检查当前是否已连接到设备。
    pub async fn is_connected(&self) -> bool {
        self.connected.lock().await.is_some()
    }

    /// 返回当前连接设备的信息（如果有的话）。
    pub async fn connected_device_info(&self) -> Option<DeviceInfo> {
        let guard = self.connected.lock().await;
        let p = guard.as_ref()?;
        let props = p.properties().await.ok()??;
        Some(DeviceInfo {
            address: props.address,
            name: props.local_name,
            rssi: props.rssi,
        })
    }

    // -- 服务 / 特征值 ---------------------------------------

    /// 在已连接的设备上执行服务发现。
    ///
    /// 通常在 [`connect`] 过程中自动完成，但你可以
    /// 再次调用以刷新。
    pub async fn discover_services(&self) -> Result<(), BleError> {
        let guard = self.connected.lock().await;
        let p = guard.as_ref().ok_or(BleError::NotConnected)?;
        p.discover_services()
            .await
            .map_err(|e| BleError::ServiceDiscoveryFailed(e.to_string()))?;
        Ok(())
    }

    /// 列出已连接设备上的所有特征值。
    pub async fn characteristics(&self) -> Result<Vec<CharacteristicInfo>, BleError> {
        let guard = self.connected.lock().await;
        let p = guard.as_ref().ok_or(BleError::NotConnected)?;
        let chars = p.characteristics();
        Ok(chars
            .iter()
            .map(|c| CharacteristicInfo {
                uuid: c.uuid,
                properties: c.properties,
            })
            .collect())
    }

    /// 在已连接设备上通过 UUID 查找特征值。
    pub async fn find_characteristic(&self, uuid: &Uuid) -> Result<CharacteristicInfo, BleError> {
        let chars = self.characteristics().await?;
        chars
            .into_iter()
            .find(|c| c.uuid == *uuid)
            .ok_or_else(|| BleError::CharacteristicNotFound(uuid.to_string()))
    }

    // -- 通信 ----------------------------------------------------

    /// 向已连接设备的特征值写入数据。
    ///
    /// * `char_uuid` – 目标特征值的 UUID。
    /// * `data`      – 要写入的字节数据。
    /// * `response`  – `true` = 请求响应；`false` = 无需响应写入
    ///   （即发即忘，速度更快）。
    pub async fn write(
        &self,
        char_uuid: &Uuid,
        data: &[u8],
        response: bool,
    ) -> Result<(), BleError> {
        let guard = self.connected.lock().await;
        let p = guard.as_ref().ok_or(BleError::NotConnected)?;

        let write_type = if response {
            WriteType::WithResponse
        } else {
            WriteType::WithoutResponse
        };

        let chars = p.characteristics();
        let characteristic = chars
            .iter()
            .find(|c| c.uuid == *char_uuid)
            .ok_or_else(|| BleError::CharacteristicNotFound(char_uuid.to_string()))?;

        p.write(characteristic, data, write_type)
            .await
            .map_err(|e| BleError::WriteFailed(e.to_string()))?;

        Ok(())
    }

    // -- 便捷辅助方法 ----------------------------------------------

    /// 短时间扫描并返回所有发现的设备。
    ///
    /// 这是一个便捷方法，组合了 `start_scan`、等待 `duration`、
    /// `stop_scan` 和查询外设列表。
    pub async fn scan_once(
        &self,
        duration: std::time::Duration,
    ) -> Result<Vec<DeviceInfo>, BleError> {
        self.start_scan().await?;
        tokio::time::sleep(duration).await;
        let devices = self.discovered_devices().await?;
        self.stop_scan().await?;
        Ok(devices)
    }
}
