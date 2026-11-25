<script setup lang="ts">
import { ref, onMounted } from 'vue'
import type { Config, Mapping } from '../types'
import { call } from '../api'
import { provideToast } from '../composables/useToast'
import AppToast from './AppToast.vue'
import { ChevronRight, Folder, FolderOpen, File, FileCode2, FileText, FileImage, FileSpreadsheet, FileArchive, FileVideo, FileAudio, Network, Globe, X, ShieldCheck, LockKeyhole } from 'lucide-vue-next'
import { fileKind, fileLink, shareBlockReason } from '../utils/fileLinks'
interface Entry {name:string;path:string;directory:boolean;depth:number}
const props=defineProps<{config:Config;mapping:Mapping;state:{local:boolean;public:string}}>()
const emit=defineEmits<{close:[]}>()
const dialog=ref<HTMLDialogElement>(), rows=ref<Entry[]>([]), expanded=ref(new Set<string>())
const ips=ref<string[]>([]), error=ref(''), loading=ref(false), ready=ref(false), copying=ref(false)
const choosing=ref<Entry|null>(null)
const fileIcons:Record<string,typeof File>={code:FileCode2,pdf:FileText,image:FileImage,document:FileText,sheet:FileSpreadsheet,archive:FileArchive,video:FileVideo,audio:FileAudio,file:File}
const {toast,show,dismiss}=provideToast()
function reason(channel:'lan'|'public'){return shareBlockReason(channel,props.config,props.mapping,props.state)}
onMounted(async()=>{
  dialog.value?.showModal()
  try {
    rows.value=(await call<Entry[]>('list_directory',{id:props.mapping.id,relative:''})).map(e=>({...e,depth:0}))
    ready.value=true
  }catch(e){error.value=String(e)}
})
async function toggle(entry:Entry){
  if(loading.value)return
  if(expanded.value.has(entry.path)){
    choosing.value=null;expanded.value.delete(entry.path);rows.value=rows.value.filter(e=>!e.path.startsWith(entry.path+'/'))
    for(const path of expanded.value)if(path.startsWith(entry.path+'/'))expanded.value.delete(path)
    return
  }
  loading.value=true
  try{const children=await call<Entry[]>('list_directory',{id:props.mapping.id,relative:entry.path});rows.value.splice(rows.value.indexOf(entry)+1,0,...children.map(e=>({...e,depth:entry.depth+1})));expanded.value.add(entry.path)}catch(e){show(String(e))}finally{loading.value=false}
}
async function copyFile(entry:Entry,channel:'lan'|'public',address?:string){
  if(copying.value)return
  copying.value=true
  try{
    const state=await call<{local:boolean;public:string}>('status')
    if(channel==='lan'){
      const blocked=shareBlockReason(channel,props.config,props.mapping,state)
      if(blocked)throw new Error(blocked)
      ips.value=await call<string[]>('lan_addresses')
      if(address&&!ips.value.includes(address)){choosing.value=null;throw new Error('所选 IP 已不可用，请重新选择')}
      if(!address&&ips.value.length>1){choosing.value=entry;return}
      address ||= ips.value[0]
    }
    await navigator.clipboard.writeText(fileLink(channel,props.config,props.mapping,state,entry.path,address))
    choosing.value=null
    show(channel==='lan'?'局域网链接已复制':'公网隧道链接已复制','success')
  }catch(e){show(String(e))}finally{copying.value=false}
}
</script>
<template>
  <dialog ref="dialog" class="directory-dialog" aria-labelledby="directory-title" @close="emit('close')" @keydown.esc="choosing&&(choosing=null,$event.preventDefault())">
    <AppToast :toast="toast" @dismiss="dismiss" />
    <div class="dialog-head"><div class="dialog-title"><FolderOpen :size="20" class="type-folder" aria-hidden="true"/><h2 id="directory-title" :title="mapping.name">{{mapping.name}}</h2><span class="directory-label">文件目录</span></div><button class="close-button" autofocus aria-label="关闭目录树" @click="dialog?.close()"><X :size="16" aria-hidden="true"/></button></div>
    <p class="browser-note"><component :is="mapping.passwordHash?LockKeyhole:ShieldCheck" :size="13" aria-hidden="true"/>只读 · {{mapping.passwordHash?'已启用访问保护':'未设置访问密码'}}<span>隐藏文件、密钥与符号链接不展示</span></p>
    <p v-if="error" class="browser-error" role="alert">{{error}}</p>
    <p v-else-if="!ready" class="empty-state" role="status">正在读取目录…</p>
    <div v-if="ready" class="browser-content">
      <div class="directory-entries" :aria-busy="loading">
        <template v-for="entry in rows" :key="entry.path">
          <div class="file-row" :style="{paddingLeft:`${8+Math.min(entry.depth,12)*16}px`}">
            <button v-if="entry.directory" class="folder" :title="entry.name" :aria-expanded="expanded.has(entry.path)" :disabled="loading" @click="toggle(entry)"><ChevronRight :size="13" class="chevron" :class="{expanded:expanded.has(entry.path)}" aria-hidden="true"/><component :is="expanded.has(entry.path)?FolderOpen:Folder" :size="18" class="type-folder" aria-hidden="true"/><span class="entry-name">{{entry.name}}</span></button>
            <div v-else class="file-name" :title="entry.name"><component :is="fileIcons[fileKind(entry.name)]" :size="18" :class="`type-${fileKind(entry.name)}`" aria-hidden="true"/><span class="entry-name">{{entry.name}}</span></div>
            <div v-if="!entry.directory" class="file-actions">
              <button class="copy-button" :disabled="copying||!!reason('lan')" :aria-label="`复制 ${entry.name} 的局域网地址`" :title="reason('lan')||'复制局域网地址'" :aria-expanded="choosing?.path===entry.path" @click="choosing?.path===entry.path?choosing=null:copyFile(entry,'lan')"><Network :size="12" aria-hidden="true"/>局域网</button>
              <button class="copy-button" :disabled="copying||!!reason('public')" :aria-label="`复制 ${entry.name} 的公网隧道地址`" :title="reason('public')||'复制公网隧道地址'" @click="copyFile(entry,'public')"><Globe :size="12" aria-hidden="true"/>公网隧道</button>
            </div>
          </div>
          <div v-if="choosing?.path===entry.path" class="ip-picker" role="group" aria-label="选择局域网地址"><span>选择接收设备可达的 IP</span><button v-for="address in ips" :key="address" class="copy-button" :disabled="copying" @click="copyFile(entry,'lan',address)">{{address}}</button><button class="copy-button" @click="choosing=null">取消</button></div>
        </template>
        <p v-if="!rows.length" class="empty-state">目录为空或没有可展示文件</p>
      </div>
      <div class="browser-footer"><span>{{rows.length}} 个可见项目<span v-if="loading"> · 正在读取…</span></span><span>局域网 HTTP 不加密</span></div>
    </div>
  </dialog>
</template>
<style scoped>
.directory-dialog { --file-folder:#b77912; --file-code:#c65d18; --file-pdf:#ce3f4d; --file-image:#8754c7; --file-document:#367ac1; --file-sheet:#24845f; --file-archive:#96713c; --file-media:#a34b91; width:min(760px,calc(100vw - 64px)); max-height:80vh; padding:16px; color:var(--text); background:var(--surface); border:1px solid var(--border); border-radius:12px; box-shadow:0 20px 64px #0003; }
:global(.app.dark .directory-dialog) { --file-folder:#e7b454; --file-code:#f4a26c; --file-pdf:#f58391; --file-image:#b79aec; --file-document:#81b5ed; --file-sheet:#6dcea8; --file-archive:#d2b079; --file-media:#dd97cf; }
.directory-dialog::backdrop { background:#0006; }
.dialog-head,.dialog-title,.file-row,.file-name,.folder,.file-actions,.browser-note,.browser-footer { display:flex; align-items:center; }
.dialog-head { justify-content:space-between; gap:16px; }.dialog-title { gap:8px; min-width:0; }.dialog-title h2 { margin:0; font-size:16px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }.directory-label { color:var(--secondary); font-size:12px; white-space:nowrap; }
.directory-dialog .close-button { display:grid; place-items:center; min-height:28px; width:28px; padding:0; border:0; flex-shrink:0; }
.browser-note { gap:6px; flex-wrap:wrap; font-size:12px; color:var(--secondary); margin:10px 0 14px; }.browser-note span { margin-left:auto; }
.directory-dialog[open] { display:flex; flex-direction:column; overflow:hidden; }
.dialog-head,.browser-note,.browser-footer,.browser-error { flex-shrink:0; }
.browser-content { display:flex; flex-direction:column; min-height:0; }
.directory-entries { min-height:0; overflow:auto; max-height:52vh; overscroll-behavior:contain; scrollbar-width:thin; scrollbar-color:var(--border) transparent; border:1px solid var(--border); border-radius:7px; }
.file-row { height:36px; gap:12px; padding:0 8px; border-bottom:1px solid var(--border); }.file-row:last-child { border-bottom:0; }.file-row:hover { background:var(--muted); }
.file-name,.folder { gap:8px; flex:1; min-width:0; text-align:left; font-size:13px; }.file-name { padding-left:21px; }.directory-dialog .folder { min-height:34px; padding:0; border:0; background:transparent; }
.entry-name { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }.file-name svg,.folder svg { flex-shrink:0; }.chevron { color:var(--secondary); }.chevron.expanded { transform:rotate(90deg); }
.file-actions { gap:8px; flex-shrink:0; }.directory-dialog .copy-button { display:inline-flex; align-items:center; gap:5px; min-height:28px; padding:3px 7px; font-size:12px; line-height:20px; border-radius:5px; color:var(--secondary); background:transparent; white-space:nowrap; }
.directory-dialog .copy-button:hover { color:var(--accent); border-color:var(--accent); }.ip-picker { display:flex; align-items:center; gap:8px; flex-wrap:wrap; padding:8px 12px; background:var(--muted); border-bottom:1px solid var(--border); font-size:12px; color:var(--secondary); }
.browser-footer { justify-content:space-between; gap:12px; margin-top:10px; font-size:12px; color:var(--secondary); }.empty-state { text-align:center; padding:24px 12px; margin:0; color:var(--secondary); font-size:13px; }.browser-error { color:var(--danger); font-size:13px; margin:12px 0; }
.type-folder { color:var(--file-folder); fill:var(--file-folder); fill-opacity:.12; }.type-code { color:var(--file-code); }.type-pdf { color:var(--file-pdf); }.type-image { color:var(--file-image); }.type-document { color:var(--file-document); }.type-sheet { color:var(--file-sheet); }.type-archive { color:var(--file-archive); }.type-audio,.type-video { color:var(--file-media); }.type-file { color:var(--secondary); }
</style>
