#!/usr/bin/env bash
cd "$(dirname "$0")"
cargo run --manifest-path ../rafx/Cargo.toml --release --package rafx-shader-processor -- \
--glsl-path glsl \
--spv-path processed_shaders \
--dx12-generated-src-path processed_shaders \
--cooked-shaders-path cooked_shaders \
--package-vk --package-dx12 \
--rs-mod-path src/shaders/
