// pdf.js needs some files it fetches at runtime (CMaps for CJK text, the standard fonts PDFs
// don't embed, wasm image decoders, ICC profiles). This copies them from the app's pdfjs-dist
// into <outDir>/pdfjs/ on build and serves them from there in dev.
import { cpSync, createReadStream, existsSync, statSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, normalize, resolve } from 'node:path'

const DIRS = ['cmaps', 'standard_fonts', 'wasm', 'iccs']

export default function pdfAssets() {
  let pkgDir, outDir, base
  return {
    name: 'pdf-assets',
    configResolved(config) {
      const require = createRequire(join(config.root, 'package.json'))
      pkgDir = dirname(require.resolve('pdfjs-dist/package.json'))
      outDir = resolve(config.root, config.build.outDir)
      base = config.base
    },
    configureServer(server) {
      server.middlewares.use(`${base}pdfjs/`, (req, res, next) => {
        const file = normalize(join(pkgDir, decodeURIComponent(req.url.split('?')[0])))
        if (!file.startsWith(pkgDir) || !existsSync(file) || !statSync(file).isFile()) return next()
        if (file.endsWith('.wasm')) res.setHeader('Content-Type', 'application/wasm')
        createReadStream(file).pipe(res)
      })
    },
    writeBundle() {
      for (const dir of DIRS) cpSync(join(pkgDir, dir), join(outDir, 'pdfjs', dir), { recursive: true })
    },
  }
}
