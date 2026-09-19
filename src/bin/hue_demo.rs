//! 后置滚动换色 demo。线上三字节就是 RGB，不是 HSV。
//!
//! 抓包：色盘 / 声控都是 `7e 07 05 03 RR GG BB TAG ef`。
//! 红键 `ff 00 00`，绿 `00 ff 00`，蓝 `00 00 ff`。若当 HSV 解，红键会变成黑。
//!
//! 本文件只在 RGB 里插值，不再调用 from_hue / from_hsv。
//!
//!   cargo run --bin hue-demo
//!   cargo run --bin hue-demo -- --warm

use std::io::Write;
use std::time::Duration;

use uuid::Uuid;

use hi_my_light::{BleError, BleManager, Brightness, Channel, Clock, Level, Power, Query, Rgb};

const TARGET_CHAR_UUID: &str = "0000fff3-0000-1000-8000-00805f9b34fb";
const TICK: Duration = Duration::from_millis(100);
/// 绕 RGB 立方体六条棱走一圈约 9 秒。
const WHEEL_STEP: f32 = 6.0 / 90.0;
const WARM_STEP: f32 = 1.0 / 40.0;
/// 上一轮颜色还在灯带上，先推进这么多格黑把它们挤出去。
const FLUSH_TICKS: u32 = 20;
const WARM_RED: Rgb = Rgb::new(0xff, 0x28, 0x00);
const WARM_AMBER: Rgb = Rgb::new(0xff, 0xa0, 0x00);
/// 色盘同款：一端常为 00 或 ff。
const RGB_CORNERS: [Rgb; 6] = [
    Rgb::RED,
    Rgb::YELLOW,
    Rgb::GREEN,
    Rgb::CYAN,
    Rgb::BLUE,
    Rgb::MAGENTA,
];

macro_rules! read_line {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        std::io::stdout().flush().ok();
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
        buf.trim().to_string()
    }};
}

fn warm() -> bool {
    std::env::args().any(|a| a == "--warm")
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
        let input = read_line!("\n选择设备编号 (1-{}), Enter 刷新, q 退出: ", devices.len());
        match input.as_str() {
            "q" | "quit" => anyhow::bail!("已取消"),
            "" => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            s => {
                if let Ok(idx) = s.parse::<usize>() {
                    if idx >= 1 && idx <= devices.len() {
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

fn next_color(phase: &mut f32, warm_only: bool) -> Rgb {
    let color = if warm_only {
        *phase = (*phase + WARM_STEP).rem_euclid(2.0);
        let t = if *phase <= 1.0 { *phase } else { 2.0 - *phase };
        WARM_RED.lerp(WARM_AMBER, t)
    } else {
        *phase = (*phase + WHEEL_STEP).rem_euclid(6.0);
        let i = phase.floor() as usize % 6;
        let f = phase.fract();
        RGB_CORNERS[i].lerp(RGB_CORNERS[(i + 1) % 6], f)
    };
    debug_assert!(!warm_only || color.b == 0);
    color
}

async fn flush_black(manager: &BleManager, uuid: &Uuid) -> Result<(), BleError> {
    println!("先推进 {FLUSH_TICKS} 格黑，挤掉灯带上上一轮的蓝/七色…");
    for _ in 0..FLUSH_TICKS {
        write_frame(manager, uuid, Rgb::BLACK.scroll_frame()).await?;
        tokio::time::sleep(TICK).await;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let warm_only = warm();
    println!("=== RGB 滚动 demo（不是 HSV）===");
    println!("帧里三个字节就是 R G B。手机 App 先断开。");

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

    flush_black(&manager, &uuid).await?;

    if warm_only {
        println!("--warm：RGB 红↔琥珀插值，B 固定 0。Ctrl+C 结束。");
    } else {
        println!("沿 RGB 六棱缓转（红黄绿青蓝品红，会有蓝）。暖色：--warm");
    }

    let mut phase = 0.0f32;
    let mut ticks: u32 = 0;
    let mut interval = tokio::time::interval(TICK);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = interval.tick() => {
                let color = next_color(&mut phase, warm_only);
                if ticks % 10 == 0 {
                    println!(
                        "推 {:02x} {:02x} {:02x}",
                        color.r, color.g, color.b
                    );
                }
                ticks += 1;
                if let Err(e) = write_frame(&manager, &uuid, color.scroll_frame()).await {
                    eprintln!("写入失败: {e}");
                    break;
                }
            }
        }
    }

    println!("\n收尾：推进黑，后置关。");
    for _ in 0..FLUSH_TICKS {
        let _ = write_frame(&manager, &uuid, Rgb::BLACK.scroll_frame()).await;
        tokio::time::sleep(TICK).await;
    }
    let _ = write_frame(&manager, &uuid, Power::off(Channel::Rear).frame()).await;
    manager.disconnect().await?;
    println!("已断开。");
    Ok(())
}
