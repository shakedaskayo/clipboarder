.PHONY: help dev build dmg test lint docs docs-install icon clean fmt

help:
	@echo "clipboarder — development targets"
	@echo ""
	@echo "  make dev          run vite + tauri dev with HMR"
	@echo "  make build        full release build (.app + .dmg)"
	@echo "  make dmg          build only the .dmg installer"
	@echo "  make test         cargo check + tsc --noEmit"
	@echo "  make lint         cargo clippy + cargo fmt --check"
	@echo "  make fmt          cargo fmt + format frontend"
	@echo "  make docs         serve MkDocs site at localhost:8000"
	@echo "  make docs-install install MkDocs requirements"
	@echo "  make icon         regenerate the app icon from scripts/make_icon.py"
	@echo "  make clean        remove target/ and dist/"

dev:
	npm install --silent
	npm run tauri dev

build:
	npm install --silent
	npm run tauri build

dmg: build
	@echo "Output:"
	@ls -lh src-tauri/target/release/bundle/dmg/*.dmg

test:
	cargo check --manifest-path src-tauri/Cargo.toml
	npx tsc --noEmit

lint:
	cargo clippy --manifest-path src-tauri/Cargo.toml --no-deps -- -D warnings -A unexpected_cfgs -A deprecated
	cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

fmt:
	cargo fmt --manifest-path src-tauri/Cargo.toml

docs-install:
	pip install -r docs/requirements.txt

docs:
	cd docs && mkdocs serve

icon:
	python3 scripts/make_icon.py
	cd src-tauri/icons && iconutil -c icns icon.iconset -o icon.icns

clean:
	rm -rf src-tauri/target dist
