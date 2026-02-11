// Vitest setup file
import { expect, afterEach } from 'vitest'
import { cleanup } from '@testing-library/react'
import * as matchers from '@testing-library/jest-dom/matchers'
import fs from 'fs'
import { fileURLToPath } from 'url'
import path from 'path'

// extends Vitest's expect with jest-dom's matchers
expect.extend(matchers)

// cleanup JSDOM between tests
afterEach(() => {
  cleanup()
})

// Enable fetch for file:// URLs so wasm-bindgen can load the local .wasm in tests
const nativeFetch = globalThis.fetch
if (nativeFetch) {
  globalThis.fetch = (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const tryServeLocalWasm = (url: URL): Response | null => {
      if (url.pathname.endsWith('rholang_wasm_bg.wasm')) {
        const wasmPath = path.resolve(
          path.dirname(fileURLToPath(import.meta.url)),
          '..',
          '..',
          'pkg',
          'rholang_wasm_bg.wasm'
        )
        const data = fs.readFileSync(wasmPath)
        return new Response(data, {
          status: 200,
          headers: { 'Content-Type': 'application/wasm' },
        })
      }
      return null
    }

    try {
      const url = typeof input === 'string' ? new URL(input, 'file://') : new URL(input.toString())
      const maybeWasm = tryServeLocalWasm(url)
      if (maybeWasm) return Promise.resolve(maybeWasm)

      if (url.protocol === 'file:') {
        const filename = fileURLToPath(url)
        const data = fs.readFileSync(filename)
        return Promise.resolve(new Response(data, { status: 200 }))
      }
      if (url.protocol.startsWith('http') && url.pathname.endsWith('rholang_wasm_bg.wasm')) {
        const maybe = tryServeLocalWasm(new URL('file://' + url.pathname))
        if (maybe) return Promise.resolve(maybe)
      }
    } catch (_) {
      // fall through to native fetch
    }
    return nativeFetch(input as any, init)
  }
}
