import { spawn } from 'child_process'
import { resolve } from 'path';
import { path as ffmpeg } from 'ffmpeg-helper'

export function scode() {
    return resolve_this_dir(relative())
    function relative() {
        if (process.argv.includes("--dev")) {
            return "target/release/scode"
        }
        const platform = process.platform
        const arm = process.arch.includes('arm')
        if (platform === 'win32') {
            return arm
                ? "artifacts/windows_arm/scode.exe"
                : "artifacts/windows_x86/scode.exe"
        }
        if (platform === 'linux') {
            return arm
                ? "artifacts/ubuntu_arm/scode"
                : "artifacts/ubuntu_x86/scode"
        }
        if (platform === 'darwin') {
            return arm
                ? "artifacts/macos_arm/scode"
                : "artifacts/macos_x86/scode"
        }
        return "unsupported platform/architecture"
    }
}

export function ffmpega() {
    return resolve_this_dir(relative())
    function relative() {
        const platform = process.platform
        const arch = process.arch
        const arm = arch.includes('arm')
        if (platform === 'win32') {
            if (arch === 'ia32') return "node_modules/ffmpeg-helper/win-ia32.exe"
            if (arch === 'x64') return "node_modules/ffmpeg-helper/win-x64.exe"
        }
        if (platform === 'linux') {
            return arm
                ? "node_modules/ffmpeg-helper/linux-arm64"
                : "node_modules/ffmpeg-helper/linux-x64"
        }
        if (platform === 'darwin') {
            return arm
                ? "node_modules/ffmpeg-helper/ffmpeg-darwin-arm64.app/Contents/MacOS/ffmpeg-darwin-arm64"
                : "node_modules/ffmpeg-helper/ffmpeg-darwin-x64.app/Contents/MacOS/ffmpeg-darwin-x64"
        }
        return "unsupported platform/architecture"
    }
}

function resolve_this_dir(file = '') {
    return resolve(import.meta.url.replace('file://', '').replace('lib.mjs', ''), file)
}

/**
 * @param {string[]} args
 */
export function encode(args) {
    // remove --dev so scode doesnt throw
    let nargs = [...args].filter(v => v !== "--dev")
    const child = spawn(scode(), [...nargs, `--ffmpeg=${ffmpeg}`, '--yes=true'])
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
