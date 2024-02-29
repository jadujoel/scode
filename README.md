# Scode (Sound Encoder)

## Description

This app is tailor made for those who are working with sound files monorepo environment and need to encode a large number of sound files to a specific format. It's opinionated and enforces a specific folder structure as well as 48kHz PCM original wav files.
It only works with the source files being `.wav`.

It will create an .atlas.json file with the original file names and the new file names.
All of the output sounds will end up in the same directory with unique names based on bitrate, number of channels and a hash of the file.

The atlas file allows you to map the original package and sound file name to the new file name, so that you can load the correct sound in your app.
It also includes information about the original number of samples for each file,
since sometimes when decoding a opus/aac file the number of samples can change from the original (for example AudioContext.decodeAudioData in firefox may report the incorrect number of samples).

The app will enforce 48kHz PCM original wav files.
If something else is found it will reencode the source files.
Unless the `--yes=false` flag is used, then it will first ask if the user wants to reencode the files.

## Quick start

Install the package:

```bash
mkdir example
cd example
npm init --yes
npm install @jadujoel/scode
```

Copy the example to your project root:

```bash
cp node_modules/@jadujoel/scode/example/* .
```

Run:

```bash
npm run start
```

Or instead of above copy and run this oneliner

```bash
mkdir example && cd example && npm init --yes && npm install @jadujoel/scode && cp -R node_modules/@jadujoel/scode/example/* . && npm run start
```

And open `http://localhost:3000` in your browser.

## Quick Setup of your own project

Create a scodefig.jsonc file

```jsonc
// scodefig.jsonc
{
  "$schema": "node_modules/@jadujoel/scode/scode.schema.json",
  "indir": "packages",
  "outdir": "encoded",
  "bitrate": 32,
  "packages": {
    "template": {
    }
  }
}
```

Add some .wav sound files to the packages/template/sounds folder

Then encode by running:

```bash
npx @jadujoel/scode
```

See the `example` folder for a full integration example with a sound manager that interprets the atlas and displays a user interface where you can playe the encoded sounds, selecting packages and languages.

## Usage

### Folder Structure

You'll need to use this structure for your sounds:

- package.json
- packages
  - normal_package_name
    - sounds
      - music.wav
      - effect.wav
  - localized_package_name
    - sounds
      - _
        - music.wav
        - effect.wav
      - english
        - hello.wav
        - goodbye.wav
      - spanish
        - hello.wav
        - goodbye.wav

### Running the encoder

Now the encoder will process all the wav files it found output the files in the output directory.
It will also create a .atlas.json file with info about the files.

- structure: `<bitrate>k.<channels>ch.<hash>.webm|mp4`
- example: `96kb.1ch.394510008784912090.webm`.

### Changing bitrates

To change the bitrate for a single file you the scodefig.jsonc file.

- list the name of each sound file and the bitrate you want to use.
- name is the filename without the extension. `music.wav` becomes `music`.

Example config:

```jsonc
{
    "$schema": "node_modules/@jadujoel/scode/schema.json",
    "indir": "packages",
    "outdir": "public/encoded",
    "bitrate": 24,
    "packages": {
        "template": {
            "sourcedir": ""
        },
        "localised": {
            "sourcedir": "sounds",
            "languages": {
                "_": "_",
                "english": "en",
                "spanish": "es",
                "swedish": "sv"
            },
            "sources": {
                "effect_riser": {
                    "bitrate": 24
                },
                "effect_spin": {
                    "bitrate": 24
                },
                "voice_banker": {
                    "bitrate": 16,
                    "channels": 1
                },
                "voice_bets": {
                    "bitrate": 16,
                    "channels": 1
                },
                "voice_player": {
                    "bitrate": 16,
                    "channels": 1
                }
            }
        }
    }
}
```

- Any non listed files will use the default bitrate.
- using bitrate `32` will result in a file with a bitrate of 32kbits per channel.
- bitrate `32` and channels 1 will result in a file with a bitrate of `32kbits` and `1` channel.
- bitrate `32` and channels `2` will result in a file with a total bitrate of `64kbits`.

### Using languages

To use different languages you update the scodefig.jsonc file.
To use different languages you need to add a `languages` object to the package.
The `_` represents `no language`.
Otherwise you mapp tha language name to the folder to look for the files in.

## .atlas.json

The generated structure is as below. Where name is the original filename without the extension.

```json
{
  "package_a": [
    ["<name>" "<filename>" "<num_samples>", "<language>"],
    ["<name>" "<filename>" "<num_samples>", "<language>"]
  ],
}
```

File is the new filename `<bitrate>k.<channels>ch.<hash>.<ext>`

## Full Options

Get the full list of cli commands by running:

```bash
npx @jadujoel/scode --help
```

Most important is the `--indir`, `--packages` and `--loglevel` flags.

```bash
npx scode --indir="../sounds-repo" --packages="pkga" --packages="pkgb" --include-mp4=false --loglevel=perf
```

Above would look for the config file in the `sounds-repo` directory and encode the `pkga` and `pkgb` packages, ignoring the other packages listed in the `scodefig.jsonc` file. It will skip generating mp4 files and only generate webm files. It will also log the time it took for each part of the program.

- loglevels: `debug`, `perf`, `info`, `success` `warn`, `error`, `silent`

## Development

### Running the app

```bash
cargo run --release -- --indir=../sounds --loglevel=perf --packages=common --packages=localisationprototype --use-cache=false
```
