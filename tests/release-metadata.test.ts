import { expect, test } from 'bun:test'
import { releaseMetadata } from '../scripts/release-metadata'

const versions = ['0.1.2', '0.1.2', '0.1.2']
test('manual release uses source version and exact commit', () => {
  expect(releaseMetadata(versions, 'refs/heads/main', 'abc123')).toEqual({version:'0.1.2', tag:'v0.1.2', sha:'abc123'})
})
test('matching version tag is accepted', () => {
  expect(releaseMetadata(versions, 'refs/tags/v0.1.2', 'abc123').tag).toBe('v0.1.2')
})
test('mismatched or unsupported tags are rejected', () => {
  for (const tag of ['v0.1.3', 'vnightly', 'v0.1.2-beta']) {
    expect(() => releaseMetadata(versions, `refs/tags/${tag}`, 'abc123')).toThrow('标签')
  }
})
test('inconsistent and invalid source versions are rejected', () => {
  expect(() => releaseMetadata(['0.1.2', '0.1.3', '0.1.2'], '', 'abc123')).toThrow('一致')
  for (const version of ['', '0.1', '0.1.2-beta', '01.1.2']) {
    expect(() => releaseMetadata([version, version, version], '', 'abc123')).toThrow('X.Y.Z')
  }
})
