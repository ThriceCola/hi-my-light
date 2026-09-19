//! 后置声控滚动换色 demo（无麦克风，用假节奏模拟有声/安静）。
//!
//! 按抓包：每 100 ms 写一帧 `7e 07 05 03 RR GG BB 20 ef`。
//! 有声推进七色，安静推进黑。手机 App 先断开。
//!
//!   cargo run --bin mic-demo
//!   cargo run --bin mic-demo -- --always

use std::io::Write;
use std::time::Duration;

use uuid::Uuid;

use hi_my_light::{
    BleError, BleManager, Brightness, Channel, Clock, Level, Power, Query, Rgb,
};

const TARGET_CHAR_UUID: &str = "0000fff3-0000-1000-8000-00805f9b34fb";
const TICK: Duration = Duration::from_millis(100);
/// 模拟一拍：安静 1.2s，有声 3.6s（够灯带滚过几格七色）。
const QUIET_TICKS: u32 = 12;
const LOUD_TICKS: u32 = 36;

macro_rules! read_line {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        std::io::stdout().flush().ok();
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
        buf.trim().to_string()
    }};
}

fn always_roll() -> bool {
    std::env::args().any(|a| a == "--always")
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let always = always_roll();
    println!("=== 滚动换色 demo（模拟声控，无麦克风）===");
    println!("电脑连灯前请先关掉手机 App 里的连接。");

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

    if always {
        println!("--always：一直推进七色。Ctrl+C 结束。");
    } else {
        println!("模拟拍手：1.2s 安静（推进黑）+ 3.6s 有声（滚动七色）。Ctrl+C 结束。");
    }

    let mut idx = 0usize;
    let mut beat: u32 = 0;
    let period = QUIET_TICKS + LOUD_TICKS;
    let mut interval = tokio::time::interval(TICK);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = interval.tick() => {
                let loud = always || beat >= QUIET_TICKS;
                let color = if loud {
                    let color = Rgb::MIC_SCROLL[idx];
                    idx = (idx + 1) % Rgb::MIC_SCROLL.len();
                    color
                } else {
                    Rgb::BLACK
                };
                if let Err(e) = write_frame(&manager, &uuid, color.scroll_frame()).await {
                    eprintln!("写入失败: {e}");
                    break;
                }
                if !always && beat == 0 {
                    println!("安静");
                }
                if !always && beat == QUIET_TICKS {
                    println!("有声");
                }
                beat = (beat + 1) % period;
            }
        }
    }

    println!("\n收尾：推进黑，后置关。");
    for _ in 0..12 {
        let _ = write_frame(&manager, &uuid, Rgb::BLACK.scroll_frame()).await;
        tokio::time::sleep(TICK).await;
    }
    let _ = write_frame(&manager, &uuid, Power::off(Channel::Rear).frame()).await;
    manager.disconnect().await?;
    println!("已断开。");
    Ok(())
}
