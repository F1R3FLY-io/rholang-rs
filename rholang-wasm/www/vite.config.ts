import { defineConfig } from 'vite'

// Allow importing the generated WASM pkg from the parent directory (../pkg)
export default defineConfig({
  server: {
    fs: {
      allow: ['..']
    }
  },
  optimizeDeps: {
    // Don't try to prebundle the generated wasm JS shim
    exclude: ['rholang_wasm']
  }
})
