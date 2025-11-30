// Vitest setup file
import { expect, afterEach } from 'vitest'
import { cleanup } from '@testing-library/react'
import * as matchers from '@testing-library/jest-dom/matchers'

// extends Vitest's expect with jest-dom's matchers
expect.extend(matchers)

// cleanup JSDOM between tests
afterEach(() => {
  cleanup()
})
