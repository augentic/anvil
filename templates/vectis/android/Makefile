.PHONY: all build clean lib typegen android-lib apk install launch run

WORKSPACE := ..

all: build

build: typegen

typegen: lib
	@echo "Generating Kotlin types..."
	@cd $(WORKSPACE) && RUST_LOG=info cargo run \
		--package shared \
		--bin codegen \
		--features codegen,facet_typegen \
		-- \
		--language kotlin \
		--output-dir Android/generated

lib:
	@echo "Building Rust shared library..."
	@cd $(WORKSPACE) && cargo build --features uniffi

android-lib:
	@echo "Cross-compiling Rust for Android..."
	@./gradlew :shared:cargoBuild

apk: typegen
	@echo "Building debug APK..."
	@./gradlew :app:assembleDebug

install: apk
	@./gradlew :app:installDebug

launch:
	@adb shell am start -n __ANDROID_PACKAGE__/.MainActivity

run: install launch

clean:
	@./gradlew clean
	@rm -rf generated/
