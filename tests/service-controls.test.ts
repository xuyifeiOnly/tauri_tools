import { describe, expect, test } from 'bun:test'
import { ref } from 'vue'
import { useServiceControls } from '../src/composables/useServiceControls'

function setup(dirtyValue = false) {
  const dirty = ref(dirtyValue), busy = ref(false), local = ref(false)
  const config = ref({ host: '', keyId: '' })
  const calls: string[] = [], errors: unknown[] = []
  const controls = useServiceControls({ dirty, busy, local, config,
    save: async () => { calls.push('save'); dirty.value = false },
    start: async () => { calls.push('start'); local.value = true },
    onError: error => errors.push(error),
  })
  return { dirty, busy, local, config, calls, errors, controls }
}

describe('service controls', () => {
  test('local start needs neither server nor key; running prevents repeat start', async () => {
    const s = setup()
    expect(s.controls.startLabel.value).toBe('启动本地服务')
    await s.controls.startLocal()
    expect(s.calls).toEqual(['start'])
    expect(s.controls.startLabel.value).toBe('本地服务运行中')
    await s.controls.startLocal()
    expect(s.calls).toEqual(['start'])
  })
  test('dirty draft is saved before starting', async () => {
    const s = setup(true)
    expect(s.controls.startLabel.value).toBe('保存并启动本地服务')
    await s.controls.startLocal()
    expect(s.calls).toEqual(['save', 'start'])
    expect(s.dirty.value).toBe(false)
    expect(s.busy.value).toBe(false)
  })
  test.each(['save', 'start'])('%s failure resets busy without running later steps', async failure => {
    const dirty = ref(true), busy = ref(false), calls: string[] = [], errors: unknown[] = []
    const c = useServiceControls({ dirty, busy, local: ref(false), config: ref({ host: '', keyId: '' }),
      save: async () => { calls.push('save'); if (failure === 'save') throw Error('save'); dirty.value = false },
      start: async () => { calls.push('start'); throw Error('start') }, onError: e => errors.push(e),
    })
    await c.startLocal()
    expect(calls).toEqual(failure === 'save' ? ['save'] : ['save', 'start'])
    expect(dirty.value).toBe(failure === 'save')
    expect(errors).toHaveLength(1)
    expect(busy.value).toBe(false)
    expect(c.phase.value).toBe('idle')
  })
  test('pending stages have labels and block duplicate submissions', async () => {
    let finishSave!: () => void, finishStart!: () => void
    const dirty = ref(true), busy = ref(false), calls: string[] = []
    const c = useServiceControls({ dirty, busy, local: ref(false), config: ref({ host: '', keyId: '' }),
      save: async () => { calls.push('save'); await new Promise<void>(r => { finishSave = r }); dirty.value = false },
      start: async () => { calls.push('start'); await new Promise<void>(r => { finishStart = r }) }, onError: () => {},
    })
    const pending = c.startLocal()
    expect(c.startLabel.value).toBe('正在保存…')
    await c.startLocal()
    finishSave()
    await Promise.resolve(); await Promise.resolve()
    expect(c.startLabel.value).toBe('正在启动…')
    await c.startLocal()
    finishStart(); await pending
    expect(calls).toEqual(['save', 'start'])
  })
  test('connect explains every missing prerequisite', () => {
    const s = setup(true)
    expect(s.controls.connectReason.value).toContain('启动本地服务')
    expect(s.controls.connectReason.value).toContain('保存')
    expect(s.controls.connectReason.value).toContain('服务器')
    expect(s.controls.connectReason.value).toContain('密钥')
    s.local.value = true; s.dirty.value = false
    s.config.value = { host: 'example.com', keyId: 'key' }
    expect(s.controls.connectReason.value).toBe('')
  })
})
