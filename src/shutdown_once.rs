use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// 关机关灯只跑一次；后来的调用必须等这一次写完，不能立刻返回。
///
/// 否则 logind 线程还在连灯，SIGTERM 轮询线程会看到“已经开始”然后
/// `process::exit`，把关灯掐死。
pub struct ShutdownOnce {
    lock: Mutex<()>,
    started: AtomicBool,
}

impl ShutdownOnce {
    pub const fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            started: AtomicBool::new(false),
        }
    }

    pub fn already_started(&self) -> bool {
        self.started.load(Ordering::SeqCst)
    }

    pub fn mark_started(&self) {
        self.started.store(true, Ordering::SeqCst);
    }

    pub fn with_lock<R, F: FnOnce() -> R>(&self, work: F) -> R {
        let _guard = self.lock.lock().expect("shutdown-once");
        work()
    }
}

/// 关机线程可能已经把磁盘上的 restore 写成 true。GUI 普通 persist
/// 内存里往往还是 false，不能把这个标志盖掉；只有亮灯成功后才能 clear。
pub fn restore_flag_to_write(disk: bool, memory: bool, clear: bool) -> bool {
    if clear {
        false
    } else {
        memory || disk
    }
}

/// 快照里没地址时（后台很久、绑定丢了）用 session 记住的灯。
pub fn turn_off_addr(snap_addr: Option<&str>, session_addr: Option<&str>) -> Option<String> {
    snap_addr
        .map(str::trim)
        .filter(|addr| !addr.is_empty())
        .map(str::to_string)
        .or_else(|| {
            session_addr
                .map(str::trim)
                .filter(|addr| !addr.is_empty())
                .map(str::to_string)
        })
}

/// D-Bus 流结束后必须重连拿 inhibit，不能退出守护。
pub fn logind_should_reconnect(signal_stream_ended: bool) -> bool {
    signal_stream_ended
}

/// 设置开着就关灯。不能用 UI 里“现在亮不亮”做快路径：
/// 断开或进托盘之后快照经常是灭的，实体灯还亮着。
pub fn should_turn_off(pref_enabled: bool) -> bool {
    pref_enabled
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::time::{Duration, Instant};

    #[test]
    fn later_caller_waits_until_first_finishes() {
        let once = Arc::new(ShutdownOnce::new());
        let finished = Arc::new(AtomicBool::new(false));
        let second_saw_done = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(AtomicUsize::new(0));

        std::thread::scope(|scope| {
            scope.spawn(|| {
                once.with_lock(|| {
                    if once.already_started() {
                        return;
                    }
                    once.mark_started();
                    calls.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(80));
                    finished.store(true, Ordering::SeqCst);
                });
            });
            std::thread::sleep(Duration::from_millis(15));
            scope.spawn(|| {
                let start = Instant::now();
                once.with_lock(|| {
                    if once.already_started() {
                        assert!(
                            finished.load(Ordering::SeqCst),
                            "second caller entered lock before turn-off finished"
                        );
                        return;
                    }
                    calls.fetch_add(1, Ordering::SeqCst);
                });
                assert!(
                    finished.load(Ordering::SeqCst),
                    "second caller returned before turn-off finished"
                );
                assert!(
                    start.elapsed() >= Duration::from_millis(40),
                    "second caller returned too quickly; would process::exit mid-write"
                );
                second_saw_done.store(true, Ordering::SeqCst);
            });
        });

        assert!(second_saw_done.load(Ordering::SeqCst));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn unbound_attempt_does_not_block_later_turn_off() {
        let once = ShutdownOnce::new();
        once.with_lock(|| {
            assert!(!once.already_started());
        });
        let mut ran = false;
        once.with_lock(|| {
            if once.already_started() {
                return;
            }
            once.mark_started();
            ran = true;
        });
        assert!(ran);
    }

    #[test]
    fn persist_must_not_clobber_shutdown_restore_flag() {
        assert!(
            restore_flag_to_write(true, false, false),
            "GUI persist with memory=false must keep disk restore=true"
        );
        assert!(restore_flag_to_write(false, true, false));
        assert!(!restore_flag_to_write(true, false, true));
        assert!(!restore_flag_to_write(false, false, false));
    }

    #[test]
    fn pref_on_turns_off_even_if_ui_thinks_lamp_is_off() {
        assert!(should_turn_off(true));
        assert!(!should_turn_off(false));
    }

    #[test]
    fn empty_snapshot_falls_back_to_session_addr() {
        assert_eq!(
            turn_off_addr(None, Some("BE:28:69:00:00:DF")).as_deref(),
            Some("BE:28:69:00:00:DF")
        );
        assert_eq!(
            turn_off_addr(Some(""), Some("BE:28:69:00:00:DF")).as_deref(),
            Some("BE:28:69:00:00:DF")
        );
        assert_eq!(
            turn_off_addr(Some("AA:BB"), Some("CC:DD")).as_deref(),
            Some("AA:BB")
        );
        assert_eq!(turn_off_addr(None, None), None);
    }

    #[test]
    fn logind_stream_end_must_reconnect() {
        assert!(logind_should_reconnect(true));
    }
}
