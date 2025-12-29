use crate::{models::ServerProfile, storage::logs_dir};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{broadcast, Mutex},
};
use std::io::{Read, Seek, SeekFrom};

const MAX_LOG_LINES: usize = 500;

#[derive(Clone)]
pub struct RunManager {
    inner: Arc<Mutex<RunInner>>,
    sender: broadcast::Sender<String>,
}

struct RunInner {
    child: Option<Child>,
    profile_id: Option<String>,
    pid: Option<u32>,
    started_at: Option<u64>,
    buffer: VecDeque<String>,
    log_path: Option<PathBuf>,
}

#[derive(Debug, serde::Serialize)]
pub struct RunStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub profile_id: Option<String>,
    pub started_at: Option<u64>,
}

impl RunManager {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(200);
        let inner = RunInner {
            child: None,
            profile_id: None,
            pid: None,
            started_at: None,
            buffer: VecDeque::new(),
            log_path: None,
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            sender,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    pub async fn status(&self) -> RunStatus {
        let mut inner = self.inner.lock().await;
        if let Some(child) = inner.child.as_mut() {
            if let Ok(Some(_)) = child.try_wait() {
                inner.child = None;
                inner.profile_id = None;
                inner.pid = None;
            }
        }
        RunStatus {
            running: inner.child.is_some(),
            pid: inner.pid,
            profile_id: inner.profile_id.clone(),
            started_at: inner.started_at,
        }
    }

    pub async fn tail(&self, limit: usize) -> Vec<String> {
        let inner = self.inner.lock().await;
        let start = inner.buffer.len().saturating_sub(limit);
        inner.buffer.iter().skip(start).cloned().collect()
    }

    pub async fn tail_persisted(&self, limit: usize) -> Vec<String> {
        let path = {
            let inner = self.inner.lock().await;
            inner.log_path.clone()
        };

        if let Some(path) = path {
            if let Ok(lines) = read_last_lines(path, limit).await {
                return lines;
            }
        }

        self.tail(limit).await
    }

    pub async fn start(
        &self,
        server_exe: &str,
        server_work_dir: &str,
        profile: &ServerProfile,
        config_path: &Path,
        profile_dir: &Path,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        if inner.child.is_some() {
            return Err("server already running".to_string());
        }

        let mut command = Command::new(server_exe);
        command
            .current_dir(server_work_dir)
            .arg("-config")
            .arg(config_path)
            .arg("-profile")
            .arg(profile_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if profile.load_session_save {
            command.arg("-loadSessionSave");
        }

        let mut child = command
            .spawn()
            .map_err(|err| format!("failed to start server: {err}"))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        inner.pid = child.id();
        inner.profile_id = Some(profile.profile_id.clone());
        inner.started_at = Some(current_epoch_seconds());
        inner.child = Some(child);
        inner.buffer.clear();
        inner.log_path = Some(log_file_path(profile.profile_id.as_str()));

        if let Some(stdout) = stdout {
            let manager = self.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    manager.push_line(line).await;
                }
            });
        }

        if let Some(stderr) = stderr {
            let manager = self.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    manager.push_line(line).await;
                }
            });
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut child = {
            let mut inner = self.inner.lock().await;
            inner.profile_id = None;
            inner.pid = None;
            inner.started_at = None;
            inner.child.take()
        };

        if let Some(ref mut child) = child {
            child
                .kill()
                .await
                .map_err(|err| format!("failed to stop server: {err}"))?;
            let _ = child.wait().await;
            Ok(())
        } else {
            Err("server is not running".to_string())
        }
    }

    async fn push_line(&self, line: String) {
        let log_path = {
            let mut inner = self.inner.lock().await;
            if inner.buffer.len() >= MAX_LOG_LINES {
                inner.buffer.pop_front();
            }
            inner.buffer.push_back(line.clone());
            inner.log_path.clone()
        };
        if let Some(path) = log_path {
            let _ = append_line_to_file(&path, &line).await;
        }
        let _ = self.sender.send(line);
    }
}

fn current_epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn log_file_path(profile_id: &str) -> PathBuf {
    let timestamp = current_epoch_seconds();
    logs_dir().join(format!("{profile_id}-{timestamp}.log"))
}

async fn append_line_to_file(path: &Path, line: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create log dir: {err}"))?;
    }
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .map_err(|err| format!("failed to open log file: {err}"))?;
    file.write_all(line.as_bytes())
        .await
        .map_err(|err| format!("failed to write log: {err}"))?;
    file.write_all(b"\n")
        .await
        .map_err(|err| format!("failed to write log newline: {err}"))?;
    Ok(())
}

async fn read_last_lines(path: PathBuf, limit: usize) -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path)
            .map_err(|err| format!("failed to open log file: {err}"))?;
        let mut position = file
            .metadata()
            .map_err(|err| format!("failed to read log metadata: {err}"))?
            .len();
        if position == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = Vec::new();
        let mut newline_count = 0usize;
        let chunk_size: u64 = 8192;

        while position > 0 && newline_count <= limit {
            let read_size = if position >= chunk_size {
                chunk_size
            } else {
                position
            };
            position -= read_size;
            file.seek(SeekFrom::Start(position))
                .map_err(|err| format!("failed to seek log file: {err}"))?;

            let mut chunk = vec![0u8; read_size as usize];
            file.read_exact(&mut chunk)
                .map_err(|err| format!("failed to read log file: {err}"))?;
            newline_count += chunk.iter().filter(|&&byte| byte == b'\n').count();
            buffer.splice(0..0, chunk);
        }

        let text = String::from_utf8_lossy(&buffer);
        let lines: Vec<&str> = text.lines().collect();
        let start = lines.len().saturating_sub(limit);
        Ok(lines[start..].iter().map(|line| (*line).to_string()).collect())
    })
    .await
    .map_err(|err| format!("failed to read log tail: {err}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::wrappers::BroadcastStream;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn tail_returns_last_lines() {
        let manager = RunManager::new();
        for idx in 0..10 {
            manager.push_line(format!("line-{idx}")).await;
        }

        let tail = manager.tail(3).await;
        assert_eq!(tail, vec!["line-7", "line-8", "line-9"]);
    }

    #[tokio::test]
    async fn broadcast_stream_emits_lines() {
        let manager = RunManager::new();
        let receiver = manager.subscribe();
        let mut stream = BroadcastStream::new(receiver).filter_map(|message| message.ok());

        manager.push_line("hello".to_string()).await;

        let next = stream.next().await.expect("missing line");
        assert_eq!(next, "hello");
    }
}
