.PHONY: all build clean typegen package xcode sim-build

SHARED_DIR := ../shared

all: build

build: typegen package xcode

# Phase 1: generate SharedTypes (domain types as a Swift package)
typegen:
	@echo "Generating SharedTypes..."
	@RUST_LOG=info cargo run --manifest-path $(SHARED_DIR)/Cargo.toml \
		--bin codegen --features codegen,facet_typegen -- \
		--language swift --output-dir generated

# Phase 2: build Shared (UniFFI bindings as a Swift package via cargo-swift)
package:
	@echo "Building Shared Swift package..."
	@cd $(SHARED_DIR) && \
		cargo swift package --name Shared --platforms ios \
			--lib-type static --features uniffi && \
		rm -rf ../iOS/generated/Shared && \
		mkdir -p ../iOS/generated/Shared && \
		cp -r Shared/* ../iOS/generated/Shared/ && \
		rm -rf Shared

# Phase 3: generate Xcode project from project.yml
xcode:
	@echo "Generating Xcode project..."
	@xcodegen

# Build the iOS app for the simulator
sim-build:
	@xcodebuild build \
		-project __APP_NAME__.xcodeproj \
		-scheme __APP_NAME__ \
		-destination 'platform=iOS Simulator,name=iPhone 16' \
		-configuration Debug \
		CODE_SIGNING_ALLOWED=NO \
		2>&1 | xcbeautify

clean:
	@rm -rf generated/ *.xcodeproj
