export function statusTone(status: string): 'error' | 'success' | 'pending' | 'neutral' {
  if (/失败|错误|不可达/.test(status)) return 'error'
  if (/运行中|^可访问$/.test(status)) return 'success'
  if (/连接中|重连/.test(status)) return 'pending'
  return 'neutral'
}
