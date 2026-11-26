import { expect, test } from 'bun:test'
import { versionUpdates } from '../scripts/bump-version'

const texts = ['{ "version": "0.1.2", "name": "demo" }\n', '{"version":"0.1.2","app":{}}', '[package]\nname = "demo"\nversion = "0.1.2"\n\n[dependencies]\nlib = { version = "2" }\n']
test('defaults to patch +1 and preserves formatting and dependencies', () => {
  const result = versionUpdates(texts)
  expect(result.target).toBe('0.1.3')
  expect(result.updated).toEqual(texts.map(text => text.replace('0.1.2', '0.1.3')))
})
test('explicit version can unify inconsistent source versions', () => {
  const mixed = texts.map((text, i) => text.replace('0.1.2', `0.1.${i}`))
  expect(() => versionUpdates(mixed)).toThrow('不一致')
  expect(versionUpdates(mixed, '1.0.0').updated).toEqual(texts.map(text => text.replace('0.1.2', '1.0.0')))
})
test('rejects invalid versions and ambiguous JSON without modifying input', () => {
  expect(() => versionUpdates(texts, 'v1.0.0')).toThrow('X.Y.Z')
  expect(() => versionUpdates([texts[0], texts[1], '[dependencies]\nversion="1"'])).toThrow('[package]')
  expect(() => versionUpdates([texts[0].replace('"name": "demo"', '"nested":{"version":"2"}'), ...texts.slice(1)])).toThrow('不唯一')
  expect(texts[0]).toContain('0.1.2')
})
