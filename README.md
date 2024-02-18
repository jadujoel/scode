# Scode (Sound Encoder)

## Description

This app is tailor made for those who are working with sound files monorepo environment and need to encode a large number of sound files to a specific format. It's opinionated and enforces a specific folder structure as well as 48kHz PCM original wav files.
It only works with the source files being `.wav`.

It will create an info.json file with the original file names and the new file names.
All of the output sounds will end up in the same directory with unique names based on bitrate, number of channels and a hash of the file.

The info file allows you to map the original package and sound file name to the new file name, so that you can load the correct sound in your app.
It also includes information about the original number of samples for each file,
since sometimes when decoding a opus/aac file the number of samples can change from the original (for example AudioContext.decodeAudioData in firefox may report the incorrect number of samples).

## Quick start

```bash
npm install scode
npx scode --indir=packages --outdir=encoded
```

## Usage

### Folder Structure

You'll need to use this structure for your sounds:

- package.json
- packages
  - normal_package_name
    - sounds
      - .bitrates (optional, see [Changing bitrates](#changing-bitrates))
      - music.wav
      - effect.wav
  - localized_package_name
    - sounds
      - .bitrates (optional, see [Changing bitrates](#changing-bitrates))
      - music.wav
      - effect.wav
      - english
        - .lang
        - hello.wav
        - goodbye.wav
      - spanish
        - .lang
        - hello.wav
        - goodbye.wav

### Running the encoder

```bash
npm install scode
npx scode --indir=packages --outdir=encoded
```

Now the encoder will process all the wav files it found in the directory and its subdirectory and output the files in the output directory.
It will also create a .info.json file with info about the files.

- structure: `<bitrate>k.<channels>ch.<hash>.webm|mp4`
- example: `96kb.1ch.394510008784912090.webm`.

### Changing bitrates

To change the bitrate for a single file you edit the `.bitrates` file in the sounds folder of the package.

- list the name of each sound file and the bitrate you want to use.
- name is the filename without the extension. `music.wav` becomes `music`.

For example adding to `packages/normal_package_name/sounds/.bitrates`.

```text
music 96
effect 64
```

Will encode those specific files with that bitrate in kbits.
Any non listed files will use the default bitrate.

### Using languages

To use different languages you create a folder with the language name and put the files in there. Then you add a `.language` file. See [file structure](#folder-structure) above. This will add an extra field to into the `.info.json` file that the ingesting audio library can use to select the correct file for the user.

```text
english
```

## .info.json

The structure is as below. Where name is the original filename without the extension.

```rs
{
  normal_package_name: [
    [<name> <filename> <num_samples>],
    [<name> <filename> <num_samples>]
  ],
  localised_package_name: [
    [<name> <filename> <num_samples>, Optional<language>]
  ]
  // ...
}
```

file is the new filename `<bitrate>k.<channels>ch.<hash>.<ext>`

## Full Options

```bash
npx scode --indir="packages" --outdir="encoded" --bitrate=64 --ffmpeg="<path_to_ffmpeg>" --packages="pkga,pkgb" --no-mp4 --yes
```

- `--indir` - input directory
- `--outdir` - output directory
- `--bitrate` - default bitrate
- `--ffmpeg` - path to ffmpeg
- `--packages` - comma separated list of packages to encode
- `--no-include-mp4` - do not encode mp4 files, only webm
- `--yes` - do not ask for confirmation when overwriting input files with 48khz

## Notes

Serviceworker gets cache time from header.
Hash 7 characters should be enough.
CI/CD doesnt want to change cache time depending on the file type.
So we need to change the hash on the file name.

For each package, also create a separate .package.info.json
If input files resampled, rerun the files info function automatically.

In ecas engine later when trying to play / load a sound:

- check if sound exists in the current package
- if not check if it exists in the common package
- otherwise check if it exists in any other package

in the build script: run scode, then run esbuild on the tsconfig file and check which sound files are used in the config.
Add a list of the sound files used in the config to the loadrConfig bit, so that "loadAllSounds" can be called on only the sound files that are used in the config.
Use the checklist above when defining which sound should be added when the name is the same in multiple packages.
Only add the sound files info from the info.json file that are used in the specific config, to minimize the size of the loadrConfig.

Encoding all packages
Encoding 2591 of 2591 (100%) | ETA: 0 seconds
Encoded 5182 sounds in 2m 28s 221ms

Encoding all packages
Encoding 2591 of 2591 (100%) | ETA: 0 seconds
Encoded 2591 sounds in 1m 7s 494ms

## Times

hash: 9 microseconds per file
wave data: 1.5ms per file
encoding: 1.5s per file?
