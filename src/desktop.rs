use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const APP_ID: &str = "hi-my-light";
pub const APP_NAME: &str = "HML";

pub fn start_hidden() -> bool {
    std::env::args().any(|arg| matches!(arg.as_str(), "--background" | "--tray" | "--hidden"))
}

pub fn install_title(installed: bool) -> &'static str {
    if installed {
        "重新安装到开始菜单"
    } else {
        "安装到开始菜单"
    }
}

pub fn install_hint(installed: bool) -> &'static str {
    if installed {
        "已加入本机开始菜单，点这里可覆盖更新"
    } else {
        #[cfg(windows)]
        {
            "复制到用户目录，并加入开始菜单"
        }
        #[cfg(target_os = "linux")]
        {
            "复制到 ~/.local/bin，并加入应用菜单"
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            "仅支持 Linux 和 Windows"
        }
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(tmp, path)
}

fn copy_current_binary(dest: &Path) -> io::Result<PathBuf> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let src = std::env::current_exe()?;
    if same_file(&src, dest) {
        return Ok(dest.to_path_buf());
    }
    let tmp = dest.with_extension("bin.tmp");
    fs::copy(&src, &tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&tmp)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&tmp, perms)?;
    }
    fs::rename(&tmp, dest)?;
    Ok(dest.to_path_buf())
}

fn same_file(a: &Path, b: &Path) -> bool {
    let (Ok(a), Ok(b)) = (fs::canonicalize(a), fs::canonicalize(b)) else {
        return false;
    };
    a == b
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    fn home_dir() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn data_home() -> PathBuf {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".local/share"))
    }

    fn config_home() -> PathBuf {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".config"))
    }

    pub fn bin_path() -> PathBuf {
        home_dir().join(".local/bin").join(APP_ID)
    }

    pub fn menu_entry_path() -> PathBuf {
        data_home()
            .join("applications")
            .join(format!("{APP_ID}.desktop"))
    }

    fn autostart_path() -> PathBuf {
        config_home()
            .join("autostart")
            .join(format!("{APP_ID}.desktop"))
    }

    fn icon_svg_path() -> PathBuf {
        data_home()
            .join("icons/hicolor/scalable/apps")
            .join(format!("{APP_ID}.svg"))
    }

    fn icon_png_path(size: u32) -> PathBuf {
        data_home()
            .join(format!("icons/hicolor/{size}x{size}/apps"))
            .join(format!("{APP_ID}.png"))
    }

    pub fn is_menu_installed() -> bool {
        menu_entry_path().is_file() && bin_path().is_file()
    }

    fn desktop_body(exec: &Path) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Version=1.0\n\
             Name={APP_NAME}\n\
             GenericName=吊灯控制\n\
             Comment=控制 THUNDEROBOT 吊灯\n\
             Exec={}\n\
             Icon={APP_ID}\n\
             Terminal=false\n\
             Categories=Utility;\n\
             StartupNotify=false\n\
             StartupWMClass={APP_ID}\n\
             X-KDE-StartupNotify=false\n",
            shell_escape(exec)
        )
    }

    fn autostart_body(exec: &Path) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Version=1.0\n\
             Name={APP_NAME}\n\
             Comment=托盘驻留\n\
             Exec={} --background\n\
             Icon={APP_ID}\n\
             Terminal=false\n\
             Categories=Utility;\n\
             StartupNotify=false\n\
             X-GNOME-Autostart-enabled=true\n\
             X-KDE-autostart-after=panel\n\
             X-KDE-StartupNotify=false\n",
            shell_escape(exec)
        )
    }

    fn shell_escape(path: &Path) -> String {
        let s = path.display().to_string();
        if s.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '\'')) {
            format!("\"{}\"", s.replace('"', "\\\""))
        } else {
            s
        }
    }

    pub fn install_user() -> io::Result<PathBuf> {
        let exec = copy_current_binary(&bin_path())?;
        write_file(&icon_svg_path(), crate::icon::HALO_SVG.as_bytes())?;
        for size in [32_u32, 64, 128, 256] {
            let png = crate::icon::halo_png(size)
                .map_err(|err| io::Error::other(err))?;
            write_file(&icon_png_path(size), &png)?;
        }
        write_file(&menu_entry_path(), desktop_body(&exec).as_bytes())?;
        if autostart_path().exists() {
            write_file(&autostart_path(), autostart_body(&exec).as_bytes())?;
        }
        refresh_desktop_cache();
        Ok(exec)
    }

    pub fn sync_autostart(enabled: bool) -> io::Result<()> {
        let path = autostart_path();
        if enabled {
            let exec = if bin_path().is_file() {
                bin_path()
            } else {
                install_user()?
            };
            write_file(&path, autostart_body(&exec).as_bytes())?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn refresh_desktop_cache() {
        let apps = data_home().join("applications");
        let _ = std::process::Command::new("update-desktop-database")
            .arg(&apps)
            .status();
        let _ = std::process::Command::new("kbuildsycoca6")
            .arg("--noincremental")
            .status();
        let _ = std::process::Command::new("kbuildsycoca5")
            .arg("--noincremental")
            .status();
        let icon_dir = data_home().join("icons/hicolor");
        let _ = std::process::Command::new("gtk-update-icon-cache")
            .args(["-f", "-t"])
            .arg(&icon_dir)
            .status();
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::os::windows::ffi::OsStrExt;

    fn local_app_data() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|home| PathBuf::from(home).join("AppData").join("Local"))
            })
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn roaming_app_data() -> PathBuf {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|home| PathBuf::from(home).join("AppData").join("Roaming"))
            })
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn bin_path() -> PathBuf {
        local_app_data().join(APP_ID).join(format!("{APP_ID}.exe"))
    }

    pub fn menu_entry_path() -> PathBuf {
        roaming_app_data()
            .join("Microsoft/Windows/Start Menu/Programs")
            .join(format!("{APP_NAME}.lnk"))
    }

    fn menu_bat_path() -> PathBuf {
        roaming_app_data()
            .join("Microsoft/Windows/Start Menu/Programs")
            .join(format!("{APP_NAME}.bat"))
    }

    pub fn is_menu_installed() -> bool {
        bin_path().is_file() && (menu_entry_path().is_file() || menu_bat_path().is_file())
    }

    fn icon_ico_path() -> PathBuf {
        local_app_data().join(APP_ID).join(format!("{APP_ID}.ico"))
    }

    pub fn install_user() -> io::Result<PathBuf> {
        let exec = copy_current_binary(&bin_path())?;
        let ico = crate::icon::halo_ico(256).map_err(io::Error::other)?;
        write_file(&icon_ico_path(), &ico)?;
        if create_shortcut(&menu_entry_path(), &exec, &icon_ico_path()).is_err() {
            write_file(
                &menu_bat_path(),
                format!("@echo off\r\nstart \"\" \"{}\"\r\n", exec.display()).as_bytes(),
            )?;
        } else if menu_bat_path().exists() {
            let _ = fs::remove_file(menu_bat_path());
        }
        Ok(exec)
    }

    pub fn sync_autostart(enabled: bool) -> io::Result<()> {
        let exec = if enabled {
            if bin_path().is_file() {
                bin_path()
            } else {
                install_user()?
            }
        } else {
            bin_path()
        };
        set_run_key(enabled, &exec)
    }

    fn ps_single_quote(path: &Path) -> String {
        path.display().to_string().replace('\'', "''")
    }

    fn create_shortcut(lnk: &Path, target: &Path, icon: &Path) -> io::Result<()> {
        if let Some(parent) = lnk.parent() {
            fs::create_dir_all(parent)?;
        }
        let work = target.parent().unwrap_or(target);
        let script = format!(
            "$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{}'); \
             $s.TargetPath = '{}'; \
             $s.WorkingDirectory = '{}'; \
             $s.IconLocation = '{}'; \
             $s.Save()",
            ps_single_quote(lnk),
            ps_single_quote(target),
            ps_single_quote(work),
            ps_single_quote(icon),
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .status()?;
        if status.success() && lnk.is_file() {
            Ok(())
        } else {
            Err(io::Error::other("无法创建开始菜单快捷方式"))
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(Some(0))
            .collect()
    }

    fn set_run_key(enabled: bool, exec: &Path) -> io::Result<()> {
        use windows_sys::Win32::Foundation::ERROR_SUCCESS;
        use windows_sys::Win32::System::Registry::{
            HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegCreateKeyExW,
            RegDeleteValueW, RegSetValueExW,
        };

        const RUN_SUBKEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let subkey = wide(RUN_SUBKEY);
        let name = wide(APP_ID);
        let mut key = std::ptr::null_mut();
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut key,
                std::ptr::null_mut(),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        let result = if enabled {
            let value = format!("\"{}\" --background", exec.display());
            let data = wide(&value);
            let status = unsafe {
                RegSetValueExW(
                    key,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    data.as_ptr().cast(),
                    (data.len() * 2) as u32,
                )
            };
            if status == ERROR_SUCCESS {
                Ok(())
            } else {
                Err(io::Error::from_raw_os_error(status as i32))
            }
        } else {
            let status = unsafe { RegDeleteValueW(key, name.as_ptr()) };
            if status == ERROR_SUCCESS || status == 2 {
                Ok(())
            } else {
                Err(io::Error::from_raw_os_error(status as i32))
            }
        };
        unsafe { RegCloseKey(key) };
        result
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod other {
    use super::*;

    pub fn menu_entry_path() -> PathBuf {
        PathBuf::from(APP_ID)
    }

    pub fn is_menu_installed() -> bool {
        false
    }

    pub fn install_user() -> io::Result<PathBuf> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "安装到开始菜单仅支持 Linux 和 Windows",
        ))
    }

    pub fn sync_autostart(_enabled: bool) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "开机自启动仅支持 Linux 和 Windows",
        ))
    }
}

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(windows)]
use win as platform;
#[cfg(not(any(target_os = "linux", windows)))]
use other as platform;

pub fn menu_entry_path() -> PathBuf {
    platform::menu_entry_path()
}

pub fn is_menu_installed() -> bool {
    platform::is_menu_installed()
}

pub fn install_user() -> io::Result<PathBuf> {
    platform::install_user()
}

pub fn sync_autostart(enabled: bool) -> io::Result<()> {
    platform::sync_autostart(enabled)
}
