use crate::views::layout::{breadcrumb, render_layout};
use backend::models::ServerProfile;

pub fn render_run_logs_page(profiles: &[ServerProfile]) -> String {
    let mut options = String::new();
    for profile in profiles {
        options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = html_escape::encode_text(&profile.profile_id),
            name = html_escape::encode_text(&profile.display_name),
        ));
    }

    if options.is_empty() {
        options.push_str("<option value=\"\">No profiles available</option>");
    }

    let content = format!(
        r#"<h1 class="h3 mb-3">Run & Logs</h1>
        <div class="card card-body mb-3">
          <div class="row g-3 align-items-end">
            <div class="col-md-6">
              <label class="form-label" for="profile-select">Profile</label>
              <select class="form-select arssm-input" id="profile-select">{options}</select>
            </div>
            <div class="col-md-6">
              <div class="d-flex gap-2">
                <button class="btn btn-arssm-primary" id="start-btn">Start</button>
                <button class="btn btn-arssm-danger" id="stop-btn">Stop</button>
              </div>
            </div>
          </div>
          <p class="mt-3 mb-0"><strong>Status:</strong> <span id="status-text">unknown</span></p>
        </div>
        <div class="card">
          <div class="card-header">Live Log</div>
          <div class="card-body">
            <pre class="arssm-log p-3" id="log-output" style="height: 360px; overflow-y: auto;"></pre>
          </div>
        </div>
        <script>
          const statusText = document.getElementById('status-text');
          const logOutput = document.getElementById('log-output');
          const profileSelect = document.getElementById('profile-select');

          function appendLine(line) {
            logOutput.textContent += line + '\n';
            logOutput.scrollTop = logOutput.scrollHeight;
          }

          async function refreshStatus() {
            const response = await fetch('/api/run/status');
            const data = await response.json();
            statusText.textContent = data.running ? ('running (pid ' + data.pid + ')') : 'stopped';
          }

          document.getElementById('start-btn').addEventListener('click', async () => {
            const profile_id = profileSelect.value;
            const response = await fetch('/api/run/start', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ profile_id })
            });
            if (!response.ok) {
              const text = await response.text();
              alert(text);
            }
            refreshStatus();
          });

          document.getElementById('stop-btn').addEventListener('click', async () => {
            await fetch('/api/run/stop', { method: 'POST' });
            refreshStatus();
          });

          const eventSource = new EventSource('/api/run/logs/stream');
          eventSource.onmessage = (event) => {
            appendLine(event.data);
          };

          refreshStatus();
        </script>"#,
        options = options,
    );

    render_layout(
        "ARSSM Run & Logs",
        "run",
        vec![breadcrumb("Run / Logs", None)],
        &content,
    )
}
