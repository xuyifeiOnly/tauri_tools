import { inject, onUnmounted, provide, readonly, shallowRef, type InjectionKey } from 'vue'

export type ToastKind = 'success' | 'error' | 'info'
interface Toast { message: string; kind: ToastKind }
interface ToastContext { show: (message: string, kind?: ToastKind) => void }
const toastKey: InjectionKey<ToastContext> = Symbol('toast')

export function provideToast() {
  const toast = shallowRef<Toast | null>(null)
  let timer: ReturnType<typeof setTimeout> | undefined
  function dismiss() { clearTimeout(timer); toast.value = null }
  function show(message: string, kind: ToastKind = 'error') {
    clearTimeout(timer)
    toast.value = { message, kind }
    // 错误可手动关闭；成功及信息提示自动消失。
    if (kind !== 'error') timer = setTimeout(dismiss, kind === 'success' ? 2000 : 5000)
  }
  provide(toastKey, { show })
  onUnmounted(() => clearTimeout(timer))
  return { toast: readonly(toast), show, dismiss }
}

export function useToast() {
  const context = inject(toastKey)
  if (!context) throw new Error('useToast requires provideToast')
  return context
}
