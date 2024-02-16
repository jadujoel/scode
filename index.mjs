import { path } from 'ffmpeg-helper'
import { spawn } from 'child_process'

/**
 * @param {string[]} args
 */
export function encode(args) {
  const child = spawn('./target/release/scode', [...args, `--ffmpeg=${path}`, '-y'])
  child.stdout.on('data', (data) => {
    process.stdout.write(data.toString());
  })
  child.stderr.on('data', (data) => {
    process.stderr.write(data.toString());
  });
  child.on('error', (err) => {
    console.error('\nFailed to start subprocess.', err);
  });
}
