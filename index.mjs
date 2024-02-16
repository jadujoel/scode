import { path } from 'ffmpeg-helper'
import { spawn } from 'child_process'

const child = spawn('./target/release/ecas-encoder', [process.argv.slice(2), `--ffmpeg=${path}`, '-y'])
child.stdout.on('data', (data) => {
  process.stdout.write(data.toString());
})
child.stderr.on('data', (data) => {
  process.stderr.write(data.toString());
});
child.on('error', (err) => {
  console.error('\nFailed to start subprocess.', err);
});
