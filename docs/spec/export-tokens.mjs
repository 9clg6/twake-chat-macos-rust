// Exports the twake-mui theme (light + dark) to tokens.json.
// Usage: node export-tokens.mjs [path/to/twake-space-chat] > tokens.json
import { pathToFileURL } from 'node:url'
import { resolve } from 'node:path'
import { createRequire, register } from 'node:module'

// twake-mui's dist uses extensionless relative imports (bundler style):
// teach Node to resolve them.
register('data:text/javascript,' + encodeURIComponent(`
import { existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
export async function resolve(spec, ctx, next) {
  if (spec.startsWith('.') && !/\\.(m?js|json)$/.test(spec) && ctx.parentURL) {
    const base = new URL(spec, ctx.parentURL)
    for (const s of ['.js', '/index.js'])
      if (existsSync(fileURLToPath(base.href + s))) return next(base.href + s, ctx)
  }
  return next(spec, ctx)
}
export async function load(url, ctx, next) {
  if (url.endsWith('.json')) return next(url, { ...ctx, format: 'json', importAttributes: { type: 'json' } })
  return next(url, ctx)
}`))

const root = resolve(process.argv[2] ?? `${process.env.HOME}/Developer/twake-space-chat`)
const mui = `${root}/node_modules/@linagora/twake-mui`
const { makeTheme } = await import(pathToFileURL(`${mui}/dist/lib/makeTheme.js`))
const { radius } = await import(pathToFileURL(`${mui}/dist/lib/radius.js`))
const { supportedColors, colorMapping } = await import(pathToFileURL(`${mui}/dist/components/Avatar/helpers.js`))
const { default: palette } = await import(pathToFileURL(`${root}/node_modules/@linagora/twake-css/palette.json`), { with: { type: 'json' } })
const version = createRequire(`${mui}/`)('./package.json').version

const theme = makeTheme('light')
const dark = makeTheme('dark')

// rgba()/hex strings as MUI emits them; the macOS client parses both.
const scheme = (t, name) => ({
  palette: t.colorSchemes[name].palette,
  shadows: t.colorSchemes[name].shadows ?? t.shadows
})
const plain = v => JSON.parse(JSON.stringify(v))

// Resolve style callbacks with the theme so static values show up; theme.vars
// resolves to var(--twake-...) names, kept as-is (token references).
const resolve_ = (v, depth = 0, key = '') => {
  if (typeof v === 'function') {
    if (depth > 9) return '<fn>'
    try { return resolve_(v({ theme, ownerState: {} }), depth + 1, key) } catch { return '<fn>' }
  }
  if (v && typeof v === 'object' && v.$$typeof) return '<ReactElement>'
  if (depth > 9) return '<deep>'
  if (Array.isArray(v)) return v.map(x => resolve_(x, depth + 1, key))
  if (v && typeof v === 'object') {
    return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve_(x, depth + 1, k)]))
  }
  return v
}
const components = {}
for (const [name, c] of Object.entries(theme.components ?? {})) components[name] = resolve_(c)

// Icons: every component of @linagora/twake-icons rendered to static SVG markup.
const req = createRequire(`${root}/package.json`)
const React = req('react')
const { renderToStaticMarkup } = req('react-dom/server')
const I = await import(pathToFileURL(`${root}/node_modules/@linagora/twake-icons/dist/index.js`))
const icons = {}
for (const [n, C] of Object.entries(I)) {
  if (!/^[A-Z]/.test(n) || typeof C === 'string') continue
  try { const svg = renderToStaticMarkup(React.createElement(C)); if (svg.startsWith('<svg')) icons[n] = svg } catch {}
}

const out = {
  twakeMuiVersion: version,
  cssVarPrefix: 'twake',
  colorSchemeSelector: 'data-theme',
  paletteRaw: palette,
  schemes: { light: scheme(theme, 'light'), dark: scheme(dark, 'dark') },
  typography: Object.fromEntries(
    Object.entries(theme.typography).filter(([, v]) => typeof v === 'object' && v !== null)
  ),
  typographyBase: { fontFamily: theme.typography.fontFamily, htmlFontSize: theme.typography.htmlFontSize, fontSize: theme.typography.fontSize,
    fontWeightLight: theme.typography.fontWeightLight, fontWeightRegular: theme.typography.fontWeightRegular,
    fontWeightMedium: theme.typography.fontWeightMedium, fontWeightBold: theme.typography.fontWeightBold },
  shape: theme.shape,
  radius,
  spacing: { unit: theme.spacing(1), sample: [1, 2, 3, 4].map(n => theme.spacing(n)) },
  breakpoints: theme.breakpoints.values,
  zIndex: theme.zIndex,
  transitions: { duration: theme.transitions.duration, easing: theme.transitions.easing },
  avatarColors: { supportedColors, colorMapping },
  components,
  icons
}
console.log(JSON.stringify(plain(out), null, 2))
