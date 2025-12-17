import { describe, expect, it, vi } from 'vitest'
import React from 'react'
import { render, screen, waitFor, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

// Mock the wasm-bindgen JS glue the same way it is imported in App.tsx
// App imports: `import init, { WasmInterpreter } from '../../pkg/rholang_wasm.js'`
vi.mock('../../pkg/rholang_wasm.js', () => {
  let initialized = false

  const init = vi.fn(async () => {
    // simulate async initialization delay
    await new Promise((r) => setTimeout(r, 0))
    initialized = true
  })

  class WasmInterpreter {
    async interpret(code: string): Promise<string> {
      if (!initialized) throw new Error('WASM not initialized')
      // Simulate the stubbed wasm behavior
      return `StubEval: ${code}`
    }

    async disassemble(code: string): Promise<string> {
      if (!initialized) throw new Error('WASM not initialized')
      return `StubDisasm: ${code}`
    }
  }

  return { default: init, WasmInterpreter }
})

import App from './App'

describe('App WASM initialization ordering', () => {
  it('does not construct interpreter before init, reaches ready state, and runs code', async () => {
    render(<App />)

    const runBtn = await screen.findByRole('button', { name: /run/i })
    // Should be enabled once init completed and interpreter created
    await waitFor(() => expect(runBtn).toBeEnabled())

    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement
    fireEvent.change(textarea, { target: { value: 'new x in { x!(1) }' } })

    await userEvent.click(runBtn)

    // Result panel should contain our mocked evaluation output
    await screen.findByText(/StubEval: new x in \{ x!\(1\) \}/i)
  })
})
