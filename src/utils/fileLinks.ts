import type { Config, Mapping } from '../types'

export function fileKind(name: string): string {
  const ext = name.split('.').pop()?.toLowerCase() || ''
  if (['html','htm','css','js','ts','tsx','jsx','vue','json','xml','py','rs','sh','sql','yaml','yml'].includes(ext)) return 'code'
  if (ext === 'pdf') return 'pdf'
  if (['png','jpg','jpeg','gif','svg','webp','heic','avif'].includes(ext)) return 'image'
  if (['doc','docx','txt','md','rtf','pages'].includes(ext)) return 'document'
  if (['xls','xlsx','csv','numbers'].includes(ext)) return 'sheet'
  if (['zip','tar','gz','rar','7z','dmg'].includes(ext)) return 'archive'
  if (['mp4','mov','webm','mkv'].includes(ext)) return 'video'
  if (['mp3','wav','m4a','flac','ogg'].includes(ext)) return 'audio'
  return 'file'
}

export type ShareChannel = 'local' | 'lan' | 'public'
type ShareStatus = {local:boolean;public:string}

export function shareBlockReason(channel:ShareChannel, config:Config, mapping:Mapping|undefined, state:ShareStatus):string {
  if (!mapping) return '请先保存此目录'
  if (!mapping.enabled) return '此目录未启用'
  if (channel === 'lan' && !mapping.allowLan) return '请先允许局域网访问，保存并重启服务'
  if (channel === 'public' && !config.publicOrigin) return '请先设置并保存公网源站'
  if (!state.local) return '请先启动本地文件服务'
  if (channel === 'public' && state.public !== '可访问') return '公网尚未确认可达，请检查隧道状态'
  return ''
}

export function fileLink(channel:ShareChannel, config:Config, mapping:Mapping, state:ShareStatus, path:string, ip=''):string {
  const reason = shareBlockReason(channel, config, mapping, state)
  if (reason) throw new Error(reason)
  let base: string
  if (channel === 'local') {
    base = `http://127.0.0.1:${config.localPort}`
  } else if (channel === 'lan') {
    if (!ip) throw new Error('未检测到局域网IPv4地址')
    base = `http://${ip}:${config.localPort}`
  } else {
    base = config.publicOrigin.replace(/\/$/,'') + config.basePath.replace(/\/$/,'')
  }
  return `${base}/${encodeURIComponent(mapping.route)}/${path.split('/').map(encodeURIComponent).join('/')}`
}
