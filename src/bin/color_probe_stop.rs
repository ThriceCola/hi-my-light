//! 关键：滚着的时候停发，灯怎样。
//!
//!   cargo run --bin color-probe-stop
//!
//! 只有 3 步。看完 3 秒空窗再按 Enter。

use std::io::Write;
use std::time::Duration;

use uuid::Uuid;

use hi_my_light::{
    BleError, BleManager, Brightness, Channel, Clock, Level, Power, Query, Rgb,
};

const TARGET_CHAR_UUID: &str = "0000fff3-0000-1000-8000-00805f9b34fb";

macro_rules! read_line {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        std::io::stdout().flush().ok();
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
        buf.trim().to_string()
    }};
}

async fn pick_device(manager: &BleManager) -> anyhow::Result<btleplug::api::BDAddr> {
    println!("正在扫描附近的 BLE 设备 (3 秒)...");
    manager.scan_once(Duration::from_secs(3)).await?;
    loop {
        let devices = manager.discovered_devices().await?;
        if devices.is_empty() {
            println!("未发现设备。Enter 重扫，q 退出");
            let input = read_line!("> ");
            if input == "q" || input == "quit" {
                anyhow::bail!("已取消");
            }
            manager.scan_once(Duration::from_secs(2)).await?;
            continue;
        }
        println!("\n发现的设备:");
        for (i, d) in devices.iter().enumerate() {
            println!("  {}. {}", i + 1, d.display_name());
        }
        let input = read_line!(
            "\n选择设备编号 (1-{}), Enter 刷新, q 退出: ",
            devices.len()
        );
        match input.as_str() {
            "q" | "quit" => anyhow::bail!("已取消"),
            "" => tokio::time::sleep(Duration::from_secs(2)).await,
            s => {
                if let Ok(idx) = s.parse::<usize>() {
                    if (1..=devices.len()).contains(&idx) {
                        return Ok(devices[idx - 1].address);
                    }
                }
                println!("✗ 无效输入");
            }
        }
    }
}

async fn write_frame(
    manager: &BleManager,
    uuid: &Uuid,
    frame: hi_my_light::Frame,
) -> Result<(), BleError> {
    manager.write(uuid, &frame.bytes(), false).await
}

fn pause(step: u32, watch: &str) -> bool {
    println!();
    println!("── 看灯 ── {watch}");
    let input = read_line!("[{step}/3] 记下后 Enter，q 结束: ");
    input == "q" || input == "quit"
}

async fn solid_black(manager: &BleManager, uuid: &Uuid) -> Result<(), BleError> {
    write_frame(manager, uuid, Rgb::BLACK.frame()).await
}

async fn stream_secs(
    manager: &BleManager,
    uuid: &Uuid,
    color: Rgb,
    secs: u64,
) -> Result<u32, BleError> {
    let frame = color.scroll_frame();
    let mut n = 0u32;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    while tokio::time::Instant::now() < deadline {
        write_frame(manager, uuid, frame).await?;
        n += 1;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(n)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 关键：停发之后 ===");
    println!("手机 App 先断开。");

    let manager = BleManager::new().await?;
    let addr = pick_device(&manager).await?;
    println!("正在连接 {addr} ...");
    let info = manager.connect(&addr).await?;
    println!("✓ 已连接: {}", info.display_name());

    let uuid = Uuid::parse_str(TARGET_CHAR_UUID).expect("静态 UUID");
    manager.find_characteristic(&uuid).await?;

    let query = Query.frame();
    let clock = Clock::now().frame();
    for frame in [query, query, clock, clock] {
        write_frame(&manager, &uuid, frame).await?;
        tokio::time::sleep(Duration::from_millis(55)).await;
    }
    write_frame(&manager, &uuid, Power::on(Channel::Rear).frame()).await?;
    tokio::time::sleep(Duration::from_millis(80)).await;
    write_frame(
        &manager,
        &uuid,
        Brightness::new(Channel::Rear, Level::MAX).frame(),
    )
    .await?;

    // 1 一次一色然后停
    solid_black(&manager, &uuid).await?;
    println!();
    println!("步骤 1: 20 80 00 00 连发 3s（一次一色），然后 3s 完全不发");
    let n = stream_secs(&manager, &uuid, Rgb::new(0x80, 0, 0), 3).await?;
    println!("已停发（发过 {n} 帧）。看接下来 3 秒。");
    tokio::time::sleep(Duration::from_secs(3)).await;
    println!("3 秒到。");
    if pause(1, "停发后：还在滚 / 停在某一格 / 灭了？") {
        return wrap_up(&manager, &uuid).await;
    }

    // 2 一次一色然后写全零
    solid_black(&manager, &uuid).await?;
    println!();
    println!("步骤 2: 20 80 00 00 连发 3s，然后改发 20 00 00 00 共 2s");
    let n = stream_secs(&manager, &uuid, Rgb::new(0x80, 0, 0), 3).await?;
    println!("改发全零（前面 {n} 帧）。");
    let z = stream_secs(&manager, &uuid, Rgb::BLACK, 2).await?;
    println!("全零发完 {z} 帧。");
    if pause(2, "写全零后：灭了 / 还在滚 / 别的？") {
        return wrap_up(&manager, &uuid).await;
    }

    // 3 不间断然后停
    solid_black(&manager, &uuid).await?;
    println!();
    println!("步骤 3: 20 67 67 67 连发 3s（不间断），然后 3s 完全不发");
    let n = stream_secs(&manager, &uuid, Rgb::new(0x67, 0x67, 0x67), 3).await?;
    println!("已停发（发过 {n} 帧）。看接下来 3 秒。");
    tokio::time::sleep(Duration::from_secs(3)).await;
    println!("3 秒到。");
    if pause(3, "停发后：还在滚 / 停在某一格 / 灭了？") {
        return wrap_up(&manager, &uuid).await;
    }

    wrap_up(&manager, &uuid).await
}

async fn wrap_up(manager: &BleManager, uuid: &Uuid) -> anyhow::Result<()> {
    println!("\n收尾：10 全黑，后置关。");
    let _ = solid_black(manager, uuid).await;
    let _ = write_frame(manager, uuid, Power::off(Channel::Rear).frame()).await;
    manager.disconnect().await?;
    println!("已断开。把 1–3 发回来就够了。");
    Ok(())
}
