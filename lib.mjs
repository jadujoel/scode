import { spawn } from 'child_process'
import { resolve } from 'path';

export function scode() {
    return resolve_this_dir(relative())
    function relative() {
        if (process.argv.includes("--dev")) {
            return "target/release/scode"
        }
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
            // if (arch === 'arm64') return "binaries/macos_arm64/scode.app/Contents/MacOS/scode"
            if (arch === 'arm64') return "binaries/macos_arm64/scode.app/Contents/MacOS/scode"
        }
        return "not_found"
    }
}

export function ffmpeg() {
    return resolve_this_dir(relative())
    function relative() {
        const platform = process.platform
        const arch = process.arch
        if (platform === 'win32') {
            if (arch === 'ia32') return "node_modules/ffmpeg-helper/win-ia32.exe"
            if (arch === 'x64') return "node_modules/ffmpeg-helper/win-x64.exe"
        }
        if (platform === 'linux') {
            if (arch === 'x64') return "node_modules/ffmpeg-helper/linux-x64"
            if (arch === 'arm') return "node_modules/ffmpeg-helper/linux-arm64"
        }
        if (platform === 'darwin') {
            if (arch === 'x64') return "node_modules/ffmpeg-helper/ffmpeg-darwin-x64.app/Contents/MacOS/ffmpeg-darwin-x64"
            if (arch === 'arm64') return "node_modules/ffmpeg-helper/ffmpeg-darwin-arm64.app/Contents/MacOS/ffmpeg-darwin-arm64"
        }
        return "not_found"
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
    const child = spawn(scode(), [...nargs, `--ffmpeg=${ffmpeg()}`, '--yes=true'])
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
