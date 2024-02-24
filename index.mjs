#!/usr/bin/env node

import { path } from 'ffmpeg-helper'
import { spawn } from 'child_process'

export function binary() {
  const platform = process.platform
  const arch = process.arch
  if (platform === 'win32') {
    if (arch === 'ia32') return "binaries/win_x86/scode.exe"
    if (arch === 'x64') return "binaries/win_x86_64/scode.exe"
  }
  if (platform === 'linux') {
    if (arch === 'x64') return "binaries/linux_x86_64/scode"
    if (arch === 'arm') return "binaries/linux_arm/scode"
  }
  if (platform === 'darwin') {
    if (arch === 'x64') return "binaries/macos_x86_64/scode.app/Contents/MacOS/scode"
    if (arch === 'arm64') return "binaries/macos_arm64/scode.app/Contents/MacOS/scode"
  }
  return "not_found"
}

/**
 * @param {string[]} args
 */
export function encode(args) {
  const child = spawn('./target/release/scode', [...args, `--ffmpeg=${path}`, '--yes=true'])
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

if (import.meta?.url === `file://${process.argv[1]}`) {
  console.log(...process.argv.slice(2))
  encode(process.argv.slice(2))
  console.log(binary())
}
