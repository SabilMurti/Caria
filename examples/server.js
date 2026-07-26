import http from 'http';
import fs from 'fs';
import path from 'path';
import { execFile } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PORT = 8080;

const server = http.createServer((req, res) => {
    const url = new URL(req.url, `http://${req.headers.host}`);

    if (url.pathname === '/' || url.pathname === '/index.html') {
        fs.readFile(path.join(__dirname, 'index.html'), (err, content) => {
            if (err) {
                res.writeHead(500, { 'Content-Type': 'text/plain' });
                res.end('Error loading index.html');
            } else {
                res.writeHead(200, { 'Content-Type': 'text/html' });
                res.end(content);
            }
        });
    } else if (url.pathname === '/api/calculate') {
        const op = url.searchParams.get('op');
        const a = url.searchParams.get('a');
        const b = url.searchParams.get('b');

        if (!op || !a || !b) {
            res.writeHead(400, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ success: false, error: 'Missing parameters' }));
            return;
        }

        // Execute native Calesteria calculator binary
        const binPath = path.join(__dirname, 'calculator');
        execFile(binPath, {
            env: {
                ...process.env,
                OP: op,
                A: a,
                B: b
            }
        }, (error, stdout, stderr) => {
            if (error) {
                res.writeHead(500, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({ success: false, error: error.message }));
                return;
            }

            const resultVal = parseInt(stdout.trim());
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({ success: true, result: resultVal }));
        });
    } else {
        res.writeHead(404, { 'Content-Type': 'text/plain' });
        res.end('Not Found');
    }
});

server.listen(PORT, () => {
    console.log(`Calculator Server running at http://localhost:${PORT}`);
});
