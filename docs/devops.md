# HEXABELLUM

## DevOps & Deployment Guide

---

## Document Version
`DevOps v1.0 — Aligned with Phase 7 Technical Spec`

---

## 1. Infrastructure Overview

### 1.1 What We're Deploying

Hexabellum consists of three deployable components:

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Game Server** | Rust (Axum + Tokio) | Authoritative simulation, WebSocket, match management |
| **Web Client** | TypeScript + PixiJS + WASM | Player-facing UI, rendering, input |
| **Database** | PostgreSQL | Player accounts, match history, rankings (future) |
| **Cache / Pub-Sub** | Redis | Session management, matchmaking queue (future) |

### 1.2 Architecture Diagram

```mermaid
flowchart TD
    subgraph "Player Browser"
        Client[Web Client<br/>TypeScript + PixiJS + WASM]
    end

    subgraph "Cloud Infrastructure"
        LB[Load Balancer / Reverse Proxy<br/>Nginx / Caddy / Cloud LB]
        
        subgraph "Application Tier"
            GS1[Game Server Instance 1<br/>Rust / Axum]
            GS2[Game Server Instance 2<br/>Rust / Axum]
            GSN[Game Server Instance N<br/>Rust / Axum]
        end

        subgraph "Data Tier"
            DB[(PostgreSQL<br/>Accounts, Matches, Stats)]
            Redis[(Redis<br/>Sessions, Matchmaking)]
        end

        CDN[CDN / Static Hosting<br/>Client Assets]
    end

    Client -->|HTTPS| CDN
    Client -->|WSS| LB
    LB --> GS1
    LB --> GS2
    LB --> GSN
    GS1 --> DB
    GS1 --> Redis
    GS2 --> DB
    GS2 --> Redis
```

### 1.3 Communication Protocols

| Path | Protocol | Purpose |
|------|----------|---------|
| Client → CDN | HTTPS | Static assets (JS, WASM, images) |
| Client → Load Balancer | WSS (WebSocket Secure) | Real-time game communication |
| Load Balancer → Game Server | WS (internal) | Forward WebSocket connections |
| Game Server → Database | TCP | Persistent data storage |
| Game Server → Redis | TCP | Session/cache/matchmaking |

---

## 2. Development Environment

### 2.1 Prerequisites

Every developer needs:

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
rustup target add wasm32-unknown-unknown

# WASM packaging tool
cargo install wasm-pack

# Node.js (for web client)
# Use nvm for version management
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 20
nvm use 20

# Docker (for local database and integration testing)
# https://docs.docker.com/get-docker/

# Just (command runner, optional but recommended)
cargo install just
```

### 2.2 Project Structure for DevOps

```
hexabellum/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── core/                   # Game logic (pure Rust)
│   ├── protocol/               # Shared message types
│   ├── server/                 # Axum server binary
│   └── wasm/                   # WASM bindings
├── web/                        # TypeScript client
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
├── deploy/                     # Deployment configs
│   ├── docker/
│   │   ├── Dockerfile.server
│   │   ├── Dockerfile.client
│   │   └── docker-compose.yml
│   ├── nginx/
│   │   └── nginx.conf
│   ├── k8s/                    # Kubernetes manifests (future)
│   └── terraform/              # Infrastructure as Code (future)
├── .github/
│   └── workflows/
│       ├── ci.yml              # Continuous Integration
│       ├── deploy-staging.yml  # Deploy to staging
│       └── deploy-prod.yml     # Deploy to production
├── justfile                    # Task runner commands
├── .env.example                # Environment variable template
└── README.md
```

### 2.3 Local Development Commands

Create a `justfile` for common tasks:

```just
# justfile

# Build WASM package
build-wasm:
    cd crates/wasm && wasm-pack build --target web --out-dir ../../web/src/wasm/pkg

# Start web dev server
dev-web: build-wasm
    cd web && npm run dev

# Start Rust server
dev-server:
    cargo run --bin hexabellum-server

# Start local database
dev-db:
    docker compose -f deploy/docker/docker-compose.yml up -d postgres redis

# Stop local database
dev-db-stop:
    docker compose -f deploy/docker/docker-compose.yml down

# Run all tests
test:
    cargo test --workspace
    cd web && npm test

# Build for production
build-prod: build-wasm
    cargo build --release --bin hexabellum-server
    cd web && npm run build

# Run integration tests
test-integration:
    cargo test --package hexabellum-server --test integration

# Start full local stack (server + web + db)
dev-full: dev-db dev-server dev-web
```

### 2.4 Local Docker Compose

`deploy/docker/docker-compose.yml`:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: hexabellum
      POSTGRES_USER: hexabellum
      POSTGRES_PASSWORD: ${DB_PASSWORD:-devpassword}
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U hexabellum"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
```

### 2.5 Environment Variables

`.env.example`:

```bash
# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=info

# Database
DATABASE_URL=postgres://hexabellum:devpassword@localhost:5432/hexabellum

# Redis
REDIS_URL=redis://localhost:6379

# Game Config
TURN_DURATION_SECS=30
MAX_PLAYERS_PER_MATCH=10
MATCH_TIMEOUT_SECS=300

# Security
JWT_SECRET=your-secret-key-change-in-production
CORS_ORIGINS=http://localhost:5173

# Feature Flags
ENABLE_AI_BACKFILL=true
ENABLE_RECONNECTION=true
```

---

## 3. Docker Configuration

### 3.1 Server Dockerfile

`deploy/docker/Dockerfile.server`:

```dockerfile
# Build stage
FROM rust:1.77-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy workspace
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release --bin hexabellum-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/hexabellum-server /app/hexabellum-server

# Non-root user
RUN useradd -m -u 1000 hexabellum
USER hexabellum

EXPOSE 3000

ENV RUST_LOG=info

CMD ["./hexabellum-server"]
```

### 3.2 Client Dockerfile (for static hosting)

`deploy/docker/Dockerfile.client`:

```dockerfile
# Build stage
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY web/package*.json ./
RUN npm ci

# Copy source
COPY web/ ./
COPY crates/wasm/ ./wasm-source/

# Build WASM
RUN cd wasm-source && npx wasm-pack build --target web --out-dir ../src/wasm/pkg

# Build web app
RUN npm run build

# Runtime stage (Nginx)
FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html
COPY deploy/nginx/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
```

### 3.3 Nginx Configuration

`deploy/nginx/nginx.conf`:

```nginx
server {
    listen 80;
    server_name _;

    root /usr/share/nginx/html;
    index index.html;

    # Gzip compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript application/wasm;
    gzip_min_length 1000;

    # WASM files need correct MIME type
    location ~ \.wasm$ {
        types { application/wasm wasm; }
        add_header Cache-Control "public, max-age=31536000, immutable";
    }

    # Static assets with long cache
    location /assets/ {
        add_header Cache-Control "public, max-age=31536000, immutable";
    }

    # SPA fallback
    location / {
        try_files $uri $uri/ /index.html;
    }

    # WebSocket proxy (if serving from same origin)
    location /ws/ {
        proxy_pass http://game-server:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;
    }

    # API proxy
    location /api/ {
        proxy_pass http://game-server:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

---

## 4. CI/CD Pipeline

### 4.1 Continuous Integration

`.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  # Rust checks
  rust-check:
    name: Rust Check & Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy lint
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Run tests
        run: cargo test --workspace

      - name: Build WASM
        run: |
          cargo install wasm-pack
          cd crates/wasm && wasm-pack build --target web

  # Web client checks
  web-check:
    name: Web Client Check
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: web
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: web/package-lock.json

      - name: Install dependencies
        run: npm ci

      - name: Type check
        run: npx tsc --noEmit

      - name: Lint
        run: npm run lint

      - name: Build
        run: npm run build

  # Integration tests
  integration-test:
    name: Integration Tests
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_DB: hexabellum_test
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run integration tests
        env:
          DATABASE_URL: postgres://test:test@localhost:5432/hexabellum_test
          REDIS_URL: redis://localhost:6379
        run: cargo test --package hexabellum-server --test integration
```

### 4.2 Deploy to Staging

`.github/workflows/deploy-staging.yml`:

```yaml
name: Deploy Staging

on:
  push:
    branches: [develop]

jobs:
  deploy:
    name: Deploy to Staging
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - uses: actions/checkout@v4

      - name: Build and push server image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: deploy/docker/Dockerfile.server
          push: true
          tags: |
            registry.example.com/hexabellum-server:staging
            registry.example.com/hexabellum-server:${{ github.sha }}

      - name: Build and push client image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: deploy/docker/Dockerfile.client
          push: true
          tags: |
            registry.example.com/hexabellum-client:staging
            registry.example.com/hexabellum-client:${{ github.sha }}

      - name: Deploy to staging server
        uses: appleboy/ssh-action@v1
        with:
          host: ${{ secrets.STAGING_HOST }}
          username: ${{ secrets.STAGING_USER }}
          key: ${{ secrets.STAGING_SSH_KEY }}
          script: |
            cd /opt/hexabellum
            docker compose pull
            docker compose up -d
            docker system prune -f
```

### 4.3 Deploy to Production

`.github/workflows/deploy-prod.yml`:

```yaml
name: Deploy Production

on:
  push:
    tags:
      - 'v*'

jobs:
  deploy:
    name: Deploy to Production
    runs-on: ubuntu-latest
    environment: production
    steps:
      - uses: actions/checkout@v4

      - name: Run full test suite
        run: |
          cargo test --workspace
          cd web && npm ci && npm test

      - name: Build and push server image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: deploy/docker/Dockerfile.server
          push: true
          tags: |
            registry.example.com/hexabellum-server:latest
            registry.example.com/hexabellum-server:${{ github.ref_name }}

      - name: Build and push client image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: deploy/docker/Dockerfile.client
          push: true
          tags: |
            registry.example.com/hexabellum-client:latest
            registry.example.com/hexabellum-client:${{ github.ref_name }}

      - name: Deploy to production
        uses: appleboy/ssh-action@v1
        with:
          host: ${{ secrets.PROD_HOST }}
          username: ${{ secrets.PROD_USER }}
          key: ${{ secrets.PROD_SSH_KEY }}
          script: |
            cd /opt/hexabellum
            docker compose pull
            docker compose up -d --remove-orphans
            docker system prune -f
            echo "Deployed ${{ github.ref_name }} at $(date)" >> /var/log/hexabellum-deploys.log
```

---

## 5. Hosting Strategy

### 5.1 Recommended Hosting by Phase

| Phase | Recommendation | Monthly Cost |
|-------|---------------|-------------|
| MVP / Development | Single VPS (DigitalOcean, Hetzner) | $6–12 |
| Early Access | VPS + managed PostgreSQL | $20–40 |
| Launch (100–500 players) | 2–3 VPS + load balancer + managed DB | $60–120 |
| Growth (500–5000 players) | Kubernetes cluster or cloud auto-scaling | $200–500 |

### 5.2 MVP Hosting (Recommended Start)

**Provider:** Hetzner Cloud or DigitalOcean

**Single VPS Setup:**
```
VPS: 2 vCPU, 4GB RAM, 40GB SSD
OS: Ubuntu 22.04 LTS
Cost: ~$6-12/month
```

**What runs on this VPS:**
- Game server (Rust binary via Docker)
- Nginx reverse proxy + static client hosting
- PostgreSQL (Docker container)
- Redis (Docker container)

**Production Docker Compose:**

```yaml
# /opt/hexabellum/docker-compose.yml
version: '3.8'

services:
  game-server:
    image: registry.example.com/hexabellum-server:latest
    restart: always
    environment:
      - DATABASE_URL=postgres://hexabellum:${DB_PASSWORD}@postgres:5432/hexabellum
      - REDIS_URL=redis://redis:6379
      - SERVER_HOST=0.0.0.0
      - SERVER_PORT=3000
      - RUST_LOG=info
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    networks:
      - internal

  web-client:
    image: registry.example.com/hexabellum-client:latest
    restart: always
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./certs:/etc/nginx/certs:ro
    depends_on:
      - game-server
    networks:
      - internal
      - external

  postgres:
    image: postgres:16-alpine
    restart: always
    environment:
      POSTGRES_DB: hexabellum
      POSTGRES_USER: hexabellum
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U hexabellum"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - internal

  redis:
    image: redis:7-alpine
    restart: always
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5
    networks:
      - internal

volumes:
  postgres_data:
  redis_data:

networks:
  internal:
    driver: bridge
  external:
    driver: bridge
```

### 5.3 Scaling Path

When the single VPS is no longer sufficient:

```
Stage 1: Single VPS (all-in-one)
    ↓ (when CPU > 70% sustained)
Stage 2: Separate DB from app server
    ↓ (when concurrent matches > 50)
Stage 3: Multiple game server instances + load balancer
    ↓ (when concurrent players > 500)
Stage 4: Kubernetes / auto-scaling group
```

---

## 6. Database Schema (Initial)

### 6.1 Core Tables

```sql
-- Players
CREATE TABLE players (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(32) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE,
    password_hash VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    settings JSONB DEFAULT '{}'
);

-- Matches
CREATE TABLE matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_code VARCHAR(16) UNIQUE NOT NULL,
    status VARCHAR(20) DEFAULT 'waiting', -- waiting, active, completed
    config JSONB NOT NULL,
    winner_team SMALLINT,
    rounds_played INTEGER DEFAULT 0,
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Match Participants
CREATE TABLE match_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_id UUID REFERENCES matches(id) ON DELETE CASCADE,
    player_id UUID REFERENCES players(id),
    team SMALLINT NOT NULL,
    hero_def_id VARCHAR(32) NOT NULL,
    is_ai BOOLEAN DEFAULT false,
    final_level INTEGER,
    final_gold INTEGER,
    kills INTEGER DEFAULT 0,
    deaths INTEGER DEFAULT 0,
    damage_dealt INTEGER DEFAULT 0,
    UNIQUE(match_id, player_id)
);

-- Match Events (for replays)
CREATE TABLE match_events (
    id BIGSERIAL PRIMARY KEY,
    match_id UUID REFERENCES matches(id) ON DELETE CASCADE,
    round INTEGER NOT NULL,
    event_type VARCHAR(32) NOT NULL,
    event_data JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_matches_status ON matches(status);
CREATE INDEX idx_matches_created ON matches(created_at DESC);
CREATE INDEX idx_participants_match ON match_participants(match_id);
CREATE INDEX idx_participants_player ON match_participants(player_id);
CREATE INDEX idx_events_match_round ON match_events(match_id, round);
```

### 6.2 Database Migrations

Use `sqlx` migrations in Rust:

```bash
# Install sqlx-cli
cargo install sqlx-cli

# Create migration
sqlx migrate add create_players_table
sqlx migrate add create_matches_table
sqlx migrate add create_match_participants_table
sqlx migrate add create_match_events_table

# Run migrations
sqlx migrate run
```

---

## 7. Security

### 7.1 TLS / HTTPS

**Requirement:** All production traffic must be encrypted.

**Options:**

| Method | Cost | Complexity |
|--------|------|-----------|
| Let's Encrypt + Caddy | Free | Low (auto-renewal) |
| Let's Encrypt + Nginx + certbot | Free | Medium |
| Cloud provider managed cert | $0–10/mo | Low |
| Cloudflare proxy | Free tier | Low |

**Recommended:** Caddy reverse proxy (auto-HTTPS):

```
# /etc/caddy/Caddyfile
hexabellum.example.com {
    reverse_proxy /ws/* localhost:3000
    reverse_proxy /api/* localhost:3000
    reverse_proxy /* localhost:8080
}
```

### 7.2 WebSocket Security

- Always use `wss://` in production (WebSocket over TLS)
- Validate origin header on WebSocket upgrade
- Rate limit connection attempts (max 10 per minute per IP)
- Implement heartbeat/ping to detect dead connections
- Close idle connections after 5 minutes

### 7.3 Input Validation

- All client messages validated server-side before processing
- Maximum message size: 64 KB
- Reject malformed JSON immediately
- Sanitize player names (no HTML, max 32 chars)

### 7.4 Rate Limiting

| Endpoint | Limit |
|----------|-------|
| WebSocket connect | 10/min per IP |
| API match create | 5/min per IP |
| Order submission | 1 per round per player (enforced by game logic) |
| Chat (future) | 10/min per player |

### 7.5 Secrets Management

**Never commit secrets to git.**

Use environment variables or secret managers:

```bash
# Production secrets stored in:
# - /opt/hexabellum/.env (on server, chmod 600)
# - GitHub Actions secrets (for CI/CD)
# - Cloud secret manager (for production)
```

---

## 8. Monitoring & Logging

### 8.1 Application Logging

Use `tracing` crate with structured JSON output:

```rust
// In server main.rs
tracing_subscriber::fmt()
    .json()
    .with_env_filter(EnvFilter::from_default_env())
    .init();
```

**Log levels:**
- `error` — Match crashes, database failures
- `warn` — Player disconnects, invalid orders
- `info` — Match created, round resolved, player joined
- `debug` — Order details, pathfinding calculations
- `trace` — Every state mutation (development only)

### 8.2 Health Check Endpoint

Add to server:

```rust
// GET /health
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: get_uptime(),
        active_matches: get_active_match_count(),
        connected_players: get_connected_player_count(),
    })
}
```

### 8.3 Monitoring Stack (MVP)

For MVP, keep it simple:

| Tool | Purpose | Cost |
|------|---------|------|
| Server logs to file | Debugging | Free |
| `docker logs` | Container health | Free |
| UptimeRobot / BetterStack | Uptime monitoring | Free tier |
| Grafana Cloud (optional) | Metrics dashboard | Free tier |

### 8.4 Key Metrics to Track

| Metric | Alert Threshold |
|--------|----------------|
| Server CPU usage | > 80% for 5 min |
| Server memory usage | > 85% |
| Active WebSocket connections | > 90% of capacity |
| Match resolution time | > 2 seconds |
| Database connection pool usage | > 80% |
| Error rate (5xx) | > 1% of requests |
| WebSocket disconnect rate | > 5% per hour |

### 8.5 Structured Log Example

```json
{
  "timestamp": "2024-01-15T10:30:45Z",
  "level": "INFO",
  "target": "hexabellum_server::match_actor",
  "message": "Round resolved",
  "match_id": "match_abc123",
  "round": 7,
  "resolution_time_ms": 45,
  "active_units": 18,
  "events_emitted": 12
}
```

---

## 9. Backup & Recovery

### 9.1 Database Backups

**Automated daily backups:**

```bash
# /opt/hexabellum/scripts/backup.sh
#!/bin/bash
BACKUP_DIR="/opt/hexabellum/backups"
DATE=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=30

mkdir -p $BACKUP_DIR

# PostgreSQL dump
docker exec hexabellum-postgres pg_dump -U hexabellum hexabellum | gzip > "$BACKUP_DIR/db_$DATE.sql.gz"

# Remove old backups
find $BACKUP_DIR -name "db_*.sql.gz" -mtime +$RETENTION_DAYS -delete

echo "Backup completed: db_$DATE.sql.gz"
```

**Cron job:**
```bash
# Run daily at 3 AM
0 3 * * * /opt/hexabellum/scripts/backup.sh >> /var/log/hexabellum-backup.log 2>&1
```

### 9.2 Disaster Recovery

| Scenario | Recovery Time | Recovery Point |
|----------|--------------|----------------|
| Server crash | 5 min (Docker restart) | No data loss (state in memory) |
| Database corruption | 30 min (restore backup) | Up to 24 hours |
| Full server loss | 1–2 hours (reprovision) | Up to 24 hours |
| Region outage | 4+ hours (manual failover) | Up to 24 hours |

### 9.3 Match State Recovery

Active matches exist in server memory. If the server crashes:
- Active matches are lost
- Players are disconnected
- Completed match data in database is preserved

Mitigation (future):
- Periodic match state snapshots to Redis
- Match resumption from last snapshot

---

## 10. Deployment Checklist

### 10.1 Pre-Deployment

- [ ] All CI tests pass
- [ ] Database migrations tested on staging
- [ ] WASM bundle size < 2 MB
- [ ] Client bundle size < 500 KB
- [ ] No `console.log` statements in production build
- [ ] Environment variables configured
- [ ] Database backup taken
- [ ] Rollback plan documented

### 10.2 Deployment Steps

```bash
# 1. SSH into server
ssh user@hexabellum.example.com

# 2. Pull latest images
cd /opt/hexabellum
docker compose pull

# 3. Run database migrations (if any)
docker compose run --rm game-server ./hexabellum-server migrate

# 4. Restart services
docker compose up -d

# 5. Verify health
curl https://hexabellum.example.com/health

# 6. Check logs
docker compose logs -f --tail=50 game-server

# 7. Test WebSocket connection
wscat -c wss://hexabellum.example.com/ws/match/test
```

### 10.3 Post-Deployment

- [ ] Health check returns 200
- [ ] Can create a match via API
- [ ] Can connect via WebSocket
- [ ] Client loads in browser
- [ ] No error spikes in logs
- [ ] Response time < 200ms
- [ ] Monitor for 30 minutes after deploy

### 10.4 Rollback Procedure

```bash
# If deployment fails:
cd /opt/hexabellum

# Rollback to previous image
docker compose down
export IMAGE_TAG=previous-version
docker compose up -d

# If database migration needs rollback:
docker compose run --rm game-server ./hexabellum-server migrate rollback
```

---

## 11. Performance Targets

### 11.1 Server Performance

| Metric | Target |
|--------|--------|
| Match creation time | < 50 ms |
| Round resolution time (10 heroes, 20 minions) | < 100 ms |
| WebSocket message latency (server processing) | < 10 ms |
| Concurrent matches per server instance | 50–100 |
| Memory per active match | < 10 MB |
| CPU per active match (during resolution) | < 5% |

### 11.2 Client Performance

| Metric | Target |
|--------|--------|
| Initial page load | < 3 seconds |
| WASM module load | < 2 seconds |
| Time to interactive | < 5 seconds |
| Frame rate during resolution | 60 fps |
| Input latency (click to visual feedback) | < 100 ms |
| Memory usage | < 200 MB |

### 11.3 Network Performance

| Metric | Target |
|--------|--------|
| WebSocket round-trip time | < 100 ms (same region) |
| Snapshot size (full state) | < 50 KB |
| Event batch size (per round) | < 20 KB |
| Reconnection time | < 2 seconds |

---

## 12. Cost Estimation

### 12.1 MVP / Development Phase

| Item | Provider | Monthly Cost |
|------|----------|-------------|
| VPS (2 vCPU, 4GB) | Hetzner / DigitalOcean | $6–12 |
| Domain name | Namecheap / Cloudflare | $10–15/year |
| SSL certificate | Let's Encrypt | Free |
| Total | | **$6–12/month** |

### 12.2 Early Access (100–500 players)

| Item | Provider | Monthly Cost |
|------|----------|-------------|
| VPS (4 vCPU, 8GB) | Hetzner / DigitalOcean | $12–24 |
| Managed PostgreSQL | DigitalOcean / Supabase | $15–25 |
| CDN (Cloudflare) | Cloudflare | Free tier |
| Monitoring | BetterStack / UptimeRobot | Free tier |
| Total | | **$27–49/month** |

### 12.3 Launch (500–5000 players)

| Item | Provider | Monthly Cost |
|------|----------|-------------|
| 2–3 VPS instances | Hetzner / AWS | $40–80 |
| Load balancer | Cloud provider | $10–20 |
| Managed PostgreSQL | Cloud provider | $25–50 |
| Redis | Cloud provider | $10–20 |
| CDN | Cloudflare Pro | $20 |
| Monitoring | Grafana Cloud / Datadog | $0–50 |
| Total | | **$105–240/month** |

---

## 13. Future Infrastructure Considerations

### 13.1 Matchmaking Service (Phase 8+)

When adding ranked matchmaking:
- Separate matchmaking service (can be same Rust binary, different endpoint)
- Redis sorted sets for player queues
- ELO/Glicko rating calculation
- Queue timeout handling

### 13.2 Replay Service (Phase 8+)

When adding replays:
- Store match events in database (already designed)
- Replay playback endpoint: `GET /api/matches/:id/replay`
- Client-side replay player (reuse WASM engine)

### 13.3 Multi-Region Deployment (Phase 10+)

When player base grows globally:
- Regional game servers (US, EU, Asia)
- Global matchmaking service
- Cross-region database replication
- CDN for static assets

### 13.4 Kubernetes Migration (When Needed)

Migrate to Kubernetes when:
- Running > 5 server instances
- Need auto-scaling based on player demand
- Need zero-downtime deployments
- Need service discovery between components

---

## 14. Quick Start: Deploy in 30 Minutes

For a solo developer wanting to get online fast:

```bash
# 1. Provision a VPS (Hetzner, DigitalOcean, etc.)
#    Ubuntu 22.04, 2 vCPU, 4GB RAM

# 2. SSH in and install Docker
ssh root@your-server-ip
curl -fsSL https://get.docker.com | sh
apt install docker-compose-plugin

# 3. Clone and configure
git clone https://github.com/yourname/hexabellum.git /opt/hexabellum
cd /opt/hexabellum
cp .env.example .env
nano .env  # Set secure passwords

# 4. Build and start
docker compose -f deploy/docker/docker-compose.yml up -d --build

# 5. Verify
curl http://localhost:3000/health

# 6. Set up domain + HTTPS (install Caddy)
apt install caddy
nano /etc/caddy/Caddyfile
# Add your domain config
systemctl restart caddy

# Done! Game is live at https://your-domain.com
```

---

*End of DevOps & Deployment Guide*

---

*HEXABELLUM — Six sides of war. One victor.*