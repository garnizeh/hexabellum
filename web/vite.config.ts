import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
        configure: (proxy) => {
          proxy.on('error', (_err, _req, res) => {
            if (!res.headersSent) {
              res.writeHead(503, { 'Content-Type': 'application/json' });
              res.end(
                JSON.stringify({
                  error: 'Backend Server Offline',
                  message:
                    'Hexabellum server is not reachable on localhost:3000. Start it with "make run-server" or "cargo run -p hexabellum-server".',
                }),
              );
            }
          });
        },
      },
      '/ws': {
        target: 'ws://localhost:3000',
        ws: true,
        configure: (proxy) => {
          proxy.on('error', () => {
            // Silently handle proxy websocket disconnects when server is down
          });
        },
      },
    },
  },
});

