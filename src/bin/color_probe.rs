//! 逐步探针：每步只发已知帧，停住等你看灯。
//!
//!   cargo run --bin color-probe
//!
//! 看完灯上实际颜色，Enter 下一步。q 结束。
//! 做完把每步看到的颜色发回对话，不要凭印象猜。

use std::io::Write;
use std::time::Duration;

use uuid::Uuid;

use hi_my_light::{
    BleError, BleManager, Brightness, Channel, Clock, Level, Power, Query, Rgb,
};

const TARGET_CHAR_UUID: &str = "0000fff3-0000-1000-8000-00805f9b34fb";
const TICK: Duration = Duration::from_millis(100);
const FILL: u32 = 80;

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

fn pause(step: u32, total: u32, expect: &str) -> bool {
    println!();
    println!("── 看灯 ── 这一步期望对照：{expect}");
    let input = read_line!("[{step}/{total}] 记下颜色后 Enter 下一步，q 结束: ");
    input == "q" || input == "quit"
}

async fn fill_scroll(
    manager: &BleManager,
    uuid: &Uuid,
    color: Rgb,
) -> Result<(), BleError> {
    for _ in 0..FILL {
        write_frame(manager, uuid, color.scroll_frame()).await?;
        tokio::time::sleep(TICK).await;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== color-probe：对照指令和灯色 ===");
    println!("手机 App 先断开。每步终端会打印完整 hex。");

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

    let steps: &[(&str, &str)] = &[
        (
            "10 整条 ff 00 00（官方红键）",
            "若整条是红，10=整条RGB且通道顺序就是 R G B",
        ),
        (
            "10 整条 00 ff 00（官方绿键）",
            "若整条是绿，第二字节是 G",
        ),
        (
            "10 整条 00 00 ff（官方蓝键）",
            "若整条是蓝，第三字节是 B；若这步才是红，则通道对调",
        ),
        (
            "10 整条 00 00 00",
            "若整条灭，10 能清屏",
        ),
        (
            "20 连续推进 80 次 ff 00 00",
            "若整条变红，20 用的是我们发的 RGB；若仍有蓝/彩虹，20 不听这三字节当像素",
        ),
        (
            "20 连续推进 80 次 00 00 ff",
            "若整条变蓝，20 的第三字节是蓝；若变红，20 可能是 BGR",
        ),
        (
            "20 连续推进 80 次 00 ff 00",
            "对照第二字节",
        ),
        (
            "20 连续推进 80 次 00 00 00",
            "若整条灭，20 推进黑能清带；若还亮着，20 不是移位写色",
        ),
    ];
    let total = steps.len() as u32;

    let colors_10 = [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::BLACK];
    let colors_20 = [Rgb::RED, Rgb::BLUE, Rgb::GREEN, Rgb::BLACK];

    for (i, (title, expect)) in steps.iter().enumerate() {
        let n = (i as u32) + 1;
        println!();
        println!("步骤 {n}: {title}");
        if i < 4 {
            let color = colors_10[i];
            let frame = color.frame();
            println!("发送 1 次  {}", frame.hex());
            write_frame(&manager, &uuid, frame).await?;
        } else {
            let color = colors_20[i - 4];
            let frame = color.scroll_frame();
            println!("发送 {FILL} 次  {}  （间隔 100ms）", frame.hex());
            fill_scroll(&manager, &uuid, color).await?;
            println!("发完。");
        }
        if pause(n, total, expect) {
            break;
        }
    }

    println!("\n收尾：后置关。");
    let _ = write_frame(&manager, &uuid, Power::off(Channel::Rear).frame()).await;
    manager.disconnect().await?;
    println!("已断开。把 1–8 步灯上看到的颜色发回来。");
    Ok(())
}
