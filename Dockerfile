# syntax=docker/dockerfile:1.7-labs

ARG CUDA_VER=12.8.0
ARG UBUNTU_VER=24.04

ARG DANSER_REPO=https://github.com/Wieku/danser-go.git
ARG DANSER_COMMIT=e97c891604b08d0992b915772965b1d594ad530d

ARG FFMPEG_TAG=autobuild-2026-01-08-12-59
ARG FFMPEG_ASSET=ffmpeg-N-122390-gaf6a1dd0b2-linux64-gpl-shared.tar.xz
ARG FFMPEG_SHA256=508c6de70a7ec2840d514ba8ce8bd48aaebc6529b16db435066c876c79a243fc

FROM golang:1.25.6-bookworm@sha256:2f768d462dbffbb0f0b3a5171009f162945b086f326e0b2a8fd5d29c3219ff14 AS danser-builder
ARG DANSER_REPO
ARG DANSER_COMMIT
ARG FFMPEG_TAG
ARG FFMPEG_ASSET
ARG FFMPEG_SHA256

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
ENV GOOS=linux GOARCH=amd64 CGO_ENABLED=1

RUN --mount=type=cache,id=apt-cache-danser,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,id=apt-lists-danser,target=/var/lib/apt/lists,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
      git git-lfs ca-certificates curl xz-utils \
      xorg-dev libgl1-mesa-dev libgtk-3-dev

RUN set -eux; \
    mkdir -p /src/danser; \
    git init /src/danser; \
    git -C /src/danser remote add origin "${DANSER_REPO}"; \
    git -C /src/danser fetch --depth 1 origin "${DANSER_COMMIT}"; \
    git -C /src/danser checkout -q FETCH_HEAD; \
    git -C /src/danser lfs install --system; \
    git -C /src/danser lfs pull

WORKDIR /src/danser

RUN --mount=type=cache,id=go-build,target=/root/.cache/go-build \
    --mount=type=cache,id=go-mod,target=/go/pkg/mod \
    set -eux; \
    export CC=gcc CXX=g++; \
    BUILD_DIR=/tmp/danser-build-linux; \
    mkdir -p "$BUILD_DIR"; \
    go run -buildvcs=false tools/assets/assets.go ./ "$BUILD_DIR/"; \
    go build -buildvcs=false -trimpath \
      -ldflags "-s -w -X 'github.com/wieku/danser-go/build.VERSION=dev-${DANSER_COMMIT}' -X 'github.com/wieku/danser-go/build.Stream=Release'" \
      -buildmode=c-shared -o "$BUILD_DIR/danser-core.so" \
      -tags "exclude_cimgui_glfw exclude_cimgui_sdli"; \
    mv "$BUILD_DIR/danser-core.so" "$BUILD_DIR/libdanser-core.so"; \
    cp libbass.so libbass_fx.so libbassmix.so libyuv.so libSDL3.so "$BUILD_DIR/"; \
    gcc -no-pie -O3 -o "$BUILD_DIR/danser-cli" -I. cmain/main_danser.c -I"$BUILD_DIR/" -Wl,-rpath,'$ORIGIN' -L"$BUILD_DIR/" -ldanser-core; \
    gcc -no-pie -O3 -D LAUNCHER -o "$BUILD_DIR/danser" -I. cmain/main_danser.c -I"$BUILD_DIR/" -Wl,-rpath,'$ORIGIN' -L"$BUILD_DIR/" -ldanser-core; \
    strip "$BUILD_DIR/danser" "$BUILD_DIR/danser-cli" 2>/dev/null || true; \
    rm -f "$BUILD_DIR/danser-core.h"

RUN set -eux; \
    BUILD_DIR=/tmp/danser-build-linux; \
    mkdir -p "$BUILD_DIR/ffmpeg"; \
    url="https://github.com/BtbN/FFmpeg-Builds/releases/download/${FFMPEG_TAG}/${FFMPEG_ASSET}"; \
    curl -fSL --retry 5 --retry-connrefused --retry-delay 3 "$url" -o /tmp/ffmpeg.tar.xz; \
    echo "${FFMPEG_SHA256}  /tmp/ffmpeg.tar.xz" | sha256sum -c -; \
    \
    tar -xJf /tmp/ffmpeg.tar.xz -C "$BUILD_DIR/ffmpeg" \
      --strip-components=1 --wildcards \
      '*/bin/ffmpeg' \
      '*/bin/ffprobe' \
      '*/lib/*'; \
    \
    tar -xJf /tmp/ffmpeg.tar.xz -C "$BUILD_DIR/ffmpeg" \
      --strip-components=1 --wildcards \
      '*/bin/ffplay' || true; \
    \
    rm -f /tmp/ffmpeg.tar.xz; \
    chmod 755 "$BUILD_DIR"/danser* "$BUILD_DIR/ffmpeg/bin/"* 2>/dev/null || true

RUN set -eux; \
    BUILD_DIR=/tmp/danser-build-linux; \
    mkdir -p /out/danser; \
    cp -a "$BUILD_DIR/." /out/danser/; \
    mkdir -p /out/danser/{settings,Songs,Skins,Replays,videos}; \
    rm -rf "$BUILD_DIR"


FROM rust:1.92.0-bookworm@sha256:e90e846de4124376164ddfbaab4b0774c7bdeef5e738866295e5a90a34a307a2 AS oscbot-builder
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

RUN --mount=type=cache,id=apt-cache-final,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,id=apt-lists-final,target=/var/lib/apt/lists,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
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

COPY --link --from=danser-builder --chown=1000:1000 --chmod=755 /out/danser /app/danser
COPY --link --from=oscbot-builder  --chown=1000:1000 --chmod=755 /out/oscbot /app/oscbot/oscbot

COPY --chown=1000:1000 default-danser.json /app/danser/settings/default.json
COPY --chown=1000:1000 src/generate/data   /app/oscbot/src/generate/data

RUN printf "%s\n" /app/danser /app/danser/ffmpeg/lib >/etc/ld.so.conf.d/app-danser.conf \
 && ldconfig

ENV PATH="/app/danser/ffmpeg/bin:${PATH}"
ENV LD_LIBRARY_PATH=/usr/local/nvidia/lib:/usr/local/nvidia/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}

USER 1000:1000
WORKDIR /app/oscbot
ENTRYPOINT ["tini","--","/app/oscbot/oscbot"]
