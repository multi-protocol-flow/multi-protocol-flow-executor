import '@testing-library/jest-dom'
import { vi } from 'vitest'

// Mock CSS imports
vi.mock('@xyflow/react/dist/style.css', () => ({}))
vi.mock('*.css', () => ({}))

// Polyfill ResizeObserver for jsdom
global.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
} as any
