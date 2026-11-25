import { invoke, isTauri } from '@tauri-apps/api/core'
export const native=isTauri()
export async function call<T>(name:string,args?:Record<string,unknown>):Promise<T>{if(!native)throw new Error('当前是界面预览，请在随享桌面客户端中操作真实服务');return invoke<T>(name,args)}
export async function openAddress(url:string){if(native)await call('open_url',{url});else window.open(url,'_blank','noopener,noreferrer')}
