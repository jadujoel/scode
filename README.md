# Scode (Sound Encoder)

## Description

This is an app that allows you to encode 48khz wav files to webm (opus) and mp4 (aac) files.

## Usage

create a folder structure like this

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


run

```bash
npm install scode
npx scode --indir=packages --outdir=encoded
# full options
npx scode --indir="<input_directory>" --outdir="<output_directory>" --bitrate="<default_bitrate_in_kbits>"--ffmpeg="<path_to_ffmpeg>" --no-include-mp4
```

Now the encoder will process all the wav files it found in the directory and its subdirectory and output the files in the output directory.
It will also create a json file with info about the files.
The output file will be named `<bitrate>k.<channels>ch.<hash>.webm|mp4`. Ex: `96kb.1ch.394510008784912090.webm`.

## info.json

The structure is

```rs
{
  game: [
    [<name> <file> <num_samples>, Optional<language>]
  ]
}
```

where name is the original filename without the extension.
file is the new filename `<bitrate>k.<channels>ch.<hash>.<ext>`

## Notes

Serviceworker gets cache time from header.
Hash 7 characters should be enough.
CI/CD doesnt want to change cache time depending on the file type.
So we need to change the hash on the file name.
