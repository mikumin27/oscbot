ARG CUDA_VER=12.8.0
ARG UBUNTU_VER=24.04
ARG BUILD_IMAGE=oscbot-build:latest

FROM rust:1.93.0-bookworm@sha256:d0a4aa3ca2e1088ac0c81690914a0d810f2eee188197034edf366ed010a2b382 AS oscbot-builder
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG OSCBOT_PROFILE=release
ARG OSCBOT_LTO=thin
ARG OSCBOT_CODEGEN_UNITS=16
ARG OSCBOT_INCREMENTAL=0

ENV CARGO_PROFILE_RELEASE_STRIP=symbols \
    CARGO_PROFILE_RELEASE_LTO=${OSCBOT_LTO} \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=${OSCBOT_CODEGEN_UNITS} \
    CARGO_INCREMENTAL=${OSCBOT_INCREMENTAL}

RUN --mount=type=cache,id=apt-cache-rust,target=/var/cache/apt,sharing=locked \
  --mount=type=cache,id=apt-lists-rust,target=/var/lib/apt/lists,sharing=locked \
  apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libssl-dev ca-certificates build-essential mold

ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src

RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cargo-target-release,target=/app/target-release \
    --mount=type=cache,id=cargo-target-debug,target=/app/target-debug \
    set -eux; \
    mkdir -p /out; \
    if [ "${OSCBOT_PROFILE}" = "release" ]; then \
      export CARGO_TARGET_DIR=/app/target-release; \
      cargo build --release --locked; \
      cp /app/target-release/release/oscbot /out/oscbot; \
    else \
      export CARGO_TARGET_DIR=/app/target-debug; \
      cargo build --locked; \
      cp /app/target-debug/debug/oscbot /out/oscbot; \
    fi

FROM nvidia/cuda:${CUDA_VER}-runtime-ubuntu${UBUNTU_VER}@sha256:44e43f0e0bcca1fc6fdc775e6002c67834bf78d39eb1fd76825240fc79ba4a49 AS final
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG BUILD_IMAGE

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates tini \
      libssl3 \
      libglvnd0 libegl1 libgles2 libgl1 libgtk-3-0 libglib2.0-0 \
    && apt-get clean

RUN install -d -m 755 /etc/glvnd/egl_vendor.d \
 && cat >/etc/glvnd/egl_vendor.d/10_nvidia.json <<'EOF'
{
  "file_format_version": "1.0.0",
  "ICD": { "library_path": "libEGL_nvidia.so.0" }
}
EOF

RUN groupadd -g 1000 appuser 2>/dev/null || true \
 && id -u 1000 >/dev/null 2>&1 || useradd -u 1000 -g 1000 -m -s /bin/bash appuser \
 && install -d -m 755 -o 1000 -g 1000 /app/danser /app/oscbot

COPY --link --from=${BUILD_IMAGE} --chown=1000:1000 --chmod=755 /out/danser /app/danser
COPY --link --from=oscbot-builder --chown=1000:1000 --chmod=755 /out/oscbot /app/oscbot/oscbot

COPY --chown=1000:1000 default-danser.json /app/danser/settings/default.json
COPY --chown=1000:1000 src/generate/data   /app/oscbot/src/generate/data

RUN printf "%s\n" /app/danser /app/danser/ffmpeg/lib >/etc/ld.so.conf.d/app-danser.conf \
 && ldconfig

ENV PATH="/app/danser/ffmpeg/bin:${PATH}"
ENV LD_LIBRARY_PATH=/usr/local/nvidia/lib:/usr/local/nvidia/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}

USER 1000:1000
WORKDIR /app/oscbot
ENTRYPOINT ["tini","--","/app/oscbot/oscbot"]
