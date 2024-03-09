import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const port = 3000;

http.createServer((req, res) => {
  const basePath = './public';
  const filePath = path.join(basePath, req.url === '/' ? 'index.html' : req.url);
  const extname = String(path.extname(filePath)).toLowerCase();
  const mimeTypes = {
    '.html': 'text/html',
    '.js': 'text/javascript',
    '.css': 'text/css',
    '.json': 'application/json',
    '.png': 'image/png',
    '.jpg': 'image/jpg',
    '.gif': 'image/gif',
    '.svg': 'image/svg+xml',
    '.wav': 'audio/wav',
    '.mp4': 'video/mp4',
    '.woff': 'application/font-woff',
    '.ttf': 'application/font-ttf',
    '.eot': 'application/vnd.ms-fontobject',
    '.otf': 'application/font-otf',
    '.wasm': 'application/wasm'
  };

  const contentType = mimeTypes[extname] ?? 'application/octet-stream';
  /** @type {undefined | string} */
  const acceptEncoding = req.headers['accept-encoding'];
  const gzipSupported = Boolean(acceptEncoding && acceptEncoding.includes('gzip'));

  fs.readFile(filePath, (error, content) => {
    if (error) {
      if (error.code === 'ENOENT') {
        fs.readFile('./public/404.html', (error, content) => {
          res.writeHead(404, { 'Content-Type': 'text/html' });
          res.end(content, 'utf-8');
        });
      } else {
        res.writeHead(500);
        res.end(`Sorry, check with the site admin for error: ${error.code}`);
      }
    } else {
      if (gzipSupported) {
        zlib.gzip(content, (error, compressedContent) => {
          if (error) {
            res.writeHead(500);
            res.end(`Error compressing content: ${error.code}`);
          } else {
            res.writeHead(200, {
              'Content-Type': contentType,
              'Content-Encoding': 'gzip'
            });
            res.end(compressedContent, 'utf-8');
          }
        });
      } else {
        res.writeHead(200, { 'Content-Type': contentType });
        res.end(content, 'utf-8');
      }
    }
  });
}).listen(port, () => {
  console.log(`Server running at http://localhost:${port}/`);
});
