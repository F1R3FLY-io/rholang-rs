import React, { useEffect, useState } from 'react'

// Import wasm init and bindings from the generated pkg one level up
import init, { WasmInterpreter } from '../../pkg/rholang_wasm.js'

export default function App() {
  const [status, setStatus] = useState<'idle'|'loading'|'ready'|'running'|'error'>('idle')
  const [code, setCode] = useState<string>(`new x in {
  x!(42)
}`)
  const [output, setOutput] = useState<string>('')
  const [disassembly, setDisassembly] = useState<string>('')
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
      const [res, disasm] = await Promise.all([
        interpreter.interpret(code),
        interpreter.disassemble(code),
      ])
      setOutput(res)
      setDisassembly(disasm)
      setStatus('ready')
    } catch (e: any) {
      setOutput('Error: ' + (e?.message ?? String(e)))
      setDisassembly('')
      setStatus('error')
    }
  }

  return (
    <div style={{ fontFamily: 'system-ui, -apple-system, Segoe UI, Roboto, sans-serif', margin: '2rem' }}>
      <h1>Rholang WASM (React)</h1>
      <p>Status: <strong>{status}</strong></p>
      <div style={{ display: 'grid', gap: '1rem', gridTemplateRows: '4fr 1fr', minHeight: '80vh' }}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', alignItems: 'start' }}>
          <div style={{ display: 'grid', gap: '0.5rem', height: '100%' }}>
            <label htmlFor="code">Rholang code</label>
            <textarea
              id="code"
              value={code}
              onChange={e => setCode(e.target.value)}
              placeholder="// Type Rholang here"
              style={{ width: '100%', height: '100%', minHeight: 240, fontFamily: 'monospace' }}
            />
            <div>
              <button onClick={run} disabled={status !== 'ready' || !interpreter}>Run</button>
            </div>
          </div>
          <div style={{ display: 'grid', gridTemplateRows: 'auto 1fr', gap: '0.5rem', height: '100%' }}>
            <h3 style={{ marginTop: 0 }}>Disassembly</h3>
            <div style={{ whiteSpace: 'pre-wrap', background: '#0a0a0a', color: '#0ff', padding: '1rem', borderRadius: 6, minHeight: 240, overflow: 'auto' }}>
              {disassembly}
            </div>
          </div>
        </div>

        <div>
          <h3 style={{ marginTop: 0 }}>Result</h3>
          <div style={{ whiteSpace: 'pre-wrap', background: '#111', color: '#0f0', padding: '1rem', borderRadius: 6, minHeight: 120 }}>
            {output}
          </div>
        </div>
      </div>
    </div>
  )
}
