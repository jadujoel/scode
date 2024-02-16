# Scode (Sound Encoder)

## Description

This is an app that allows you to encode 48khz wav files to webm (opus) and mp4 (aac) files.

## Quick start

```bash
npm install scode
npx scode --indir=packages --outdir=encoded
```

## Usage

### Folder Structure

use this structure for your sounds:

- package.json
- packages
  - normal_package_name
    - sounds
      - .bitrates
      - music.wav
      - effect.wav
  - localized_package_name
    - sounds
      - .bitrates
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
npx scode --indir="<input_directory>" --outdir="<output_directory>" --bitrate="<default_bitrate_in_kbits>"--ffmpeg="<path_to_ffmpeg>" --no-include-mp4
```

## Notes

Serviceworker gets cache time from header.
Hash 7 characters should be enough.
CI/CD doesnt want to change cache time depending on the file type.
So we need to change the hash on the file name.
