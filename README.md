# oscbot

Discord bot for the osu! Swiss community.

It provides a “suggest → approve/decline → (optional) render + upload” workflow:

- Users submit scores/replays via `/suggest score`.
- Staff approve/decline via message buttons in a configured request channel.
- If approved “with upload” (osu!standard only), the bot renders the replay with danser and uploads it to YouTube.
- A background task polls the channel’s YouTube RSS feed and posts new uploads to a Discord channel.

## Commands

All commands are Discord slash commands.

### Suggest

- `/suggest score` (either `scoreid` or `scorefile`, optional `reason`)
  - Posts a request into the configured request channel with approve/decline buttons.
  - Prevents duplicate requests via Firebase.

### Replay (requires role)

`/replay generate` provides:

- `/replay generate thumbnail` (either `scoreid` or `scorefile`, optional `subtitle`)
- `/replay generate title_and_description` (either `scoreid` or `scorefile`)
- `/replay generate render_and_upload` (either `scoreid` or `scorefile`, optional `subtitle`)

Notes:

- Rendering/upload is currently only supported for osu!standard.
- For score IDs, the score must have a downloadable replay (`score.has_replay`).

### Skin

- `/skin set url:<link>`
  - Stores a skin URL for your osu! account (based on your Discord nickname/username matching osu username).
  - Download URL must be from `https://git.sulej.net/` and end in `.osk`.
- `/skin get [member]` (requires role)
  - Returns the saved skin URL for a member (or yourself).

### Admin (requires role)

- `/admin blacklist add <member>`
- `/admin blacklist remove <member>`
- `/admin blacklist list`

Blacklisted users are blocked from using commands by a global check.

### Dev (debug builds only)

Only compiled in debug builds (`cargo run`). Not present in release builds / container image.

- `/dev test_osu_client`
- `/dev test_thumbnail`
- `/dev test_danser_and_youtube <scorefile>`
- `/dev test_upload`
- `/dev regenerate_token`

## Configuration

The bot loads environment variables from `.env` (via `dotenvy`) and from the process environment.

### Required environment variables

```bash
# Discord
OSC_BOT_DISCORD_TOKEN=
OSC_BOT_DISCORD_SERVER=            # guild id (u64)
OSC_BOT_REPLAY_ADMIN_ROLE=         # role id (u64)
OSC_BOT_REQUEST_CHANNEL=           # channel id (u64) used by /suggest
OSC_BOT_NEW_VIDEOS_CHANNEL=        # channel id (u64) for "new upload" notifications

# osu! OAuth (rosu-v2)
OSC_BOT_CLIENT_ID=                 # osu! OAuth client id (u64)
OSC_BOT_CLIENT_SECRET=

# Firebase Realtime Database
OSC_BOT_FIREBASE_PROJECT_URL=      # e.g. https://<project-id>-default-rtdb.firebaseio.com/
OSC_BOT_FIREBASE_AUTH_KEY=

# danser
OSC_BOT_DANSER_PATH=               # directory containing Songs/, Skins/, Replays/, videos/

# YouTube feed polling
OSC_BOT_YOUTUBE_CHANNEL_ID=
```

### Optional environment variables

```bash
# Defaults to "danser-cli" (must be on PATH). In Docker, set this explicitly.
OSC_BOT_DANSER_CLI=/app/danser/danser-cli

# Stream danser stdout/stderr into logs (default: true)
OSC_BOT_DANSER_LOG=true
```

### Required files

#### YouTube (uploads)

The upload code expects these files in the working directory (same folder as the `oscbot` binary):

- `youtube_secret.json` (Google OAuth client secret for an “Installed/Desktop App” client)
- `token.json` (OAuth token cache, created after the first successful auth)

#### danser

danser expects a credentials file in its settings folder:

- `/app/danser/settings/credentials.json` (inside the container)

With the provided `docker-compose.yaml`, you supply this as:

- `./credentials.json` (next to `docker-compose.yaml`), mounted to `/app/danser/settings/credentials.json`

You must generate this using danser (run danser once and complete its setup/auth flow so it writes `credentials.json`).

## Running with Docker (recommended)

This repo ships a container setup that builds danser-go + ffmpeg and runs the bot on an NVIDIA CUDA runtime.

Prerequisites:

- Docker + Docker Compose
- NVIDIA driver + `nvidia-container-toolkit`

### Files/folders next to docker-compose

Create this layout:

```text
.
├─ docker-compose.yaml
├─ .env
├─ youtube_secret.json
├─ token.json
├─ credentials.json
└─ danser/
   ├─ Songs/
   ├─ Skins/
   ├─ Replays/
   └─ videos/
```

Then run:

```bash
docker compose up -d
```

Important Docker notes:

- Set `OSC_BOT_DANSER_PATH=/app/danser` and `OSC_BOT_DANSER_CLI=/app/danser/danser-cli` in your `.env`.
- The container runs as `1000:1000` (see `docker-compose.yaml`). Make sure mounted folders are writable.

## Running locally (development)

You can run directly with Rust:

```bash
cargo run
```

For an optimized build:

```bash
cargo run --release
```

## YouTube OAuth (token.json)

Uploads use the installed-app OAuth flow and persist tokens to `token.json` in the working directory.

Practical setup for headless servers:

1. Run the bot locally once (where you have a browser) with `youtube_secret.json` present.
2. Trigger an upload once (any render/upload path will do) to complete OAuth.
3. Copy the resulting `token.json` to the server next to the bot binary (or mount it into the container).

## Logging

Logging is controlled via `RUST_LOG` (defaults to `info`). Example:

```bash
RUST_LOG=info
```