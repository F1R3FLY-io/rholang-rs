import React from 'react'
import { describe, it, expect } from 'vitest'
import { render, screen, waitFor, fireEvent } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

import App from './App'

describe('App integration with real WASM bindings', () => {
  it('initializes wasm, runs code, and renders result plus disassembly', { timeout: 20000 }, async () => {
    render(<App />)

    const runBtn = await screen.findByRole('button', { name: /run/i })
    await waitFor(() => expect(runBtn).toBeEnabled())

    const textarea = screen.getByRole('textbox') as HTMLTextAreaElement
    fireEvent.change(textarea, { target: { value: '2 + 2' } })

    await userEvent.click(runBtn)

    // Result should reflect actual evaluation (not stubbed/echoed) and be non-empty
    const resultHeading = screen.getByRole('heading', { name: /result/i })
    const resultPanel = resultHeading.nextElementSibling as HTMLElement
    await waitFor(() => {
      const text = (resultPanel?.textContent ?? '').trim()
      expect(text.length).toBeGreaterThan(0)
      expect(text).not.toMatch(/StubEval:|Echo:/i)
    })

    // Disassembly should be populated and come from real compiler output
    const disasmHeading = screen.getByRole('heading', { name: /disassembly/i })
    const disasmPanel = disasmHeading.nextElementSibling as HTMLElement
    await waitFor(() => {
      const text = (disasmPanel?.textContent ?? '').trim()
      expect(text.length).toBeGreaterThan(0)
    })
    const disasmText = (disasmPanel?.textContent ?? '')
    expect(disasmText).not.toMatch(/StubDisasm|EchoDisasm/i)
  })
})
