import React, { useEffect, useState } from 'react'

// Import wasm init and bindings from the generated pkg one level up
import init, { WasmInterpreter } from '../../pkg/rholang_wasm.js'

export default function App() {
  const [status, setStatus] = useState<'idle'|'loading'|'ready'|'running'|'error'>('idle')
  const [code, setCode] = useState<string>(`new x in {
  x!(42)
}`)
  const [output, setOutput] = useState<string>('')
  const [interpreter, setInterpreter] = useState<any | null>(null)

  useEffect(() => {
    let mounted = true
    setStatus('loading')
    init().then(() => {
      if (!mounted) return
      // Instantiate interpreter only after WASM init completes
      const interp = new WasmInterpreter()
      setInterpreter(interp)
      setStatus('ready')
    }).catch(err => {
      console.error(err)
      if (mounted) setStatus('error')
    })
    return () => { mounted = false }
  }, [])

  const run = async () => {
    setStatus('running')
    try {
      if (!interpreter) throw new Error('Interpreter not ready')
      const res = await interpreter.interpret(code)
      setOutput(res)
      setStatus('ready')
    } catch (e: any) {
      setOutput('Error: ' + (e?.message ?? String(e)))
      setStatus('error')
    }
  }

  return (
    <div style={{ fontFamily: 'system-ui, -apple-system, Segoe UI, Roboto, sans-serif', margin: '2rem' }}>
      <h1>Rholang WASM (React)</h1>
      <p>Status: <strong>{status}</strong></p>
      <div style={{ display: 'grid', gap: '1rem' }}>
        <label htmlFor="code">Rholang code</label>
        <textarea
          id="code"
          value={code}
          onChange={e => setCode(e.target.value)}
          placeholder="// Type Rholang here"
          style={{ width: '100%', height: '220px', fontFamily: 'monospace' }}
        />
        <div>
          <button onClick={run} disabled={status !== 'ready' || !interpreter}>Run</button>
        </div>
        <div style={{ whiteSpace: 'pre-wrap', background: '#111', color: '#0f0', padding: '1rem', borderRadius: 6, minHeight: 120 }}>
          {output}
        </div>
      </div>
    </div>
  )
}
