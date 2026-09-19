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
    for attempt in 0..2 {
        match shutdown_turn_off_once(&bd, &uuid).await {
            Ok(()) => return Ok(()),
            Err(err) => last = err,
        }
        if attempt + 1 < 2 {
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }
    Err(last)
}

async fn shutdown_turn_off_once(bd: &btleplug::api::BDAddr, uuid: &Uuid) -> Result<(), BleError> {
    let manager = BleManager::new().await?;
    let result = async {
        if manager.connect(bd).await.is_err() {
            manager.scan_once(Duration::from_secs(2)).await?;
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
    .await;
    let _ = manager.disconnect_address(bd).await;
    result
}

/// 退出进程前把指定地址从适配器上拆掉。
pub async fn release_device(addr: &str) -> Result<(), BleError> {
    let bd = BDAddr::from_str(addr).map_err(|_| BleError::PeripheralNotFound(addr.into()))?;
    let manager = BleManager::new().await?;
    manager.disconnect_address(&bd).await
}

pub fn release_device_blocking(addr: &str) {
    let work = async {
        let _ = tokio::time::timeout(Duration::from_secs(2), release_device(addr)).await;
    };
    match tokio::runtime::Handle::try_current() {
        Ok(_) => {
            let _ = std::thread::scope(|scope| {
                scope
                    .spawn(|| {
                        tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .ok()
                            .map(|rt| rt.block_on(work))
                    })
                    .join()
            });
        }
        Err(_) => {
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                rt.block_on(work);
            }
        }
    }
}
