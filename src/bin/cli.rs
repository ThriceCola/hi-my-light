//! hi-my-light CLI。
//!
//! 演示如何通过终端界面使用 [`BleManager`] 后端。
//! GUI / TUI 前端将使用相同的公开 API。

use std::io::Write;
use std::time::Duration;

use uuid::Uuid;

use hi_my_light::{BleError, BleManager, Channel, Power};

// Blur 灯使用的特征值 UUID。
const TARGET_CHAR_UUID: &str = "0000fff3-0000-1000-8000-00805f9b34fb";

// ---------------------------------------------------------------------------
// CLI 辅助函数
// ---------------------------------------------------------------------------

macro_rules! read_line {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        std::io::stdout().flush().ok();
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
        buf.trim().to_string()
    }};
}

fn parse_hex_bytes(input: &str) -> Result<Vec<u8>, String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Err("输入为空".to_string());
    }
    let mut bytes = Vec::with_capacity(parts.len());
    for part in &parts {
        let b =
            u8::from_str_radix(part, 16).map_err(|_| format!("无法解析十六进制数: '{}'", part))?;
        bytes.push(b);
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// 命令循环
// ---------------------------------------------------------------------------

async fn command_loop(manager: &BleManager, char_uuid: &Uuid) -> anyhow::Result<()> {
    println!("\n已连接到设备！输入命令 (例如: 7e 07 04 ff 05 01 02 01 ef)");
    println!("特殊命令:");
    println!("  q         – 断开连接并退出");
    println!("  chars     – 列出所有特征值");
    println!("  (空输入)  – 发送默认关灯命令");

    loop {
        let input = read_line!("> ");

        match input.as_str() {
            "q" | "quit" => {
                println!("断开连接...");
                manager.disconnect().await?;
                break;
            }
            "chars" | "characteristics" => {
                match manager.characteristics().await {
                    Ok(chars) => {
                        println!("特征值列表:");
                        for (i, c) in chars.iter().enumerate() {
                            println!("  {}. {}", i + 1, c.summary());
                        }
                    }
                    Err(e) => println!("✗ 获取特征值失败: {}", e),
                }
                continue;
            }
            _ => {}
        }

        // Determine bytes to send.
        let bytes = if input.is_empty() {
            // Default: light-off command.
            Power::off(Channel::Front).frame().bytes().to_vec()
        } else {
            match parse_hex_bytes(&input) {
                Ok(b) => b,
                Err(e) => {
                    println!("✗ 输入格式错误: {}", e);
                    println!("  正确格式示例: 7e 07 04 ff 05 01 02 01 ef");
                    continue;
                }
            }
        };

        println!("发送: {:02x?}", bytes);
        match manager.write(char_uuid, &bytes, false).await {
            Ok(()) => println!("✓ 发送成功"),
            Err(BleError::NotConnected) => {
                println!("✗ 设备已断开连接");
                break;
            }
            Err(e) => println!("✗ 发送失败: {}", e),
        }

        // Small delay between commands (device-dependent).
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== hi-my-light ===");

    // 1. Create the manager.
    println!("初始化 BLE 适配器...");
    let manager = BleManager::new().await.map_err(|e| {
        eprintln!("初始化失败: {}", e);
        e
    })?;
    println!("✓ BLE 适配器就绪");

    // 2. Scan for devices.
    println!("正在扫描附近的 BLE 设备 (3 秒)...");
    let devices = manager.scan_once(Duration::from_secs(3)).await?;
    println!("扫描完成，发现 {} 个设备", devices.len());

    // 3. Let the user pick a device.
    let addr = loop {
        let devices = manager.discovered_devices().await?;

        if devices.is_empty() {
            println!("未发现任何设备。");
            println!("按 Enter 重新扫描，或输入 'q' 退出");
            let input = read_line!("> ");
            if input == "q" || input == "quit" {
                return Ok(());
            }
            // Re-scan
            manager.scan_once(Duration::from_secs(2)).await?;
            continue;
        }

        println!("\n发现的设备:");
        for (i, d) in devices.iter().enumerate() {
            println!("  {}. {}", i + 1, d.display_name());
        }

        let input = read_line!("\n选择设备编号 (1-{}), Enter 刷新, q 退出: ", devices.len());

        match input.as_str() {
            "q" | "quit" => return Ok(()),
            "" => {
                // Refresh.
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            s => {
                if let Ok(idx) = s.parse::<usize>() {
                    if idx >= 1 && idx <= devices.len() {
                        break devices[idx - 1].address;
                    }
                }
                println!("✗ 无效输入");
            }
        }
    };

    // 4. Connect.
    println!("正在连接到 {} ...", addr);
    match manager.connect(&addr).await {
        Ok(info) => println!("✓ 已连接: {}", info.display_name()),
        Err(e) => {
            eprintln!("✗ 连接失败: {}", e);
            return Err(e.into());
        }
    }

    // 5. Resolve the target characteristic.
    let char_uuid = Uuid::parse_str(TARGET_CHAR_UUID).expect("静态 UUID 无效");
    match manager.find_characteristic(&char_uuid).await {
        Ok(c) => println!("✓ 找到目标特征值: {}", c.summary()),
        Err(e) => {
            eprintln!("✗ 未找到目标特征值: {}", e);
            eprintln!("   可用特征值:");
            if let Ok(chars) = manager.characteristics().await {
                for c in &chars {
                    println!("     {}", c.summary());
                }
            }
            manager.disconnect().await?;
            return Err(e.into());
        }
    }

    // 6. Interactive command loop.
    command_loop(&manager, &char_uuid).await?;

    println!("已断开，再见！");
    Ok(())
}
