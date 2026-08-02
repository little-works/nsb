use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime};

use time::macros::format_description;
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling::{Builder, RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, time::LocalTime},
    prelude::*,
    util::SubscriberInitExt,
};

use crate::utils::path::ensure_data_dir;

const LOG_KEEP_DAYS: u64 = 7;
const LOG_CLEAN_INTERVAL: Duration = Duration::from_secs(3600 * 24 * 3);

static LOG_SINK: OnceLock<Mutex<Option<LogSink>>> = OnceLock::new();
static LOG_DIR: OnceLock<Mutex<Option<CleanupTarget>>> = OnceLock::new();
static CLEANUP_THREAD: Once = Once::new();

pub fn init() {
    let log_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let timer = LocalTime::new(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3] [offset_hour sign:mandatory]:[offset_minute]"
    ));

    let _ = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_timer(timer.clone())
                .with_ansi(true)
                .with_writer(io::stdout)
                .with_filter(log_filter.clone()),
        )
        .with(
            fmt::layer()
                .with_timer(timer)
                .with_ansi(false)
                .with_writer(FileMakeWriter)
                .with_filter(log_filter),
        )
        .try_init();
}

pub fn set_log_file(path: &Path) {
    let Some(target) = CleanupTarget::from_path(path) else {
        return;
    };

    let _ = fs::create_dir_all(&target.dir);
    clean_old_logs(&target, LOG_KEEP_DAYS);

    let file_appender = match build_file_appender(&target) {
        Ok(file_appender) => file_appender,
        Err(_) => return,
    };
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    let slot = LOG_SINK.get_or_init(|| Mutex::new(None));
    if let Ok(mut sink) = slot.lock() {
        *sink = Some(LogSink {
            writer,
            _guard: guard,
        });
    }

    let dir_slot = LOG_DIR.get_or_init(|| Mutex::new(None));
    if let Ok(mut cleanup_target) = dir_slot.lock() {
        *cleanup_target = Some(target);
    }

    CLEANUP_THREAD.call_once(spawn_cleanup_thread);
}

pub fn default_log_path() -> Result<PathBuf, String> {
    let data_dir = ensure_data_dir("")?;
    Ok(data_dir.join("logs").join("nsb.log"))
}

struct LogSink {
    writer: NonBlocking,
    _guard: WorkerGuard,
}

#[derive(Clone)]
struct CleanupTarget {
    dir: PathBuf,
    prefix: String,
    suffix: Option<String>,
}

impl CleanupTarget {
    fn from_path(path: &Path) -> Option<Self> {
        let dir = path.parent()?.to_path_buf();
        let file_name = path.file_name()?.to_str()?;
        let prefix = path.file_stem()?.to_str()?.to_string();
        let suffix = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_string);

        if file_name.is_empty() || prefix.is_empty() {
            return None;
        }

        Some(Self {
            dir,
            prefix,
            suffix,
        })
    }
}

struct FileMakeWriter;

struct FileLogWriter;

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for FileMakeWriter {
    type Writer = FileLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        FileLogWriter
    }
}

impl Write for FileLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let Some(slot) = LOG_SINK.get() else {
            return Ok(buf.len());
        };

        let Ok(mut guard) = slot.lock() else {
            return Ok(buf.len());
        };

        let Some(sink) = guard.as_mut() else {
            return Ok(buf.len());
        };

        sink.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let Some(slot) = LOG_SINK.get() else {
            return Ok(());
        };

        let Ok(mut guard) = slot.lock() else {
            return Ok(());
        };

        let Some(sink) = guard.as_mut() else {
            return Ok(());
        };

        sink.writer.flush()
    }
}

fn build_file_appender(target: &CleanupTarget) -> io::Result<RollingFileAppender> {
    let mut builder = Builder::new().rotation(Rotation::DAILY);
    builder = builder.filename_prefix(&target.prefix);

    if let Some(suffix) = target.suffix.as_deref() {
        builder = builder.filename_suffix(suffix);
    }

    builder.build(&target.dir).map_err(io::Error::other)
}

fn spawn_cleanup_thread() {
    let _ = thread::Builder::new()
        .name(String::from("gui-log-cleaner"))
        .spawn(|| {
            loop {
                if let Some(target) = current_cleanup_target() {
                    clean_old_logs(&target, LOG_KEEP_DAYS);
                }
                thread::sleep(LOG_CLEAN_INTERVAL);
            }
        });
}

fn current_cleanup_target() -> Option<CleanupTarget> {
    let slot = LOG_DIR.get()?;
    let guard = slot.lock().ok()?;
    guard.clone()
}

fn clean_old_logs(target: &CleanupTarget, keep_days: u64) {
    let now = SystemTime::now();
    let max_age = Duration::from_secs(keep_days * 24 * 60 * 60);

    let Ok(entries) = fs::read_dir(&target.dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !matches_cleanup_target(&path, target) {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };

        if now.duration_since(modified).unwrap_or_default() > max_age {
            let _ = fs::remove_file(path);
        }
    }
}

fn matches_cleanup_target(path: &Path, target: &CleanupTarget) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !file_name.starts_with(&target.prefix) {
        return false;
    }

    match target.suffix.as_deref() {
        Some(expected) => path.extension().and_then(|ext| ext.to_str()) == Some(expected),
        None => path.extension().is_none(),
    }
}
