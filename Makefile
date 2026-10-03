.PHONY: build-wasm dev build test clean build-server run-server

# Build WASM package
build-wasm:
	cd crates/wasm && wasm-pack build --target web --out-dir ../../web/src/wasm/pkg

# Build server binary
build-server:
	cargo build -p hexabellum-server --release

# Run server
run-server:
	cargo run -p hexabellum-server

# Start dev server
dev: build-wasm
	cd web && npm run dev

# Build for production
build: build-wasm build-server
	cd web && npm run build

# Run all tests (Rust workspace)
test:
	cargo test --workspace

# Clean
clean:
	rm -rf web/src/wasm/pkg
	rm -rf web/dist
	cargo clean
