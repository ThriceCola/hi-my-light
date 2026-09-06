use std::fmt;

/// BLE 操作过程中可能发生的错误。
#[derive(Debug, Clone)]
pub enum BleError {
    /// 系统上未找到 BLE 适配器。
    NoAdapter,
    /// 开始或停止扫描失败。
    ScanFailed(String),
    /// 未找到指定地址的外设。
    PeripheralNotFound(String),
    /// 未找到指定 UUID 的特征值。
    CharacteristicNotFound(String),
    /// 当前未连接到任何设备。
    NotConnected,
    /// 已连接到一个设备 —— 请先断开。
    AlreadyConnected,
    /// 向特征值写入数据失败。
    WriteFailed(String),
    /// 断开设备连接失败。
    DisconnectFailed(String),
    /// 连接设备失败。
    ConnectFailed(String),
    /// 在已连接设备上执行服务发现失败。
    ServiceDiscoveryFailed(String),
    /// 通用内部错误。
    Internal(String),
}

impl fmt::Display for BleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BleError::NoAdapter => write!(f, "没有找到 BLE 适配器"),
            BleError::ScanFailed(msg) => write!(f, "BLE 扫描失败: {}", msg),
            BleError::PeripheralNotFound(addr) => write!(f, "未找到设备: {}", addr),
            BleError::CharacteristicNotFound(uuid) => write!(f, "未找到特征值: {}", uuid),
            BleError::NotConnected => write!(f, "未连接到任何设备"),
            BleError::AlreadyConnected => write!(f, "已经连接到一个设备，请先断开"),
            BleError::WriteFailed(msg) => write!(f, "写入失败: {}", msg),
            BleError::DisconnectFailed(msg) => write!(f, "断开连接失败: {}", msg),
            BleError::ConnectFailed(msg) => write!(f, "连接失败: {}", msg),
            BleError::ServiceDiscoveryFailed(msg) => write!(f, "服务发现失败: {}", msg),
            BleError::Internal(msg) => write!(f, "内部错误: {}", msg),
        }
    }
}

impl std::error::Error for BleError {
}
