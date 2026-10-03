.PHONY: build-wasm dev build test clean

# Build WASM package
build-wasm:
	cd crates/wasm && wasm-pack build --target web --out-dir ../../web/src/wasm/pkg

# Start dev server
dev: build-wasm
	cd web && npm run dev

# Build for production
build: build-wasm
	cd web && npm run build

# Run Rust tests
test:
	cargo test

# Clean
clean:
	rm -rf web/src/wasm/pkg
	rm -rf web/dist
	cargo clean
