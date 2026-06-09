// Minimal static file server for the debugger preview. Serves the repo root so
// that /web/index.html and its ./pkg/*.wasm load with correct MIME types.
// (python3 -m http.server is unavailable under the preview sandbox.)
const http = require('http');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const PORT = Number(process.env.PORT) || 8761;
const MIME = {
  '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm',
  '.json': 'application/json', '.css': 'text/css', '.gba': 'application/octet-stream',
  '.bin': 'application/octet-stream', '.png': 'image/png',
};

http.createServer((req, res) => {
  let p = decodeURIComponent(req.url.split('?')[0]);
  // Redirect the bare root to the real page path so the browser's document URL
  // is /web/ and relative imports (./app.js, ./pkg/*.wasm) resolve correctly.
  if (p === '/') { res.writeHead(302, { Location: '/web/index.html' }); return res.end(); }
  const fp = path.join(ROOT, p);
  if (!fp.startsWith(ROOT)) { res.writeHead(403); return res.end('forbidden'); }
  fs.readFile(fp, (err, data) => {
    if (err) { res.writeHead(404); return res.end('not found'); }
    res.writeHead(200, { 'Content-Type': MIME[path.extname(fp)] || 'application/octet-stream' });
    res.end(data);
  });
}).listen(PORT, () => console.log(`static server on :${PORT} root=${ROOT}`));
