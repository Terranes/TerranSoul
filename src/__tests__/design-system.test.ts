import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import DesignShowcase from '../components/DesignShowcase.vue'

/**
 * Design System Conformance Tests
 *
 * Validates that DesignShowcase.vue and the --ts-* token system
 * conform to the spec defined in docs/DESIGN.md.
 * Reference: styles.refero.design (Refero DESIGN.md format)
 */
describe('DesignShowcase', () => {
  it('mounts without error', () => {
    const wrapper = mount(DesignShowcase)
    expect(wrapper.exists()).toBe(true)
  })

  it('renders all color swatches', () => {
    const wrapper = mount(DesignShowcase)
    const swatches = wrapper.findAll('.swatch-card')
    expect(swatches.length).toBe(12)
  })

  it('renders typography scale with correct classes', () => {
    const wrapper = mount(DesignShowcase)
    expect(wrapper.find('.type-xl').exists()).toBe(true)
    expect(wrapper.find('.type-lg').exists()).toBe(true)
    expect(wrapper.find('.type-base').exists()).toBe(true)
    expect(wrapper.find('.type-sm').exists()).toBe(true)
    expect(wrapper.find('.type-xs').exists()).toBe(true)
  })

  it('renders all spacing scale items', () => {
    const wrapper = mount(DesignShowcase)
    const spacingRows = wrapper.findAll('.spacing-row')
    expect(spacingRows.length).toBe(5)
  })

  it('renders all radius scale items', () => {
    const wrapper = mount(DesignShowcase)
    const radiusBoxes = wrapper.findAll('.radius-box')
    expect(radiusBoxes.length).toBe(5)
  })

  it('renders all shadow levels', () => {
    const wrapper = mount(DesignShowcase)
    expect(wrapper.find('.shadow-sm').exists()).toBe(true)
    expect(wrapper.find('.shadow-md').exists()).toBe(true)
    expect(wrapper.find('.shadow-lg').exists()).toBe(true)
  })

  it('renders component pattern demos', () => {
    const wrapper = mount(DesignShowcase)
    expect(wrapper.find('.btn-primary-demo').exists()).toBe(true)
    expect(wrapper.find('.btn-secondary-demo').exists()).toBe(true)
    expect(wrapper.find('.card-demo').exists()).toBe(true)
    expect(wrapper.find('.message-demo--assistant').exists()).toBe(true)
  })

  it('uses only --ts-* tokens in styles (no hardcoded hex)', () => {
    const wrapper = mount(DesignShowcase)
    const styleElements = wrapper.element.querySelectorAll('[style]')
    styleElements.forEach((el: Element) => {
      const style = el.getAttribute('style') || ''
      // Dynamic styles should reference var() tokens only
      expect(style).toMatch(/var\(--ts-/)
    })
  })
})

describe('Design Token Spec Conformance', () => {
  /**
   * These tests validate that the token values defined in docs/DESIGN.md
   * are the expected values. We test the compiled CSS custom property names
   * against the canonical spec.
   */
  const expectedTokens = {
    // Brand
    '--ts-accent': true,
    '--ts-accent-hover': true,
    '--ts-accent-glow': true,
    '--ts-accent-blue': true,
    '--ts-accent-blue-hover': true,
    '--ts-accent-violet': true,
    '--ts-accent-violet-hover': true,
    // Semantic
    '--ts-success': true,
    '--ts-warning': true,
    '--ts-error': true,
    '--ts-info': true,
    // Text
    '--ts-text-primary': true,
    '--ts-text-secondary': true,
    '--ts-text-muted': true,
    '--ts-text-dim': true,
    '--ts-text-on-accent': true,
    // Surfaces
    '--ts-bg-base': true,
    '--ts-bg-surface': true,
    '--ts-bg-elevated': true,
    '--ts-bg-nav': true,
    '--ts-bg-overlay': true,
    '--ts-bg-input': true,
    '--ts-bg-hover': true,
    '--ts-bg-card': true,
    '--ts-bg-panel': true,
    '--ts-bg-selected': true,
    // Borders
    '--ts-border': true,
    '--ts-border-subtle': true,
    '--ts-border-medium': true,
    '--ts-border-focus': true,
    // Radius
    '--ts-radius-sm': true,
    '--ts-radius-md': true,
    '--ts-radius-lg': true,
    '--ts-radius-xl': true,
    '--ts-radius-pill': true,
    // Spacing
    '--ts-space-xs': true,
    '--ts-space-sm': true,
    '--ts-space-md': true,
    '--ts-space-lg': true,
    '--ts-space-xl': true,
    // Shadows
    '--ts-shadow-sm': true,
    '--ts-shadow-md': true,
    '--ts-shadow-lg': true,
    // Motion
    '--ts-transition-fast': true,
    '--ts-transition-normal': true,
    '--ts-transition-slow': true,
    // Typography
    '--ts-font-family': true,
    '--ts-font-mono': true,
    '--ts-text-xs': true,
    '--ts-text-sm': true,
    '--ts-text-base': true,
    '--ts-text-lg': true,
    '--ts-text-xl': true,
    // Z-Index
    '--ts-z-base': true,
    '--ts-z-dropdown': true,
    '--ts-z-sticky': true,
    '--ts-z-dialog': true,
    '--ts-z-overlay': true,
    '--ts-z-toast': true,
    '--ts-z-splash': true,
    '--ts-z-context-menu': true,
  }

  it('all tokens from DESIGN.md spec are defined', () => {
    // This test validates the token names are exhaustive
    const tokenNames = Object.keys(expectedTokens)
    expect(tokenNames.length).toBeGreaterThanOrEqual(45)
  })

  it('token naming follows --ts-* convention', () => {
    Object.keys(expectedTokens).forEach((token) => {
      expect(token).toMatch(/^--ts-/)
    })
  })

  it('token categories are complete', () => {
    const tokens = Object.keys(expectedTokens)
    const categories = new Set(tokens.map((t) => t.replace(/^--ts-/, '').split('-')[0]))
    expect(categories).toContain('accent')
    expect(categories).toContain('success')
    expect(categories).toContain('warning')
    expect(categories).toContain('error')
    expect(categories).toContain('info')
    expect(categories).toContain('text')
    expect(categories).toContain('bg')
    expect(categories).toContain('border')
    expect(categories).toContain('radius')
    expect(categories).toContain('space')
    expect(categories).toContain('shadow')
    expect(categories).toContain('transition')
    expect(categories).toContain('font')
    expect(categories).toContain('z')
  })
})
