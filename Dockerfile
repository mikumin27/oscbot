FROM rustlang/rust:nightly-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    build-essential \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN USER=root cargo new --bin oscbot
WORKDIR /app/oscbot

COPY Cargo.toml Cargo.lock ./

RUN cargo build --release

RUN rm src/*.rs

COPY src ./src

RUN cargo build --release \
 && mkdir -p /out \
 && cp target/release/oscbot /out/oscbot

FROM git.sulej.net/osc/skins-image:latest

ENV OSC_BOT_DANSER_PATH=/app/danser
ENV PATH="/app/danser:${PATH}"

WORKDIR /app/oscbot

COPY --from=builder /out/oscbot /app/oscbot/oscbot
COPY default-danser.json /app/danser/settings/default.json
COPY src/generate/data /app/oscbot/src/generate/data

RUN mkdir -p \
      /app/oscbot/Songs \
      /app/oscbot/Skins \
      /app/oscbot/Replays \
      /app/oscbot/videos \
 && chmod +x /app/oscbot/oscbot \
 && chown -R 1000:1000 /app/

USER 1000:1000

CMD ["/app/oscbot/oscbot"]
