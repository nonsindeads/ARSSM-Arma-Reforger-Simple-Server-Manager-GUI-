pub fn health_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>ARSSM Health</title>
    <link
      href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css"
      rel="stylesheet"
      integrity="sha384-QWTKZyjpPEjISv5WaRU9OFeRpok6YctnYmDr5pNlyT2bRjXh0JMhjY6hW+ALEwIH"
      crossorigin="anonymous"
    >
    <link href="https://fonts.googleapis.com/css2?family=Rajdhani:wght@400;500;600;700&display=swap" rel="stylesheet">
    <link href="/web/css/theme.css" rel="stylesheet">
  </head>
  <body>
    <div class="container py-4">
      <div class="arssm-panel p-4">
        <h1 class="mb-3">ARSSM</h1>
        <p class="text-muted">Status: ok</p>
        <label class="form-label text-muted" for="workshop-url">Workshop URL</label>
        <input class="form-control arssm-input" id="workshop-url" type="text" value="https://reforger.armaplatform.com/workshop/595F2BF2F44836FB-RHS-StatusQuo">
        <button class="btn btn-arssm-primary mt-3" id="resolve">Resolve</button>
        <h2 class="mt-4">Result</h2>
        <pre class="arssm-log p-3" id="output">Waiting for input.</pre>
      </div>
    </div>
    <script>
      const button = document.getElementById('resolve');
      const output = document.getElementById('output');
      button.addEventListener('click', async () => {
        output.textContent = 'Resolving...';
        const url = document.getElementById('workshop-url').value;
        try {
          const response = await fetch('/api/workshop/resolve', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ url, max_depth: 5 })
          });
          const data = await response.json();
          output.textContent = JSON.stringify(data, null, 2);
        } catch (error) {
          output.textContent = 'Error: ' + error;
        }
      });
    </script>
  </body>
</html>
"#
}
