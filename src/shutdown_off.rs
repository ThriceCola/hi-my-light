use std::str::FromStr;
use std::time::Duration;

use btleplug::api::BDAddr;
use uuid::Uuid;

use crate::error::BleError;
use crate::manager::BleManager;
use crate::protocol::{Channel, Power};

pub const TARGET_CHAR_UUID: &str = "0000fff3-0000-1000-8000-00805f9b34fb";

/// 不经过 GUI 工作线程，直接连灯写关灯。关机路径必须走这里。
pub async fn shutdown_turn_off(addr: &str) -> Result<(), BleError> {
    let uuid = Uuid::parse_str(TARGET_CHAR_UUID)
        .map_err(|err| BleError::Internal(err.to_string()))?;
    let bd = BDAddr::from_str(addr).map_err(|_| BleError::PeripheralNotFound(addr.into()))?;

    let mut last = BleError::NoAdapter;
    for attempt in 0..3 {
        match shutdown_turn_off_once(&bd, &uuid).await {
            Ok(()) => return Ok(()),
            Err(err) => last = err,
        }
        if attempt + 1 < 3 {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    Err(last)
}

async fn shutdown_turn_off_once(bd: &btleplug::api::BDAddr, uuid: &Uuid) -> Result<(), BleError> {
    let manager = BleManager::new().await?;
    if manager.connect(bd).await.is_err() {
        manager.scan_once(Duration::from_secs(3)).await?;
        manager.connect(bd).await?;
    }
    manager.find_characteristic(uuid).await?;
    manager
        .write(uuid, Power::off(Channel::Front).frame().as_ref(), false)
        .await?;
    manager
        .write(uuid, Power::off(Channel::Rear).frame().as_ref(), false)
        .await?;
    Ok(())
}
