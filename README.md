# oscbot
This bot is made for the osu! swiss community.

## Commands
```
/suggest score: [scoreid] [scorefile]

/skin set: [url]

/skin get: [discord user]

/replay generate thumbnail [scoreid] [scorefile] {uploader role only}

/replay generate title_and_description [scoreid] [scorefile] {uploader role only}

/replay generate render_and_upload [scoreid] [scorefile] {uploader role only}
```

## Setup
For this bot you need an osu account for the osu api, firebase for saving states, danser for rendering and google cloud console for youtube. 

### Danser-cli
Download danser and extract the zipfile. paste default-danser.json into danser/settings and rename it to default.json. Also create the folders "Skins", "Replays" and "Songs" into the danser folder. Make sure that these 3 folders are writeable for my bot. then make it accessible by path. Paste the entire path to the danser folder into the respective env variable.

### Osu client
Go to your osu account settings and scroll down to OAuth. Create a new OAuth application. name can be anything and it shouldn't need a callback URL. Then copy the Client ID and the Client secret and put them into the respective env variables.

### Firebase
Since we really don't need much from firebase I decided to use the db secret. Go to firebase and create a new project. then go to the prohect settings and then Pick service accounts. Generate a DB secret and paste it into the respective env variable. Same counts for the db url.

### Google cloud console
Create a new project. Add the youtubeV3 api into the project. Select OAuth and then Data Access. Add the "youtube.upload" scope and then select clients. Create a ew Desktop App client and download the json. name it youtube_secret.json and paste it into the oscbot project folder. Lastly you need to select "Publish app" so that the refresh token won't expire.

### Google cloud console auth token
```
/dev regenerate_token is a helpful function for this.
```

The first time it wants to upload it will print out the url for verification into the terminal. Accept it using the account you want to upload videos with. If you do not have access to a browser then run the program on a pc that has a browser and accept it there. Then paste the token.json into the oscbot folder.

### Other env values
All three should be self-explanatory.
```
OSC_BOT_DISCORD_TOKEN: auth token of the bot itself.
OSC_BOT_REPLAY_ADMIN_ROLE: Role of the people that can upload to youtube.
OSC_BOT_DISCORD_SERVER: Server that hosts the bot.
```

### Server
I use a Debian bookworm server for it but it should run fine in any modern linux server.

### How to run
Install Rust and then run:

```
cargo run --release
```
or for the dev commands:
```
cargo run
```