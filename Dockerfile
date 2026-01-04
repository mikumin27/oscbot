# Build stage
FROM rust:slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN rustup toolchain install nightly --profile minimal && \
    rustup default nightly

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

COPY src/main.rs ./src/main.rs

RUN cargo fetch

COPY src ./src

ARG BUILD_PROFILE=debug
ENV CARGO_BUILD_JOBS=1
RUN if [ "$BUILD_PROFILE" = "release" ]; then \
        cargo build --locked --release; \
    else \
        cargo build --locked; \
    fi && \
    mkdir -p /out && \
    cp "/app/target/${BUILD_PROFILE}/oscbot" /out/oscbot

# Runtime stage
FROM git.sulej.net/osc/skins-image:latest

ENV OSC_BOT_DANSER_PATH=/app/danser
ENV PATH="/app/danser:${PATH}"
WORKDIR /app/oscbot

COPY --from=builder /out/oscbot /app/oscbot/oscbot

COPY default-danser.json /app/oscbot/default-danser.json

COPY default-danser.json /app/danser/settings/default.json

COPY src/generate/data /app/oscbot/src/generate/data

RUN mkdir -p \
      /app/oscbot/Songs \
      /app/oscbot/Skins \
      /app/oscbot/Replays \
      /app/oscbot/videos \
      /app/oscbot/videoForRegen \
 && chmod +x /app/oscbot/oscbot \
 && chown -R 1000:1000 /app/oscbot /app/danser

USER 1000:1000

CMD ["/app/oscbot/oscbot"]
