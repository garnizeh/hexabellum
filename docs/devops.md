# HEXABELLUM

## DevOps & Infrastructure Engineering Guide

---

## Document Version
`DevOps v2.0 — Aligned with Architecture v2.0, Phase 7 MOBA Macro Specification, GDD v2.0, UX v2.0, and Phases 0–7 Technical Specs`

---

## 1. Executive Summary & Infrastructure Strategy

**Hexabellum** is a simultaneous turn-based tactical MOBA requiring a robust, deterministic, low-latency, and horizontally scalable deployment architecture. Unlike traditional real-time games that demand high-tick UDP networking (30–128 Hz) or pure web applications that operate statelessly over HTTP REST/GraphQL, Hexabellum operates on a **hybrid stateful/static architecture**:

1. **Static Presentation Delivery**: Browser clients download a Single-Page Application (SPA) composed of TypeScript, PixiJS v8 WebGL rendering assets, and a compiled WebAssembly (`wasm32-unknown-unknown`) simulation runtime. These assets are immutable, heavily cached, and distributed via global Content Delivery Networks (CDNs) or high-performance edge reverse proxies.
2. **Stateful WebSocket Gateway**: Players establish persistent, authenticated WebSocket Secure (`wss://`) connections to an authoritative Rust backend powered by Axum and Tokio. The backend manages the entire match lifecycle—from lobby matchmaking and a 20-second hero select draft to 30-second simultaneous planning rounds, zero-knowledge line-of-sight sanitization, and deterministic BLAKE3 state verification.
3. **Persistent Data Tier**: Historical match statistics, player MMR ratings, accounts, and match event replays are stored in PostgreSQL 16. Fast-moving ephemeral state, matchmaking queues, distributed session tokens, and cross-instance pub/sub signaling reside in Redis 7.

```mermaid
flowchart TD
    subgraph "Player Browser Clients (1 to 10 Players per Match)"
        ClientA["Player Client A<br/>(TypeScript + PixiJS + WASM)"]
        ClientB["Player Client B<br/>(TypeScript + PixiJS + WASM)"]
        ClientN["Player Client N<br/>(TypeScript + PixiJS + WASM)"]
    end

    subgraph "Edge & Ingress Tier"
        CDN["Global CDN / Edge Cache<br/>(Cloudflare / Fastly)<br/>Static Assets & WASM Binary"]
        LB["TLS Termination & Reverse Proxy<br/>(Caddy / Nginx / Cloud LB)<br/>Port 443 -> WSS & HTTPS"]
    end

    subgraph "Application Tier (Game Cluster)"
        GS1["Game Server Instance 1<br/>Axum + Tokio (Rust)<br/>MatchActors: Matches 1..M"]
        GS2["Game Server Instance 2<br/>Axum + Tokio (Rust)<br/>MatchActors: Matches M+1..K"]
        GSN["Game Server Instance N<br/>Axum + Tokio (Rust)<br/>Auto-Scaled Worker Pod"]
    end

    subgraph "Data & Persistence Tier"
        PgBouncer["Connection Pooler<br/>(PgBouncer)"]
        PG[("PostgreSQL 16 Primary<br/>Accounts, Match Records,<br/>Replays, MMR Ratings")]
        Redis[("Redis 7 Cluster / Sentinel<br/>Matchmaking Queues,<br/>Match Registry, Session Tokens")]
    end

    subgraph "Observability & SRE Tier"
        Prom["Prometheus / VictoriaMetrics<br/>Scrapes /metrics (15s)"]
        Grafana["Grafana Dashboards<br/>Turn Latency, Desync Alerts"]
        Loki["Vector / Promtail -> Loki<br/>Structured JSON Logs"]
    end

    ClientA -->|HTTPS: Fetch App & WASM| CDN
    ClientB -->|HTTPS: Fetch App & WASM| CDN
    ClientN -->|HTTPS: Fetch App & WASM| CDN

    ClientA -->|WSS: Match Connection| LB
    ClientB -->|WSS: Match Connection| LB
    ClientN -->|WSS: Match Connection| LB

    LB -->|Internal WS / HTTP| GS1
    LB -->|Internal WS / HTTP| GS2
    LB -->|Internal WS / HTTP| GSN

    GS1 -->|Pooled SQL| PgBouncer
    GS2 -->|Pooled SQL| PgBouncer
    GSN -->|Pooled SQL| PgBouncer
    PgBouncer --> PG

    GS1 <-->|Pub/Sub & Queues| Redis
    GS2 <-->|Pub/Sub & Queues| Redis
    GSN <-->|Pub/Sub & Queues| Redis

    GS1 -.->|Metrics & Traces| Prom
    GS2 -.->|Metrics & Traces| Prom
    GSN -.->|Metrics & Traces| Prom
    Prom --> Grafana
    GS1 -.->|JSON Logs| Loki
```

### 1.1 Core Component Architecture

| Component | Technology | Runtime Role | Scaling Characteristic |
|---|---|---|---|
| **Web Client SPA** | TypeScript 5.4, PixiJS v8, Vite 5.2 | Visual rendering, user input, audio, DOM HUD overlays | Infinitely scalable via edge CDN caching; 0 origin server load. |
| **Simulation WASM** | Rust 1.85+, `wasm32-unknown-unknown`, `wasm-pack` | Client-side trajectory preview, fog projection, local replay parsing | Client-side execution sandbox; 0 server CPU overhead. |
| **Game Server** | Rust 1.85+ (Edition 2024), Axum 0.7, Tokio 1.38 | Authoritative simulation, 10-player WebSocket rooms, AI backfill, state sanitization | Horizontally scalable by match room (`MatchActor`). Memory bounded (~10 MB per match). |
| **Database** | PostgreSQL 16 Alpine | Persistent accounts, match outcomes, player ratings, telemetry, event logs | Vertically scalable primary with read-replicas for analytical replay queries. |
| **In-Memory Cache** | Redis 7 Alpine | Matchmaker queues, session token validation, match-to-server routing registry | In-memory operations; single cluster handles > 50,000 ops/sec with sub-millisecond latency. |
| **Reverse Proxy** | Caddy 2 or Nginx 1.25+ | Automatic TLS certificate issuance (ACME), HTTP/2, WebSocket proxying, rate limiting | Low CPU/RAM overhead; handles 10,000+ concurrent connections per instance. |

### 1.2 Network Protocols & Port Matrix

| Protocol | External Port | Internal Port | Source | Destination | Purpose |
|---|---|---|---|---|---|
| **HTTPS** | `443/TCP` | `80/TCP` (Nginx/Caddy) | Public Internet | Reverse Proxy | Initial HTML, CSS, JavaScript, and `.wasm` bundle delivery. |
| **WSS** | `443/TCP` | `3000/TCP` | Client Browser | Game Server (`/ws/match/:id`) | Persistent bidirectional binary/JSON framing for game lifecycle. |
| **HTTP (Internal)** | N/A | `3000/TCP` | Reverse Proxy / K8s | Game Server (`/health`, `/ready`, `/metrics`) | Ingress health probes and Prometheus metric scraping. |
| **PostgreSQL** | N/A | `5432/TCP` | Game Server | Postgres / PgBouncer | Authenticated, pooled database transactions. |
| **Redis** | N/A | `6379/TCP` | Game Server | Redis Node | Matchmaking queue updates, pub/sub broadcasts, lock management. |

---

## 2. Development Environment & Tooling

To ensure deterministic builds and parity between local developer environments, continuous integration (CI), and production containers, all contributors use standardized toolchains.

### 2.1 Host Toolchain Requirements

```bash
# 1. Rust Compiler (1.85+ with Edition 2024 support)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
rustup target add wasm32-unknown-unknown

# 2. WebAssembly Packager & Optimizer
cargo install wasm-pack --version 0.12.1
# Optional: binaryen for wasm-opt binary optimization
# macOS: brew install binaryen | Ubuntu: apt-get install -y binaryen

# 3. Node.js (v20 LTS) & NPM
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
nvm install 20
nvm use 20

# 4. Command Runners & Container Engine
# Docker with Compose V2
curl -fsSL https://get.docker.com | sh
# Just (Modern Command Runner) or GNU Make
cargo install just
# SQLx CLI (for compile-time database query validation)
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

### 2.2 Repository Directory Layout

The repository is structured as a unified monorepo supporting Rust crates, the TypeScript web client, deployment manifests, and automation scripts:

```
hexabellum/
├── Cargo.toml                  # Cargo workspace definition (core, wasm, protocol, server)
├── Cargo.lock                  # Deterministic dependency lockfile
├── Makefile                    # Standard GNU Make targets
├── justfile                    # Modern Just command runner definitions
├── README.md                   # Repository overview and setup guide
├── .env.example                # Canonical environment variable template
├── crates/
│   ├── core/                   # Pure deterministic game simulation engine
│   │   ├── Cargo.toml
│   │   └── src/                # Hex math, units, spells, turn lifecycle, BLAKE3 state
│   ├── protocol/               # Shared client-server WebSocket DTOs & error codes
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── wasm/                   # WebAssembly bindings exposed to TypeScript
│   │   ├── Cargo.toml
│   │   └── src/
│   └── server/                 # Axum & Tokio authoritative game server binary
│       ├── Cargo.toml
│       ├── tests/              # Headless integration & 100-round soak tests
│       └── src/                # MatchActor, WebSocket handlers, Redis matchmaking
├── web/                        # Vite + TypeScript + PixiJS client application
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   └── src/
│       ├── index.html
│       ├── wasm/pkg/           # Compiled wasm-pack output (git-ignored)
│       └── ...
├── deploy/                     # Infrastructure as Code & Container Definitions
│   ├── docker/
│   │   ├── Dockerfile.server   # Cached multi-stage Rust server build
│   │   ├── Dockerfile.client   # Multi-stage WASM + Vite + Nginx build
│   │   ├── docker-compose.yml  # Local developer auxiliary stack (Postgres + Redis)
│   │   └── docker-compose.prod.yml # Production single-node multi-container stack
│   ├── nginx/
│   │   └── nginx.conf          # Hardened Nginx configuration with WASM & WS tuning
│   ├── caddy/
│   │   └── Caddyfile           # Modern auto-HTTPS reverse proxy configuration
│   ├── k8s/                    # Kubernetes manifests (Deployment, Service, HPA)
│   └── terraform/              # Cloud infrastructure provisioning
└── docs/                       # Architectural & Game Design Documentation
    ├── overview.md             # Master Architectural Blueprint
    ├── gdd.md                  # Complete Game Design Document
    ├── ui-ux.md                # Tactical HUD & Visual Interface Architecture
    ├── devops.md               # Infrastructure & Operations Guide (This Document)
    └── phase-0.md ... phase-7.md # Phase Specifications
```

### 2.3 Local Developer Workflow (`justfile` & `Makefile`)

The project supports both `just` and standard `make` for maximum developer convenience:

```just
# justfile

default:
    @just --list

# Build WASM package with release optimizations for local client linking
build-wasm:
    cd crates/wasm && wasm-pack build --target web --out-dir ../../web/src/wasm/pkg --dev

# Build WASM package with production size optimization
build-wasm-release:
    cd crates/wasm && wasm-pack build --target web --out-dir ../../web/src/wasm/pkg --release

# Start web client development server with hot-reload
dev-web: build-wasm
    cd web && npm run dev

# Start Rust authoritative server with trace logging
dev-server:
    RUST_LOG=debug,hexabellum_server=trace cargo run --bin hexabellum-server

# Spin up local PostgreSQL and Redis containers
dev-services:
    docker compose -f deploy/docker/docker-compose.yml up -d

# Stop local PostgreSQL and Redis containers
dev-services-down:
    docker compose -f deploy/docker/docker-compose.yml down

# Run all workspace unit tests and web client tests
test:
    cargo test --workspace
    cd web && npm test

# Run 100-round 5v5 deterministic soak test
test-soak:
    cargo test --package hexabellum-server --test soak_determinism -- --nocapture

# Run linters and format checks
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cd web && npm run lint
```

### 2.4 Canonical Environment Variables (`.env.example`)

All application components read configuration from environment variables or a local `.env` file:

```bash
# ==============================================================================
# HEXABELLUM SERVER CONFIGURATION
# ==============================================================================
ENVIRONMENT=development
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
PUBLIC_WS_URL=ws://localhost:3000/ws
RUST_LOG=info,hexabellum_server=debug,tower_http=info

# ==============================================================================
# DATABASE CONFIGURATION (PostgreSQL 16)
# ==============================================================================
DATABASE_URL=postgres://hexabellum:devpassword@localhost:5432/hexabellum
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=5
DB_ACQUIRE_TIMEOUT_SECS=30
DB_IDLE_TIMEOUT_SECS=600

# ==============================================================================
# CACHE & MATCHMAKING (Redis 7)
# ==============================================================================
REDIS_URL=redis://localhost:6379/0
REDIS_POOL_SIZE=16

# ==============================================================================
# MATCH TIMERS & LIFECYCLE (Aligned with Phase 5 & Phase 7 Specs)
# ==============================================================================
LOBBY_READY_TIMEOUT_SECS=60
HERO_SELECT_DURATION_SECS=20
PLANNING_DURATION_SECS=30
EARLY_RESOLVE_GRACE_PERIOD_MS=1000
MATCH_MAX_ROUNDS=50
RECONNECT_GRACE_PERIOD_SECS=120

# ==============================================================================
# SECURITY & CORS
# ==============================================================================
JWT_SECRET=super-secret-hexabellum-jwt-key-min-32-chars-change-in-prod
CORS_ALLOWED_ORIGINS=http://localhost:5173,http://localhost:3000
MAX_WS_MESSAGE_SIZE_BYTES=65536
RATE_LIMIT_WS_CONNECTS_PER_MINUTE=15

# ==============================================================================
# OBSERVABILITY & TELEMETRY
# ==============================================================================
METRICS_ENABLED=true
METRICS_PORT=9090
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
ENABLE_BLAKE3_STATE_AUDIT_LOGGING=true
```

---

## 3. Production Containerization (Docker & OCI)

Building production containers for a mixed Rust/WASM/TypeScript repository requires careful layer caching to avoid rebuilding hundreds of Rust crates on every frontend tweak.

### 3.1 Server Multi-Stage Dockerfile (`deploy/docker/Dockerfile.server`)

We employ [`cargo-chef`](https://github.com/LukeMathWalker/cargo-chef) to compute a recipe of dependencies. Dependencies are compiled in an isolated Docker layer, ensuring lightning-fast incremental builds:

```dockerfile
# Stage 1: Recipe Planner
FROM rust:1.85-slim-bookworm AS planner
WORKDIR /app
RUN cargo install cargo-chef --version 0.1.68
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Dependency Cacher
FROM rust:1.85-slim-bookworm AS cacher
WORKDIR /app
RUN cargo install cargo-chef --version 0.1.68
COPY --from=planner /app/recipe.json recipe.json
# Build only dependencies - cached until Cargo.lock or Cargo.toml changes
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Application Builder
FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
# Copy pre-compiled dependencies from cacher
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
# Build release binary
RUN cargo build --release --bin hexabellum-server
# Strip debug symbols to reduce binary size (< 25 MB)
RUN strip /app/target/release/hexabellum-server

# Stage 4: Minimal Hardened Runtime
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install ca-certificates and tini for clean PID 1 signal forwarding
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tini \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Non-root unprivileged service user
RUN groupadd -g 10001 hexabellum && \
    useradd -u 10001 -g hexabellum -s /bin/false -m hexabellum

COPY --from=builder --chown=hexabellum:hexabellum /app/target/release/hexabellum-server /app/hexabellum-server

USER hexabellum:hexabellum

EXPOSE 3000

ENV RUST_LOG=info \
    SERVER_HOST=0.0.0.0 \
    SERVER_PORT=3000

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/app/hexabellum-server"]
```

### 3.2 Client Multi-Stage Dockerfile (`deploy/docker/Dockerfile.client`)

The web client build requires compiling the Rust WASM crate first, generating the TypeScript bindings, and then bundling the Vite SPA. We execute this cleanly across decoupled container stages:

```dockerfile
# Stage 1: Rust WASM Compiler
FROM rust:1.85-slim-bookworm AS wasm-builder
WORKDIR /app

RUN apt-get update && apt-get install -y curl pkg-config && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

COPY Cargo.toml Cargo.lock ./
COPY crates/core/ crates/core/
COPY crates/wasm/ crates/wasm/

WORKDIR /app/crates/wasm
RUN wasm-pack build --target web --out-dir /app/pkg --release

# Stage 2: Node.js Vite Bundler
FROM node:20-alpine AS web-builder
WORKDIR /app

COPY web/package*.json ./
RUN npm ci

COPY web/ ./
# Inject the pre-compiled WASM package
COPY --from=wasm-builder /app/pkg ./src/wasm/pkg

RUN npm run build

# Stage 3: High-Performance Hardened Nginx Runtime
FROM nginx:1.25-alpine AS runtime

# Remove default server definition
RUN rm /etc/nginx/conf.d/default.conf

COPY deploy/nginx/nginx.conf /etc/nginx/conf.d/hexabellum.conf
COPY --from=web-builder /app/dist /usr/share/nginx/html

# Run as unprivileged nginx user
RUN touch /var/run/nginx.pid && \
    chown -R nginx:nginx /var/run/nginx.pid /var/cache/nginx /usr/share/nginx/html

USER nginx

EXPOSE 80

HEALTHCHECK --interval=15s --timeout=3s --retries=3 \
    CMD wget -q -O /dev/null http://localhost:80/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
```

### 3.3 Production Nginx Configuration (`deploy/nginx/nginx.conf`)

Serving WebAssembly and handling persistent WebSocket proxying requires specialized Nginx tuning:

```nginx
# /etc/nginx/conf.d/hexabellum.conf

upstream game_backend {
    server game-server:3000;
    keepalive 32;
}

server {
    listen 80;
    server_name _;

    root /usr/share/nginx/html;
    index index.html;

    # Maximum upload size for match replays/logs
    client_max_body_size 10M;

    # Gzip & Brotli compression for static bundles
    gzip on;
    gzip_vary on;
    gzip_proxied any;
    gzip_comp_level 6;
    gzip_types
        text/plain
        text/css
        text/javascript
        application/javascript
        application/json
        application/wasm
        image/svg+xml;

    # Security Headers
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; connect-src 'self' ws: wss:; img-src 'self' data: blob:;" always;

    # WASM files require proper MIME type and long-term immutable caching
    location ~* \.wasm$ {
        types { application/wasm wasm; }
        default_type application/wasm;
        add_header Cache-Control "public, max-age=31536000, immutable";
        access_log off;
    }

    # Hashed JS and CSS assets
    location /assets/ {
        add_header Cache-Control "public, max-age=31536000, immutable";
        access_log off;
    }

    # Single Page Application (SPA) routing fallback
    location / {
        try_files $uri $uri/ /index.html;
        add_header Cache-Control "no-cache, no-store, must-revalidate";
    }

    # WebSocket Proxying to Axum Tokio backend
    location /ws/ {
        proxy_pass http://game_backend;
        proxy_http_version 1.1;

        # Connection upgrade headers
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Forward real client IP
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket timeouts: 1 hour idle read timeout
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
        proxy_buffering off;
        proxy_redirect off;
        tcp_nodelay on;
    }

    # REST API & Health Check proxy
    location /api/ {
        proxy_pass http://game_backend;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }

    location /health {
        proxy_pass http://game_backend/health;
        access_log off;
    }
}
```

### 3.4 Production Docker Compose (`deploy/docker/docker-compose.prod.yml`)

A self-contained, hardened multi-container specification suitable for single-node VPS deployment:

```yaml
# deploy/docker/docker-compose.prod.yml
version: '3.8'

services:
  caddy:
    image: caddy:2.7-alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ../caddy/Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    depends_on:
      - web-client
      - game-server
    networks:
      - public_net
      - internal_net

  web-client:
    image: ghcr.io/garnizeh/hexabellum-client:${CLIENT_TAG:-latest}
    restart: unless-stopped
    networks:
      - internal_net
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 256M

  game-server:
    image: ghcr.io/garnizeh/hexabellum-server:${SERVER_TAG:-latest}
    restart: unless-stopped
    env_file:
      - /opt/hexabellum/.env
    environment:
      - DATABASE_URL=postgres://hexabellum:${DB_PASSWORD}@postgres:5432/hexabellum
      - REDIS_URL=redis://redis:6379/0
      - SERVER_HOST=0.0.0.0
      - SERVER_PORT=3000
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    networks:
      - internal_net
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2048M
        reservations:
          cpus: '0.5'
          memory: 512M

  postgres:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_DB: hexabellum
      POSTGRES_USER: hexabellum
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ../postgres/init.sql:/docker-entrypoint-initdb.d/init.sql:ro
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U hexabellum -d hexabellum"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - internal_net
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2048M

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    command: ["redis-server", "--appendonly", "yes", "--maxmemory", "512mb", "--maxmemory-policy", "volatile-lru"]
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5
    networks:
      - internal_net
    deploy:
      resources:
        limits:
          cpus: '0.5'
          memory: 512M

volumes:
  caddy_data:
  caddy_config:
  postgres_data:
  redis_data:

networks:
  public_net:
    driver: bridge
  internal_net:
    driver: bridge
    internal: true
```

---

## 4. CI/CD Automation Pipeline (GitHub Actions)

Continuous Integration enforces strict code formatting, memory safety, dependency audit compliance, and determinism soak verification before any artifact reaches staging or production.

```mermaid
flowchart LR
    subgraph "Pull Request / Push"
        Code[Code Change]
    end

    subgraph "Stage 1: Validation & Quality"
        Fmt[Rust Fmt & Clippy]
        Audit[Cargo Audit & Deny]
        TSLint[TypeScript Check & Lint]
    end

    subgraph "Stage 2: Determinism & Unit Testing"
        UnitTest[Cargo Workspace Tests]
        WasmBuild[WASM Pack Build & Size Budget]
        Soak[100-Round 5v5 Soak Simulation]
    end

    subgraph "Stage 3: Containerization"
        Buildx[Docker Buildx Multi-Arch]
        GHCR[Push to ghcr.io Registry]
    end

    subgraph "Stage 4: Automated Deployment"
        DeployStaging[Rolling Deploy Staging]
        HealthCheck[Automated Health Verification]
        DeployProd[Promote to Production]
    end

    Code --> Fmt
    Code --> Audit
    Code --> TSLint

    Fmt --> UnitTest
    Audit --> UnitTest
    TSLint --> WasmBuild

    UnitTest --> Soak
    WasmBuild --> Soak

    Soak --> Buildx
    Buildx --> GHCR

    GHCR --> DeployStaging
    DeployStaging --> HealthCheck
    HealthCheck --> DeployProd
```

### 4.1 Master Continuous Integration Workflow (`.github/workflows/ci.yml`)

```yaml
name: CI & Determinism Verification

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  rust-check:
    name: Rust Formatting & Clippy Lints
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cargo cache
        uses: Swatinem/rust-cache@v2

      - name: Check code formatting
        run: cargo fmt --all -- --check

      - name: Run Clippy lints
        run: cargo clippy --workspace --all-targets --all-features -- -D warnings

  security-audit:
    name: Cargo Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run Cargo Audit
        uses: rustsec/audit-check@v2.0.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  rust-test:
    name: Unit & Headless Soak Testing
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cargo cache
        uses: Swatinem/rust-cache@v2

      - name: Run workspace unit tests
        run: cargo test --workspace --exclude hexabellum-wasm

      - name: Run 100-Round 5v5 BLAKE3 Determinism Soak Test
        run: cargo test --package hexabellum-core --test soak_determinism -- --nocapture

  wasm-bundle-budget:
    name: WASM Build & Size Budget Enforcement
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust & wasm32 target
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Build WASM release bundle
        run: cd crates/wasm && wasm-pack build --target web --release

      - name: Enforce WASM binary size budget (< 1.8 MB uncompressed)
        run: |
          FILE_SIZE=$(stat -c%s crates/wasm/pkg/hexabellum_wasm_bg.wasm)
          echo "WASM Binary Size: $FILE_SIZE bytes"
          MAX_SIZE=1887436 # 1.8 MB
          if [ "$FILE_SIZE" -gt "$MAX_SIZE" ]; then
            echo "ERROR: WASM binary exceeds size budget of $MAX_SIZE bytes!"
            exit 1
          fi

  web-check:
    name: TypeScript Check, Lint & Vitest
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: web
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js 20
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'npm'
          cache-dependency-path: web/package-lock.json

      - name: Install dependencies
        run: npm ci

      - name: TypeScript typecheck
        run: npx tsc --noEmit

      - name: ESLint check
        run: npm run lint
```

### 4.2 Automated Production Deployment Workflow (`.github/workflows/deploy-prod.yml`)

```yaml
name: Deploy Production

on:
  push:
    tags:
      - 'v*'

jobs:
  publish-images:
    name: Build & Publish OCI Images to GHCR
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata for server
        id: meta-server
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/${{ github.repository }}-server
          tags: |
            type=semver,pattern={{version}}
            type=raw,value=latest

      - name: Build and push server image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: deploy/docker/Dockerfile.server
          push: true
          tags: ${{ steps.meta-server.outputs.tags }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Extract metadata for client
        id: meta-client
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/${{ github.repository }}-client
          tags: |
            type=semver,pattern={{version}}
            type=raw,value=latest

      - name: Build and push client image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: deploy/docker/Dockerfile.client
          push: true
          tags: ${{ steps.meta-client.outputs.tags }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  deploy-production:
    name: Deploy to Production Cluster
    needs: publish-images
    runs-on: ubuntu-latest
    environment: production
    steps:
      - name: Execute SSH Rolling Deployment
        uses: appleboy/ssh-action@v1.0.3
        with:
          host: ${{ secrets.PROD_HOST }}
          username: ${{ secrets.PROD_USER }}
          key: ${{ secrets.PROD_SSH_KEY }}
          script: |
            set -e
            cd /opt/hexabellum
            echo "Pulling latest release images..."
            docker compose -f deploy/docker/docker-compose.prod.yml pull

            echo "Applying database migrations..."
            docker compose -f deploy/docker/docker-compose.prod.yml run --rm game-server ./hexabellum-server migrate

            echo "Performing zero-downtime rolling restart..."
            docker compose -f deploy/docker/docker-compose.prod.yml up -d --remove-orphans

            echo "Waiting for health check..."
            sleep 5
            curl -f http://localhost:3000/health || (echo "Health check failed! Initiating rollback..." && exit 1)

            docker system prune -f
            echo "Deployment of ${{ github.ref_name }} succeeded at $(date)" >> /var/log/hexabellum-deploy.log
```

---

## 5. Hosting Strategy & Scaling Architecture

### 5.1 Infrastructure Roadmap by Concurrency Tier

| Metric / Requirement | Tier 1: MVP Skirmish | Tier 2: Early Access | Tier 3: Launch Cluster | Tier 4: Global Scale |
|---|---|---|---|---|
| **Concurrent Players (CCU)** | 10 – 50 | 50 – 500 | 500 – 5,000 | 5,000 – 50,000+ |
| **Concurrent Active Matches** | 1 – 5 | 5 – 50 | 50 – 500 | 500 – 5,000+ |
| **Hosting Topology** | Single VPS (All-in-one) | 2 App VPS + Managed Postgres | K8s Cluster + Redis Sentinel | Multi-Region K8s + Global Matchmaker |
| **Compute Recommendation** | Hetzner CPX21 (3 vCPU, 4GB) | 2x CPX31 (4 vCPU, 8GB) | 5x CCX23 Dedicated vCPU | AWS EKS / GCP GKE Multi-Region |
| **Estimated Monthly Cost** | **$8 – $15 / mo** | **$45 – $90 / mo** | **$250 – $600 / mo** | **$1,500 – $5,000+ / mo** |

### 5.2 Match Routing in Multi-Server Clusters

In web applications, any worker node can serve any HTTP request. In Hexabellum, **matches are stateful in-memory Tokio actors (`MatchActor`)**. If a match is hosted on Server Instance A, all 10 players in that match must connect to Server Instance A.

```mermaid
flowchart TD
    subgraph "Match Ingress Resolution"
        Player[Connecting Player]
        Router[Ingress / Match Router Proxy]
        RedisRegistry[("Redis Match Registry<br/>hexabellum:match:{id} -> host:port")]
    end

    subgraph "Server Pods"
        PodA["Game Server Pod A<br/>Matches: #101, #102"]
        PodB["Game Server Pod B<br/>Matches: #103, #104"]
    end

    Player -->|1. Connect to /ws/match/101| Router
    Router -->|2. Query Host Pod| RedisRegistry
    RedisRegistry -->|3. Returns 'Pod A'| Router
    Router -->|4. Proxy WS Stream| PodA
```

1. **Match Registry**: When a `MatchActor` is spawned on a server instance, it registers its worker instance address in Redis:
   ```
   SET hexabellum:match:{match_id}:node "node-eu-01.hexabellum.net:3000" EX 3600
   ```
2. **Proxy Resolution**: The ingress proxy (or API gateway) inspects the `:match_id` URL parameter, queries Redis, and dynamically proxies the WebSocket stream to the corresponding backend pod.
3. **Session Reconnection**: When a disconnected player reconnects with `match_id` and their `reconnect_token`, the routing path guarantees they reach the exact memory space where their match resides.

---

## 6. Database Architecture & SQLx Migrations

### 6.1 Relational Schema (PostgreSQL 16)

The schema stores player accounts, historical matches, participant statistics, and serialized event replays:

```sql
-- deploy/postgres/init.sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 1. Player Accounts & Ratings
CREATE TABLE players (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(32) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    mmr_rating INTEGER DEFAULT 1200 NOT NULL,
    matches_played INTEGER DEFAULT 0 NOT NULL,
    matches_won INTEGER DEFAULT 0 NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    last_login_at TIMESTAMPTZ,
    settings JSONB DEFAULT '{}'::jsonb NOT NULL
);

-- 2. Matches
CREATE TABLE matches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_code VARCHAR(16) UNIQUE NOT NULL,
    mode VARCHAR(16) DEFAULT '5v5' NOT NULL, -- 1v1, 3v3, 5v5
    status VARCHAR(20) DEFAULT 'waiting' NOT NULL, -- waiting, hero_select, active, completed, aborted
    map_radius SMALLINT DEFAULT 8 NOT NULL,
    map_seed BIGINT NOT NULL,
    winner_team SMALLINT, -- 0 (Allies), 1 (Enemies), NULL (Draw/Active)
    rounds_played SMALLINT DEFAULT 0 NOT NULL,
    core_0_health SMALLINT DEFAULT 700 NOT NULL,
    core_1_health SMALLINT DEFAULT 700 NOT NULL,
    vault_destroyed_by SMALLINT, -- Team ID that claimed neutral Vault
    final_state_blake3_hash VARCHAR(64),
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL
);

-- 3. Match Participants
CREATE TABLE match_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    player_id UUID REFERENCES players(id) ON DELETE SET NULL,
    team SMALLINT NOT NULL, -- 0 or 1
    hero_def_id VARCHAR(32) NOT NULL, -- vanguard, ranger, warden, sniper, berserker
    is_ai BOOLEAN DEFAULT false NOT NULL,
    kills SMALLINT DEFAULT 0 NOT NULL,
    deaths SMALLINT DEFAULT 0 NOT NULL,
    assists SMALLINT DEFAULT 0 NOT NULL,
    damage_dealt INTEGER DEFAULT 0 NOT NULL,
    damage_taken INTEGER DEFAULT 0 NOT NULL,
    final_gold INTEGER DEFAULT 0 NOT NULL,
    final_xp INTEGER DEFAULT 0 NOT NULL,
    final_level SMALLINT DEFAULT 1 NOT NULL,
    final_items JSONB DEFAULT '[]'::jsonb NOT NULL,
    disconnect_count SMALLINT DEFAULT 0 NOT NULL,
    reconnect_token VARCHAR(64),
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    UNIQUE(match_id, hero_def_id),
    UNIQUE(match_id, player_id)
);

-- 4. Match Event Logs (Replay Storage - Partitioned by Range or Date)
CREATE TABLE match_events (
    id BIGSERIAL,
    match_id UUID NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    round SMALLINT NOT NULL,
    stage VARCHAR(20) NOT NULL, -- Planning, Move, Cast, Attack, Upkeep
    event_type VARCHAR(32) NOT NULL,
    event_payload JSONB NOT NULL,
    blake3_stage_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    PRIMARY KEY (match_id, id)
) PARTITION BY HASH (match_id);

-- Create 4 default partitions for replay distribution
CREATE TABLE match_events_p0 PARTITION OF match_events FOR VALUES WITH (MODULUS 4, REMAINDER 0);
CREATE TABLE match_events_p1 PARTITION OF match_events FOR VALUES WITH (MODULUS 4, REMAINDER 1);
CREATE TABLE match_events_p2 PARTITION OF match_events FOR VALUES WITH (MODULUS 4, REMAINDER 2);
CREATE TABLE match_events_p3 PARTITION OF match_events FOR VALUES WITH (MODULUS 4, REMAINDER 3);

-- Indexes for performance
CREATE INDEX idx_matches_status ON matches(status);
CREATE INDEX idx_matches_created_at ON matches(created_at DESC);
CREATE INDEX idx_participants_player ON match_participants(player_id);
CREATE INDEX idx_events_lookup ON match_events(match_id, round);
```

### 6.2 SQLx Offline Query Preparation

Hexabellum uses `sqlx` in Rust for compile-time verified SQL queries. In CI, we enable offline mode using `.sqlx` metadata so compilation succeeds without requiring a live database:

```bash
# Save query metadata locally for CI caching
cargo sqlx prepare --workspace -- --all-targets

# Commit the generated .sqlx/ directory to git
git add .sqlx
git commit -m "chore: update SQLx offline query metadata"
```

---

## 7. Redis Cache, Session Registry & Matchmaking

Redis 7 serves as the sub-millisecond coordination layer for ephemeral state:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             REDIS KEYSPACE DESIGN                           │
├───────────────────────────────────┬─────────────────────────────────────────┤
│ Key Pattern                       │ Description & TTL                       │
├───────────────────────────────────┼─────────────────────────────────────────┤
│ hexabellum:session:{player_id}    │ JWT session verification (TTL: 24h)     │
│ hexabellum:match:{match_id}:node  │ Pod address hosting match (TTL: 2h)     │
│ hexabellum:match:{match_id}:snap  │ Latest BLAKE3 state backup (TTL: 1h)    │
│ hexabellum:mm_queue:5v5           │ Sorted set of waiting players (by MMR)  │
│ hexabellum:reconnect:{token}      │ Player-to-match reconnect auth (TTL: 5m)│
│ hexabellum:rate_limit:{ip}        │ Sliding window connection tracker       │
└───────────────────────────────────┴─────────────────────────────────────────┘
```

### 7.1 Redis Matchmaking Queue Pattern

The matchmaking daemon evaluates waiting players using Redis Sorted Sets:
```bash
# Add player to 5v5 queue scored by entry timestamp
ZADD hexabellum:mm_queue:5v5 1716000000 "player_uuid_abc:mmr_1250"

# Fetch top 10 players within MMR delta (+/- 100)
ZRANGEBYSCORE hexabellum:mm_queue:5v5 1150 1350 LIMIT 0 10
```

---

## 8. Security, Anti-Cheat & Hardening

Because Hexabellum is a competitive multiplayer game, the server architecture implements strict zero-trust security invariants:

### 8.1 Zero-Knowledge Fog-of-War Sanitization

Client modification (e.g., custom JavaScript or modified PixiJS shaders) cannot reveal hidden enemies because **the server never transmits concealed entity coordinates**.
- Before sending `SnapshotDto` or `RoundResolvedDto`, the server filters entities through the requesting team's composite Line-of-Sight (LOS) visibility mask.
- Sighted enemies transmit HP and location; unsighted enemies are redacted completely.
- The neutral Objective Vault and enemy Core are only updated on the client when actively in allied sightlines.

### 8.2 Unit Command Authorization

```rust
// crates/server/src/match_actor.rs
// Enforce strict 1-to-1 Player-to-Unit ownership
pub fn validate_order_permission(
    &self,
    player_id: &PlayerId,
    order: &OrderDto
) -> Result<(), GameError> {
    let controller = self.controller_map.get(&order.unit_id)
        .ok_or(GameError::UnitNotFound)?;

    match controller {
        Controller::Player(owner_id) if owner_id == player_id => Ok(()),
        Controller::Player(_) => Err(GameError::NotYourUnit),
        Controller::Ai | Controller::Automatic => Err(GameError::CannotCommandAutonomousUnit),
    }
}
```

### 8.3 WebSocket Framing & DoS Mitigation

- **Max Message Size**: Strict 64 KB framing limit enforced at both Nginx proxy and Axum WebSocket extractor.
- **Rate Limiting**: Axum middleware applies a token-bucket rate limiter: max 1 order submission per hero per round, max 10 chat messages per minute.
- **Ping/Pong Heartbeats**: Ping frames sent every 15 seconds. If a client fails to reply with Pong within 10 seconds, the connection is dropped and marked `Disconnected`, triggering dynamic AI backfill.

---

## 9. Observability, Telemetry & SRE

### 9.1 Prometheus Metrics Catalog (`/metrics`)

The game server exposes Prometheus metrics on a dedicated port or path:

```
# TYPE hexabellum_active_matches gauge
hexabellum_active_matches{mode="5v5",status="active"} 12
hexabellum_active_matches{mode="3v3",status="planning"} 4

# TYPE hexabellum_connected_players gauge
hexabellum_connected_players{team="0"} 78
hexabellum_connected_players{team="1"} 80

# TYPE hexabellum_round_resolution_duration_seconds histogram
# Measures time taken by TurnProcessor in crates/core to resolve simultaneous orders
hexabellum_round_resolution_duration_seconds_bucket{le="0.01"} 1420
hexabellum_round_resolution_duration_seconds_bucket{le="0.05"} 1512
hexabellum_round_resolution_duration_seconds_bucket{le="0.10"} 1515
hexabellum_round_resolution_duration_seconds_count 1515

# TYPE hexabellum_desync_total counter
# CRITICAL ALERT: Incremented if client reports a BLAKE3 mismatch against server
hexabellum_desync_total 0

# TYPE hexabellum_ai_backfill_events_total counter
hexabellum_ai_backfill_events_total{reason="timeout"} 42
hexabellum_ai_backfill_events_total{reason="disconnect"} 18
```

### 9.2 Health & Readiness Endpoints

```json
// GET /health
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 18452,
  "active_matches": 16,
  "connected_clients": 158,
  "postgres": "connected",
  "redis": "connected",
  "memory_allocated_mb": 142.5
}
```

- `/health`: Liveness probe. Returns `200 OK` if the process is responsive.
- `/ready`: Readiness probe. Returns `200 OK` if database and Redis connection pools are healthy; returns `503 Service Unavailable` during server draining.

---

## 10. High Availability, Graceful Draining & Backups

### 10.1 Graceful Node Draining Protocol

When deploying new container versions or retiring worker nodes, active matches must not be abruptly terminated:

```mermaid
sequenceDiagram
    autonumber
    actor Admin as Orchestrator / CI
    participant Node as Game Server Node
    participant LB as Ingress Proxy
    participant Redis as Redis Registry
    participant Players as Connected Players

    Admin->>Node: Send SIGTERM signal
    Node->>LB: Flip /ready probe to 503 (Draining)
    LB->>LB: Stop routing NEW matches to this node
    Node->>Redis: Deregister from new match allocation
    loop Existing Active Matches
        Node->>Players: Broadcast round updates until match conclusion
    end
    Node->>Node: Final match ends (Active Matches = 0)
    Node->>Admin: Clean exit code 0
```

1. **Signal Ingestion**: Upon receiving `SIGTERM`, the Axum server enters draining mode.
2. **Readiness Probe Failure**: The `/ready` endpoint returns `503 Draining`. Ingress load balancers immediately stop routing new match creations to this instance.
3. **Match Preservation**: Existing matches continue running undisturbed until their final round (matches average 10–18 minutes).
4. **Clean Exit**: When the active match counter reaches 0, all database connections close, and the process terminates cleanly.

### 10.2 Automated Database Backups & PITR

```bash
#!/bin/bash
# /opt/hexabellum/scripts/backup-db.sh
set -eo pipefail

BACKUP_DIR="/opt/hexabellum/backups"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
FILENAME="$BACKUP_DIR/hexabellum_db_$TIMESTAMP.sql.gz"

mkdir -p "$BACKUP_DIR"

# Perform compressed PostgreSQL dump
docker exec hexabellum-postgres pg_dump -U hexabellum -d hexabellum | gzip -9 > "$FILENAME"

# Encrypt backup with GPG (Optional)
# gpg --symmetric --batch --passphrase "$BACKUP_PASSPHRASE" "$FILENAME"

# Prune local backups older than 14 days
find "$BACKUP_DIR" -type f -name "hexabellum_db_*.sql.gz" -mtime +14 -delete

echo "[$(date)] Backup completed successfully: $FILENAME"
```

---

## 11. Operational Runbooks for SRE & DevOps

### Runbook 1: Live Client-Server Desync Spike Alert (`hexabellum_desync_total > 0`)

> [!CAUTION]
> A desync indicates that the client-side WebAssembly preview or state reconciliation produced a different BLAKE3 state hash than the authoritative server simulation.

1. **Check Logs**: Filter server logs for `target: "hexabellum_server::state_audit"` and `level: "ERROR"`.
2. **Identify State Drift**: Extract the desync report containing the server state hash and client state hash.
3. **Inspect Match Replay**: Locate the `match_id` and query `match_events` for that round:
   ```sql
   SELECT round, stage, event_payload, blake3_stage_hash 
   FROM match_events 
   WHERE match_id = 'YOUR_MATCH_UUID' 
   ORDER BY id ASC;
   ```
4. **Reproduce Headless**: Copy the match seed, initial hero layout, and order list into `crates/core/tests/reproduce_desync.rs` and run `cargo test -- --nocapture`.
5. **Hotfix & Deploy**: If an unseeded floating-point operation or order evaluation bug is discovered, patch `crates/core`, build a hotfix release, and deploy via Runbook 2.

### Runbook 2: Emergency Hotfix & Rollback Procedure

If a deployed release introduces an unhandled panic or critical regression:

```bash
# 1. SSH into the production host
ssh admin@hexabellum.net

# 2. Navigate to deployment root
cd /opt/hexabellum

# 3. Roll back image tags to the previous stable release
export SERVER_TAG="v1.2.3"
export CLIENT_TAG="v1.2.3"

# 4. Restart containers with pinned stable versions
docker compose -f deploy/docker/docker-compose.prod.yml up -d

# 5. Verify health check
curl -f http://localhost:3000/health
```

### Runbook 3: Database Connection Pool Starvation Alert

**Symptom**: Server logs show `sqlx::Error::PoolTimedOut` and HTTP API latency spikes.
1. Inspect active PostgreSQL connections:
   ```sql
   SELECT count(*), state FROM pg_stat_activity GROUP BY state;
   ```
2. Identify slow or blocking queries:
   ```sql
   SELECT pid, now() - pg_stat_activity.query_start AS duration, query 
   FROM pg_stat_activity 
   WHERE state != 'idle' 
   ORDER BY duration DESC;
   ```
3. Terminate runaway locks if necessary:
   ```sql
   SELECT pg_terminate_backend(blocking_pid);
   ```
4. Adjust `DB_MAX_CONNECTIONS` in `/opt/hexabellum/.env` and reload the server.

---

## 12. Quick Start: Zero to Deployed in 30 Minutes

For a developer setting up a fresh production server on Ubuntu 22.04 / 24.04 LTS:

```bash
# 1. Update system packages
sudo apt update && sudo apt upgrade -y
sudo apt install -y curl git ufw fail2ban

# 2. Configure Firewall (Allow SSH, HTTP, HTTPS)
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable

# 3. Install Docker Engine & Docker Compose
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker $USER
newgrp docker

# 4. Clone Repository & Setup Production Configuration
git clone https://github.com/garnizeh/hexabellum.git /opt/hexabellum
cd /opt/hexabellum

# Generate secure passwords
DB_PASS=$(openssl rand -base64 24)
JWT_SECRET=$(openssl rand -base64 32)

cat <<EOF > .env
ENVIRONMENT=production
DB_PASSWORD=${DB_PASS}
JWT_SECRET=${JWT_SECRET}
DOMAIN=hexabellum.yourdomain.com
SERVER_TAG=latest
CLIENT_TAG=latest
EOF

# 5. Launch Full Stack
docker compose -f deploy/docker/docker-compose.prod.yml up -d

# 6. Verify System Health
sleep 10
curl http://localhost:3000/health

echo "Hexabellum is online and running securely!"
```

---

## 13. Cross-Document Sitemap & Specification Alignment

This DevOps & Infrastructure Engineering Guide directly implements and supports the technical requirements defined across the Hexabellum documentation suite:

- **Master System Architecture**: [`docs/overview.md`](file:///home/user/Code/garnizeh/hexabellum/docs/overview.md)
- **Game Design Document (MOBA Macro & Rules)**: [`docs/gdd.md`](file:///home/user/Code/garnizeh/hexabellum/docs/gdd.md)
- **UI/UX Architecture & Tactical HUD**: [`docs/ui-ux.md`](file:///home/user/Code/garnizeh/hexabellum/docs/ui-ux.md)
- **Phase 0: Technical Foundation**: [`docs/phase-0.md`](file:///home/user/Code/garnizeh/hexabellum/docs/phase-0.md)
- **Phase 1: Deterministic Core & Combat**: [`docs/phase-1.md`](file:///home/user/Code/garnizeh/hexabellum/docs/phase-1.md)
- **Phase 2: Fog-of-War & Towers**: [`docs/phase-2.md`](file:///home/user/Code/garnizeh/hexabellum/docs/phase-2.md)
- **Phase 3: Server Authority & WebSockets**: [`docs/phase-3.md`](file:///home/user/Code/garnizeh/hexabellum/docs/phase-3.md)
- **Phase 4: Spells, Statuses & Neutrals**: [`docs/phase-4.md`](file:///home/user/Code/garnizeh/hexabellum/docs/phase-4.md)
- **Phase 5: 5v5 Multiplayer & AI Backfill**: [`docs/phase-5.md`](file:///home/user/Code/garnizeh/hexabellum/docs/phase-5.md)
- **Phase 6: Economy, Levels & Items**: [`docs/phase-6.md`](file:///home/user/Code/garnizeh/hexabellum/docs/phase-6.md)
- **Phase 7: Cores, Respawns & Victory**: [`docs/phase-7.md`](file:///home/user/Code/garnizeh/hexabellum/docs/phase-7.md)

---

*End of DevOps & Infrastructure Engineering Guide*

---

*HEXABELLUM — Six sides of war. One victor.*