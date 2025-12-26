use crate::storage::AppSettings;
use crate::models::ServerProfile;
use std::{
    collections::VecDeque,
    path::Path,
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{broadcast, Mutex},
};

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

    pub async fn start(
        &self,
        settings: &AppSettings,
        profile: &ServerProfile,
        config_path: &Path,
        profile_dir: &Path,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        if inner.child.is_some() {
            return Err("server already running".to_string());
        }

        let mut command = Command::new(&settings.reforger_server_exe);
        command
            .current_dir(&settings.reforger_server_work_dir)
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
        let mut inner = self.inner.lock().await;
        if inner.buffer.len() >= MAX_LOG_LINES {
            inner.buffer.pop_front();
        }
        inner.buffer.push_back(line.clone());
        let _ = self.sender.send(line);
    }
}

fn current_epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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
