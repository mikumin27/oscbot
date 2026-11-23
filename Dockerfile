# syntax=docker/dockerfile:1.7
FROM alpine:3.22@sha256:49340b346798875b5592e5d420241f2f9cfba5f47e62ebb1446a1b34734e898f AS danser
ADD --checksum=sha256:b3aa87e9cd2b8697d643c4834dd9b671682cb9800843733122e4dce1ad3794d0 \
    https://github.com/Wieku/danser-go/releases/download/0.12.1/danser-0.12.1-linux.zip /danser.zip
RUN mkdir /danser && unzip -q /danser.zip -d /danser \
 && chmod 755 /danser/danser /danser/danser-cli /danser/ffmpeg/bin/*

# No CUDA userspace needed: danser renders through EGL and ffmpeg encodes with
# NVENC, both from the driver libraries the NVIDIA container toolkit injects.
FROM ubuntu:26.04@sha256:d66b366dac61b575bee2aef07e9c26e8070096fbe84a3d4d3eb0da069b5d76db AS final
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates tini gosu \
      libssl3 \
      libglvnd0 libegl1 libgles2 libgl1 libgtk-3-0 libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

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

COPY --link --from=danser /danser          /app/danser
COPY --link --chmod=755 oscbot              /app/oscbot/oscbot
COPY --chmod=755        docker/entrypoint.sh /app/oscbot/entrypoint.sh

COPY default-danser.json /app/danser/settings/default.json
COPY src/generate/data   /app/oscbot/src/generate/data

RUN printf "%s\n" /app/danser /app/danser/ffmpeg/lib >/etc/ld.so.conf.d/app-danser.conf \
 && ldconfig

# Non-root mode (`user:` in compose) runs as an arbitrary uid, so the dirs the bot
# and danser write to (sqlite db + journal, token.json, downloaded maps/skins,
# rendered videos, settings) are sticky world-writable like /tmp. Only the
# container's own process runs in here.
RUN chmod 1777 /app/oscbot /app/danser /app/danser/settings \
      /app/danser/Replays /app/danser/Songs /app/danser/Skins /app/danser/videos \
 && chmod 666 /app/danser/settings/default.json

ENV NVIDIA_DRIVER_CAPABILITIES=compute,video,graphics,utility

WORKDIR /app/oscbot
ENTRYPOINT ["tini","--","/app/oscbot/entrypoint.sh"]
