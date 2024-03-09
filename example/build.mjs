import { build } from 'esbuild'

build({
    entryPoints: ['src/index.ts'],
    outdir: 'public',
    bundle: false,
    minify: false,
    sourcemap: true,
    treeShaking: true,
})
