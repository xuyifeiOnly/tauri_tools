import { useToast } from './useToast'

export function useCopyFeedback(onError: (message: string) => void) {
  const { show } = useToast()

  async function copy(text: string, success = '已复制') {
    try {
      await navigator.clipboard.writeText(text)
      show(success, 'success')
    } catch (error) {
      onError(String(error))
    }
  }

  return { copy }
}
