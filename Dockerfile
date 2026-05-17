# syntax=docker/dockerfile:1.7
ARG CUDA_VER=12.8.0
ARG UBUNTU_VER=24.04

FROM nvidia/cuda:${CUDA_VER}-runtime-ubuntu${UBUNTU_VER}@sha256:44e43f0e0bcca1fc6fdc775e6002c67834bf78d39eb1fd76825240fc79ba4a49 AS final
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates tini gosu \
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

RUN install -d -m 755 \
      /app/danser /app/oscbot \
      /app/danser/Replays /app/danser/Songs /app/danser/Skins /app/danser/videos

COPY --link --chmod=755 danser-out          /app/danser
COPY --link --chmod=755 oscbot              /app/oscbot/oscbot
COPY --chmod=755        docker/entrypoint.sh /app/oscbot/entrypoint.sh

COPY default-danser.json /app/danser/settings/default.json
COPY src/generate/data   /app/oscbot/src/generate/data

RUN printf "%s\n" /app/danser /app/danser/ffmpeg >/etc/ld.so.conf.d/app-danser.conf \
 && ldconfig

ENV PATH="/app/danser/ffmpeg:${PATH}"
ENV LD_LIBRARY_PATH=/usr/local/nvidia/lib:/usr/local/nvidia/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}

WORKDIR /app/oscbot
ENTRYPOINT ["tini","--","/app/oscbot/entrypoint.sh"]
