import { computed, readonly, shallowRef, type Ref } from 'vue'

interface ServiceControlsOptions {
  dirty: Readonly<Ref<boolean>>
  busy: Ref<boolean>
  local: Readonly<Ref<boolean>>
  config: Readonly<Ref<{ host: string; keyId: string }>>
  save: () => Promise<void>
  start: () => Promise<void>
  onError: (error: unknown) => void
}

export function useServiceControls(options: ServiceControlsOptions) {
  const phase = shallowRef<'idle' | 'saving' | 'starting'>('idle')
  const startLabel = computed(() => {
    if (phase.value === 'saving') return '正在保存…'
    if (phase.value === 'starting') return '正在启动…'
    if (options.local.value) return '本地服务运行中'
    return options.dirty.value ? '保存并启动本地服务' : '启动本地服务'
  })
  const connectReason = computed(() => {
    const steps: string[] = []
    if (options.dirty.value) steps.push('保存配置')
    if (!options.local.value) steps.push('启动本地服务')
    if (!options.config.value.host.trim()) steps.push('配置服务器')
    if (!options.config.value.keyId) steps.push('选择 SSH 密钥')
    return steps.length ? `连接公网前，请先${steps.join('、')}。` : ''
  })
  async function startLocal() {
    if (options.busy.value || options.local.value) return
    options.busy.value = true
    try {
      if (options.dirty.value) {
        phase.value = 'saving'
        await options.save()
      }
      phase.value = 'starting'
      await options.start()
    } catch (error) {
      options.onError(error)
    } finally {
      phase.value = 'idle'
      options.busy.value = false
    }
  }
  return { phase: readonly(phase), startLabel, connectReason, startLocal }
}
