use btleplug::api::{BDAddr, CharPropFlags};
use uuid::Uuid;

/// 已发现的 BLE 外设信息。
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 设备的蓝牙 MAC 地址。
    pub address: BDAddr,
    /// 广播的本地名称（如果有的话）。
    pub name: Option<String>,
    /// 接收信号强度指示器（如果有的话）。
    pub rssi: Option<i16>,
}

impl DeviceInfo {
    /// 返回人类可读的显示名称。
    ///
    /// 优先使用设备的广播名称，否则回退为 "未知" + 地址。
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("未知 ({})", self.address))
    }
}

/// 已连接设备上的 GATT 特征值信息。
#[derive(Debug, Clone)]
pub struct CharacteristicInfo {
    /// 特征值的 UUID。
    pub uuid: Uuid,
    /// 特征值属性（读 / 写 / 通知等）。
    pub properties: CharPropFlags,
}

impl CharacteristicInfo {
    /// 返回人类可读的摘要字符串。
    pub fn summary(&self) -> String {
        let mut flags = Vec::new();
        if self.properties.contains(CharPropFlags::READ) {
            flags.push("R");
        }
        if self.properties.contains(CharPropFlags::WRITE) {
            flags.push("W");
        }
        if self
            .properties
            .contains(CharPropFlags::WRITE_WITHOUT_RESPONSE)
        {
            flags.push("Ww/o");
        }
        if self.properties.contains(CharPropFlags::NOTIFY) {
            flags.push("N");
        }
        if self.properties.contains(CharPropFlags::INDICATE) {
            flags.push("I");
        }
        format!("{} [{}]", self.uuid, flags.join(", "))
    }
}
