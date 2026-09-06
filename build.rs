fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        #[cfg(windows)]
        windows::embed_app_icon();
    }
}

#[cfg(windows)]
mod windows {
    use std::io::Write;
    use std::path::PathBuf;

    mod halo {
        include!("src/icon.rs");
    }

    pub fn embed_app_icon() {
        let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
        let ico_path = out.join("app-icon.ico");
        let rc_path = out.join("app-icon.rc");

        let ico = build_ico().expect("生成 Windows 图标失败");
        std::fs::write(&ico_path, ico).expect("写入 app-icon.ico");

        let ico_escaped = ico_path.display().to_string().replace('\\', "\\\\");
        let mut rc = std::fs::File::create(&rc_path).expect("创建 rc");
        writeln!(rc, "1 ICON \"{ico_escaped}\"").expect("写入 rc");

        embed_resource::compile(&rc_path, embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }

    fn build_ico() -> Result<Vec<u8>, image::ImageError> {
        let mut images = Vec::new();
        for size in [16_u32, 32, 48, 256] {
            images.push((size, halo::halo_png(size)?));
        }
        Ok(pack_png_ico(&images))
    }

    fn pack_png_ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&(images.len() as u16).to_le_bytes());

        let mut offset = 6 + 16 * images.len();
        for (size, png) in images {
            let dim = if *size >= 256 { 0u8 } else { *size as u8 };
            data.push(dim);
            data.push(dim);
            data.push(0);
            data.push(0);
            data.extend_from_slice(&1u16.to_le_bytes());
            data.extend_from_slice(&32u16.to_le_bytes());
            data.extend_from_slice(&(png.len() as u32).to_le_bytes());
            data.extend_from_slice(&(offset as u32).to_le_bytes());
            offset += png.len();
        }
        for (_, png) in images {
            data.extend_from_slice(png);
        }
        data
    }
}
