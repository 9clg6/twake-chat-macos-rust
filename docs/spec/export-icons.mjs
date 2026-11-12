// Exports the @linagora/twake-icons SVG sprite to icons.json.
// Usage: node export-icons.mjs [path/to/twake-space-chat] > icons.json   (anomalies go to stderr)
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const root = resolve(process.argv[2] ?? `${process.env.HOME}/Developer/twake-space-chat`)
const pkgDir = `${root}/node_modules/@linagora/twake-icons`
const pkg = JSON.parse(readFileSync(`${pkgDir}/package.json`, 'utf8'))
const src = readFileSync(`${pkgDir}/dist/index.js`, 'utf8')

// The sprite is one template literal of the bundle (no ${}): evaluate it to unescape.
const start = src.lastIndexOf('`', src.indexOf('<symbol'))
const end = src.indexOf('`', start + 1)
const lit = src.slice(start + 1, end)
if (lit.includes('${')) throw new Error('sprite literal has interpolation')
const sprite = new Function('return `' + lit + '`')()

const fail = m => { console.error('ERROR: ' + m); process.exit(1) }
// Shared <defs>: the per-icon defs concatenated in symbol order, each restarting its ids at
// "a" (so ids repeat and a bare url(#a) is ambiguous). Each icon takes, in order, the top-level
// elements it needs: consume until every id it references (recursively) is defined.
const defsText = sprite.match(/<defs>([\s\S]*?)<\/defs>/)[1]
const els = []
for (let depth = 0, from = 0, m, re = /<(\/?)([\w:]+)([^>]*?)(\/?)>/g; (m = re.exec(defsText));) {
  if (m[1]) depth--
  else if (!m[4]) depth++
  if (depth === 0) {
    const el = defsText.slice(from, re.lastIndex); from = re.lastIndex
    const id = el.match(/^<[\w:]+[^>]*\bid="([^"]*)"/)[1]
    els.push([id, el])
  }
}
const refsOf = t => [...t.matchAll(/url\(#([^)]*)\)|href="#([^"]*)"/g)].map(m => m[1] ?? m[2])
const symbols = [...sprite.matchAll(/<symbol\b([^>]*)>([\s\S]*?)<\/symbol>/g)]
if (symbols.length !== (sprite.match(/<symbol\b/g) ?? []).length) fail('symbol count mismatch')

const attr = (a, n) => a.match(new RegExp(`\\b${n}="([^"]*)"`))?.[1]
const icons = {}
const inlined = [], noViewBox = []
let next = 0
const note = {}
for (const [, attrs, body] of symbols) {
  // kebab-case id (the source has carbonCopy)
  const id = attr(attrs, 'id')?.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase(), viewBox = attr(attrs, 'viewBox')
  if (!id) fail('symbol without id')
  // ponytail: Icon's DEFAULT_SIZE is 16 and the standalone components carry no viewBox either
  if (!viewBox) noViewBox.push(id)
  if (id in icons) fail(`duplicate id ${id}`)
  if (!body.trim()) fail(`empty body ${id}`)
  let out = body
  const need = new Set(refsOf(body).filter(r => !body.includes(`id="${r}"`))) // some symbols define their own mask
  if (need.size) {
    const chunk = {}
    const covered = () => {
      const seen = new Set(), q = [...need]
      for (let r; (r = q.pop()) !== undefined;) {
        if (seen.has(r)) continue
        seen.add(r)
        if (!chunk[r]) return false
        q.push(...refsOf(chunk[r]))
      }
      return true
    }
    while (!covered()) {
      if (next >= els.length) fail(`defs exhausted at ${id}`)
      const [eid, el] = els[next++]
      if (eid in chunk) fail(`id ${eid} repeats in the defs of ${id}`)
      chunk[eid] = el
    }
    const used = new Set()
    const walk = r => { if (!used.has(r)) { used.add(r); refsOf(chunk[r]).forEach(walk) } }
    need.forEach(walk)
    const rename = t => t.replace(/url\(#([^)]*)\)|href="#([^"]*)"/g, (x, u, h) => (u ? `url(#${id}-${u})` : `href="#${id}-${h}"`))
    const defs = [...used].sort().map(r => rename(chunk[r].replace(`id="${r}"`, `id="${id}-${r}"`))).join('')
    out = `<defs>${defs}</defs>${rename(body).replace(/\bid="([^"]*)"/g, (x, l) => `id="${id}-${l}"`)}`
    inlined.push(id)
  }
  icons[id] = { viewBox: viewBox ?? '0 0 16 16', body: out }
  const flag = (k, c) => c && (note[k] ??= []).push(id)
  flag('style', /<style\b/.test(body))
  flag('defs', /<defs\b/.test(body))
  flag('url(#) refs', /url\(#/.test(body))
  flag('hardcoded fill/stroke', /\b(?:fill|stroke)="(?!none|currentColor)[^"]*"/.test(body))
  flag('inline style attr', /\bstyle="/.test(body))
  flag('opacity', /\b(?:fill-|stroke-)?opacity="/.test(body))
}
if (next !== els.length) fail(`unused defs elements: ${els.length - next}`)
if (Object.keys(icons).length !== symbols.length) fail('count mismatch')

const sorted = Object.fromEntries(Object.keys(icons).sort().map(k => [k, icons[k]]))
const out = JSON.stringify({ source: `${pkg.name}@${pkg.version}`, icons: sorted }, null, 2) + '\n'

const outside = sprite.replace(/<symbol\b[\s\S]*?<\/symbol>/g, '')
console.error(`icons: ${symbols.length}, license: ${pkg.license}`)
console.error(`shared markup outside symbols: ${outside.length} chars, ids: ${[...outside.matchAll(/\bid="([^"]*)"/g)].map(m => m[1]).join(',')}`)
console.error(`no viewBox, set to 0 0 16 16 (${noViewBox.length}): ${noViewBox.join(' ')}`)
console.error(`defs inlined, ids renamed (${inlined.length}): ${inlined.join(' ')}`)
console.error(`viewBoxes: ${JSON.stringify(Object.entries(Object.values(icons).reduce((a, i) => (a[i.viewBox] = (a[i.viewBox] ?? 0) + 1, a), {})))}`)
for (const [k, v] of Object.entries(note)) console.error(`${k} (${v.length}): ${v.join(' ')}`)
process.stdout.write(out)
