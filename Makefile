# Keres — build orchestration for the three binaries that share the
# `keres_engine` library crate.
#
# Each binary is built with the profile + Cargo features that suit its purpose:
#
#   keres  (CLI)      [profile.release]   speed — AI inspection / debugging tool
#   server (HTTP API) [profile.release]   speed — production AI latency
#   gui    (minifb)   [profile.gui]       size  — micro-keres <1.44 MB lineage
#
# The `gui` feature pulls in minifb; it is OPTIONAL so the server/CLI (and the
# musl Docker server build) stay free of the X11/minifb dependency. See the
# per-profile settings in Cargo.toml.

CARGO      ?= cargo
GUI_PROFILE := gui

# Binaries land in target/<profile>/ — release for cli/server, the named
# profile for the gui.
CLI_BIN    := target/release/keres
SERVER_BIN := target/release/server
GUI_BIN    := target/$(GUI_PROFILE)/gui

.DEFAULT_GOAL := all

.PHONY: all help cli server gui test test-core fmt fmt-fix clippy check sizes \
        run-cli run-server run-gui clean icons base symbols splash pixel-assets

all: cli server gui  ## Build all three binaries

##@ Build

cli:  ## Plain-text CLI (keres): show-moves / engine-move / debug-tree
	$(CARGO) build --release --bin keres

server:  ## HTTP server (the binary wire API; see docs/PROTOCOL.md)
	$(CARGO) build --release --bin server

gui: src/gui/icons.rs src/gui/base.rs src/gui/symbols.rs src/gui/window_icon.rs src/gui/splash.rs  ## Native minifb desktop GUI (size-optimized; enables the `gui` feature)
	$(CARGO) build --profile $(GUI_PROFILE) --bin gui --features gui
	@# UPX is applied opportunistically — it shrinks the GUI another ~50-60%
	@# but needs the `upx` binary (dnf install upx / apt install upx). No-op
	@# if absent, so the target stays deterministic without it.
	@if command -v upx >/dev/null 2>&1; then \
		echo "  upx: compressing $(GUI_BIN)"; \
		upx --best --lzma --quiet $(GUI_BIN) || true; \
	else \
		echo "  tip: install upx to compress $(GUI_BIN) further"; \
	fi

##@ Quality

test:  ## Full test suite incl. GUI module tests (headless; needs the gui feature)
	$(CARGO) test --workspace --features gui

test-core:  ## Test only the engine/CLI/server (no minifb feature)
	$(CARGO) test --workspace

fmt:  ## Check formatting (CI-blocking)
	$(CARGO) fmt --check

fmt-fix:  ## Apply rustfmt
	$(CARGO) fmt

clippy:  ## Lint everything incl. the GUI (informational; a pre-existing backlog means CI runs the strict -D pass with continue-on-error)
	$(CARGO) clippy --workspace --all-targets --features gui

check: fmt clippy test  ## fmt + clippy + full test suite

##@ Inspect & run

sizes:  ## Print built binary sizes
	@for p in "$(CLI_BIN):cli" "$(SERVER_BIN):server" "$(GUI_BIN):gui"; do \
		path="$${p%%:*}"; name="$${p##*:}"; \
		if [ -f "$$path" ]; then \
			sz=$$(stat -c%s "$$path"); \
			awk -v n="$$name" -v p="$$path" -v s="$$sz" \
				'BEGIN{printf "  %-7s %9d B  %7.1f KB   %s\n", n, s, s/1024, p}'; \
		else \
			printf "  %-7s (not built — run 'make %s')\n" "$$name" "$$name"; \
		fi; \
	done

run-cli: cli  ## Run the CLI, e.g. `make run-cli ARGS='engine-move'`
	./$(CLI_BIN) $(ARGS)

run-server: server  ## Run the HTTP server (PORT env var selects the listen port)
	./$(SERVER_BIN)

run-gui: gui  ## Run the native GUI
	./$(GUI_BIN)

##@ Pixel assets

ICON_SRCS := $(wildcard assets/pixel/icons/*.xcf)
BASE_SRCS := $(wildcard assets/pixel/base/*.xcf)
SYMBOL_SRCS := $(wildcard assets/pixel/symbols/*.xcf)

src/gui/icons.rs: $(ICON_SRCS) scripts/gen_icons.py scripts/pixel_raster.py
	python3 scripts/gen_icons.py

src/gui/base.rs: $(BASE_SRCS) scripts/gen_base.py scripts/pixel_raster.py
	python3 scripts/gen_base.py

src/gui/symbols.rs: $(SYMBOL_SRCS) scripts/gen_symbols.py scripts/pixel_raster.py
	python3 scripts/gen_symbols.py

src/gui/window_icon.rs: assets/pixel/logo.xcf scripts/gen_window_icon.py scripts/pixel_raster.py
	python3 scripts/gen_window_icon.py

src/gui/splash.rs: assets/pixel/logo.xcf assets/pixel/title.xcf scripts/gen_splash.py scripts/pixel_raster.py
	python3 scripts/gen_splash.py

icons: src/gui/icons.rs  ## Regenerate src/gui/icons.rs from assets/pixel/icons/*.xcf

base: src/gui/base.rs  ## Regenerate src/gui/base.rs from assets/pixel/base/*.xcf

symbols: src/gui/symbols.rs  ## Regenerate src/gui/symbols.rs from assets/pixel/symbols/*.xcf

window-icon: src/gui/window_icon.rs  ## Regenerate src/gui/window_icon.rs (window/taskbar icon)

splash: src/gui/splash.rs  ## Regenerate src/gui/splash.rs (splash-screen crest/wordmark) from assets/pixel/logo.xcf, title.xcf

pixel-assets: icons base symbols window-icon splash  ## Regenerate all generated pixel-art Rust sources

##@ Misc

clean:  ## Remove all build artifacts
	$(CARGO) clean

help:  ## Show this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*##/ { printf "  \033[36m%-11s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
