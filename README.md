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
- example: `96kb.1ch.394510008784912.webm`.

The hash is set with `"hash"` in scodefig.jsonc or with `--hash`:

- `"siphash"` (default): first 15 decimal digits of the siphash of the file, e.g. `96kb.1ch.394510008784912.webm`.
- `"sha256"`: first 10 hex characters of the sha256 of the file, git lfs compatible, e.g. `96kb.1ch.9fe3bc1b7d.webm`.

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

### Mp4 for some files only

`--include-mp4=true` makes an mp4 file for every source. To make mp4 files for some sources only, set `include_mp4` on the package or on a source. A source setting overrides the package setting.

```jsonc
{
    "packages": {
        "template": {
            "sources": {
                "music_loop": {
                    "include_mp4": true
                }
            }
        }
    }
}
```

Here only `music_loop` gets an mp4 file next to its webm file.

### Using languages

To use different languages you update the scodefig.jsonc file.
To use different languages you need to add a `languages` object to the package.
The `_` represents `no language`.
Otherwise you mapp tha language name to the folder to look for the files in.

### Extra ffmpeg flags

Builds are deterministic: the same source and settings give byte identical output files and `.atlas.json`.
To get that, every output is encoded with these flags by default:

- `-fflags +bitexact -flags:a +bitexact`: no ffmpeg version, timestamps or random ids in the file.
- `-map 0:a:0`: only the first audio stream of the source.
- `-map_metadata -1 -map_chapters -1 -metadata:s:a encoder=`: no metadata, chapters or encoder tag.

The output can still differ between ffmpeg versions, since the encoders themselves change, so pin the ffmpeg version if you build on several machines.

To pass extra flags to ffmpeg for a specific output format, add an `ffmpeg_flags` object to the scodefig.jsonc file.
The keys are the extensions `webm`, `opus`, `mp4` and `flac`.
The value is either a whitespace separated string, or an array where each entry is passed as one argument (use the array form when an argument contains spaces).
The flags are placed right before the output file, so they apply to that output only.

```jsonc
{
  "ffmpeg_flags": {
    "webm": "-application voip",
    "mp4": ["-cutoff", "18000"]
  }
}
```

The flags come after the defaults, so you can override them, e.g. `"webm": "-fflags -bitexact -flags:a -bitexact"` turns bitexact off for webm.

Existing output files are not re-encoded when the flags change, delete them from the output directory to re-encode.

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

### Publishing a new version

Releases are built and published by GitHub Actions. There is no single deploy button; the flow is:

1. **Push to `master`.** If anything in `src/**` changed, the `artifacts` workflow builds and signs the binaries for all platforms and commits them to the `artifacts` branch.
2. **Wait for `artifacts` to finish.** `publish` packages whatever is on the `artifacts` branch, so starting a release too early ships the old binaries. If you changed only JS files, `artifacts` does not run, and that is fine.
3. **Run the `version` workflow** (Actions → `version` → Run workflow) and pick a release type:
   - `alpha` (default): `1.9.6` → `1.9.7-alpha.0`, published under the `alpha` npm tag
   - `patch`, `minor`, `major`: published under the `latest` npm tag

   This runs `npm version` and pushes the bump commit and tag to `master`.
4. **`publish` runs automatically** when `version` succeeds. It copies the binaries from the `artifacts` branch and runs `npm publish`.

If a publish fails, fix the cause and run `publish` manually (Actions → `publish` → Run workflow). It publishes the version currently in `package.json` on `master`, so do not run `version` again.

#### npm authentication

`publish` supports two ways to authenticate with npm:

- **Trusted publishing (recommended):** on npmjs.com, open the package settings for `@jadujoel/scode` and add a trusted publisher for this GitHub repository with the workflow file `publish.yml`. No token is needed.
- **Token:** set the `NPM_TOKEN` repository secret to an npm automation or granular access token with publish rights for `@jadujoel/scode`.

A `404 Not Found - PUT https://registry.npmjs.org/@jadujoel%2fscode` error from `npm publish` means the request was not authenticated. Check both options above.
