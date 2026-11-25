<script setup lang="ts">
import { computed, ref, shallowRef } from 'vue'
import DirectoryBrowser from './DirectoryBrowser.vue'
import SettingsFooter from './SettingsFooter.vue'
import { ChevronRight } from 'lucide-vue-next'
import { useCopyFeedback } from '../composables/useCopyFeedback'
import { open } from '@tauri-apps/plugin-dialog'
import type { Config, Mapping } from '../types'
import { call,openAddress } from '../api'
import { useToast } from '../composables/useToast'
import { fileLink,shareBlockReason,type ShareChannel } from '../utils/fileLinks'
const { show } = useToast()
const props=defineProps<{config:Config;saved:Config;state:{local:boolean;public:string};dirty:boolean;busy:boolean;restart:boolean;saveLabel:string;setPassword:(id:string,username:string,password:string)=>Promise<boolean>}>()
const emit=defineEmits<{save:[];start:[];error:[message:string]}>()
const passwords=ref<Record<string,string>>({})
const browsing=ref<Mapping|null>(null)
const expanded=ref(new Set<string>())
const localBusy=shallowRef(false)
const ips=ref<string[]>([])
const localChoice=ref<{id:string;action:'open'|'copy'}|null>(null)
const savedMappings=computed(()=>new Map(props.saved.mappings.map(m=>[m.id,m])))
const browserMapping=computed(()=>browsing.value&&savedMappings.value.get(browsing.value.id))
const channels=[['local','本机'],['lan','局域网'],['public','公网']] as const
function reason(id:string,channel:ShareChannel){return shareBlockReason(channel,props.saved,savedMappings.value.get(id),props.state)}
const {copy}=useCopyFeedback(message=>emit('error',message))
function toggle(id:string){if(expanded.value.has(id))expanded.value.delete(id);else expanded.value.add(id)}
function add(){const id=crypto.randomUUID();let n=1;while(props.config.mappings.some(m=>m.route===`folder-${n}`))n++;props.config.mappings.push({id,name:'新目录',localPath:'',route:`folder-${n}`,enabled:true,allowLan:false,username:'',passwordHash:''});expanded.value.add(id)}
async function choose(m:Mapping){try{const path=await open({directory:true,multiple:false});if(typeof path==='string')m.localPath=path}catch(e){emit('error',String(e))}}
const usernames=ref<Record<string,string>>({})
async function protect(m:Mapping){
  const username=usernames.value[m.id]??m.username
  const password=passwords.value[m.id]||''
  if(!username.trim()||password.length<8){show('请输入用户名和至少 8 位密码');return}
  if(await props.setPassword(m.id,username,password)){delete passwords.value[m.id];delete usernames.value[m.id]}
}
async function share(id:string,channel:ShareChannel,action:'open'|'copy',ip=''){
  if(localBusy.value)return
  localBusy.value=true
  try{
    const saved=props.saved
    const mapping=savedMappings.value.get(id)
    if(!mapping)throw new Error('请先保存此目录')
    const state=await call<{local:boolean;public:string}>('status')
    if(channel==='lan'){
      const blocked=shareBlockReason(channel,saved,mapping,state)
      if(blocked)throw new Error(blocked)
      ips.value=await call<string[]>('lan_addresses')
      if(ip&&!ips.value.includes(ip))throw new Error('所选 IP 已不可用，请重新选择')
      if(!ip&&ips.value.length>1){localChoice.value={id,action};return}
      ip ||= ips.value[0]||''
    }
    const url=fileLink(channel,saved,mapping,state,'',ip)
    localChoice.value=null
    if(action==='open')await openAddress(url)
    else await copy(url,'地址已复制')
  }catch(e){localChoice.value=null;emit('error',String(e))}
  finally{localBusy.value=false}
}
</script>
<template>
  <section class="workspace-panel">
    <DirectoryBrowser v-if="browserMapping" :config="saved" :mapping="browserMapping" :state="state" @close="browsing=null" />
    <div class="section-head workspace-heading"><div><h2>目录映射</h2><p class="muted">只共享你选定的目录。移除映射不会删除电脑文件。</p></div><button class="primary" :disabled="busy" @click="add">添加目录</button></div>
    <div class="scroll-region" tabindex="0" role="region" aria-label="目录映射列表">
    <div v-if="!config.mappings.length" class="empty panel">尚未添加目录。添加后即可在本地浏览器访问。</div>
    <article v-for="m in config.mappings" :key="m.id" class="panel mapping">
      <div class="mapping-head">
        <button class="mapping-toggle" :aria-expanded="expanded.has(m.id)" :aria-controls="`mapping-${m.id}`" @click="toggle(m.id)">
          <ChevronRight :size="18" class="mapping-chevron" :class="{expanded:expanded.has(m.id)}" aria-hidden="true" />
          <span class="mapping-summary"><strong>{{m.name||'未命名目录'}}</strong><span class="mapping-route">/{{m.route}}/</span></span>
          <span class="mapping-toggle-text">{{expanded.has(m.id)?'收起':'展开'}}</span>
        </button>
        <span class="mapping-route">{{!savedMappings.has(m.id)?'未保存':!savedMappings.get(m.id)?.enabled?'已停用':`${savedMappings.get(m.id)?.allowLan?'允许局域网':'局域网关闭'} · ${savedMappings.get(m.id)?.passwordHash?'有密码':'未设密码'}`}}</span>
        <label class="inline mapping-enabled"><input v-model="m.enabled" :disabled="busy" type="checkbox" :aria-label="`${m.name||'未命名目录'}启用`">启用</label>
        <button class="browse-button" :disabled="busy||!savedMappings.has(m.id)" @click="browsing=m">浏览文件</button>
      </div>
      <div v-show="expanded.has(m.id)" :id="`mapping-${m.id}`" class="mapping-body">
      <div class="fields">
        <div class="wide"><label :for="`path-${m.id}`">本地目录</label><div class="row path-row"><input :id="`path-${m.id}`" v-model="m.localPath" :disabled="busy" placeholder="选择要共享的目录"><button :disabled="busy" @click="choose(m)">选择目录</button></div></div>
        <label>显示名称<input v-model="m.name" :disabled="busy"></label><label>子路径<input v-model="m.route" :disabled="busy" placeholder="docs"></label>
        <div class="wide lan-setting"><label class="lan-toggle"><input v-model="m.allowLan" :disabled="busy" type="checkbox" @change="localChoice=null"><span>允许局域网访问</span></label><span class="lan-hint">保存并重启服务后生效</span></div>
      </div>
      <details><summary>分享地址 <small>使用已保存的配置</small></summary>
      <div v-for="[channel,label] in channels" :key="channel" class="share-row">
        <div><strong>{{label}}</strong><small>{{reason(m.id,channel)||(channel==='local'?'仅此电脑':channel==='lan'?'同一局域网内的设备':'公网验证已通过')}}</small></div>
        <button :disabled="busy||localBusy||!!reason(m.id,channel)" @click="share(m.id,channel,'open')">打开{{label}}地址 ↗</button>
        <button :disabled="busy||localBusy||!!reason(m.id,channel)" @click="share(m.id,channel,'copy')">复制{{label}}地址</button>
      </div>
      <div v-if="localChoice?.id===m.id" class="local-ip-picker" role="group" aria-label="选择电脑局域网IP">
        <span class="lan-hint">选择接收设备可达的 IP</span>
        <button v-for="ip in ips" :key="ip" :disabled="busy||localBusy" @click="share(m.id,'lan',localChoice!.action,ip)">{{ip}}</button>
        <button :disabled="localBusy" @click="localChoice=null">取消</button>
      </div>
      </details>
      <details><summary>访问保护 · {{m.passwordHash?'已启用':'未启用'}}</summary>
        <p class="muted">{{dirty?'请先保存配置，再设置密码。':state.local?'保存密码会同时停止本地服务和 SSH 隧道。':'密码至少 8 位，公网应使用 HTTPS。'}}</p>
        <div class="fields"><label>用户名<input :value="usernames[m.id]??m.username" :disabled="busy" autocomplete="off" @input="usernames[m.id]=($event.target as HTMLInputElement).value"></label><label>新密码<input v-model="passwords[m.id]" :disabled="busy" type="password" autocomplete="new-password"></label></div>
        <div class="row"><button :disabled="busy||dirty||!savedMappings.has(m.id)" @click="protect(m)">{{state.local?'保存密码并停止共享':'保存密码'}}</button><button v-if="m.passwordHash" :disabled="busy" @click="m.passwordHash='';m.username='';delete usernames[m.id];delete passwords[m.id]">取消保护（保存后生效）</button></div>
      </details>
      <button class="danger" :disabled="busy" @click="config.mappings=config.mappings.filter(i=>i.id!==m.id)">移除映射</button>
      </div>
    </article>
    </div>
    <SettingsFooter :label="saveLabel" :dirty="dirty" :busy="busy" :running="state.local" :restart="restart" @save="emit('save')" @start="emit('start')"/>
  </section>
</template>

<style scoped>
.mapping.panel { padding:4px 12px; margin-bottom:8px; }
.browse-button { padding:6px 10px; font-size:12px; }
.mapping-head { display: flex; align-items: center; gap: 16px; }
.mapping-toggle { display: flex; align-items: center; gap: 10px; flex: 1; min-width: 0; padding: 4px 0; border: 0; background: transparent; text-align: left; min-height: 44px; }
.mapping-toggle:hover { background: var(--muted); }
.mapping-summary { display: flex; align-items: baseline; gap: 14px; flex: 1; min-width: 0; }
.mapping-summary strong, .mapping-route { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mapping-route { color: var(--secondary); font-size: 12px; }
.mapping-toggle-text { color: var(--secondary); font-size: 12px; flex-shrink: 0; }
.mapping-chevron { flex-shrink: 0; color: var(--secondary); }
.mapping-chevron.expanded { transform: rotate(90deg); }
.mapping-enabled { flex-shrink: 0; }
.mapping-body { padding:12px 0; margin-top:4px; border-top:1px solid var(--border); }
.mapping-body .fields { gap:12px 16px; margin:0 0 12px; }
.path-row { margin-top:7px; }
.mapping summary small { display:inline; margin-left:8px; }
.share-row { display:grid; grid-template-columns:minmax(0,1fr) auto auto; align-items:center; gap:8px; margin-top:10px; }
.lan-setting { display:flex; align-items:center; justify-content:flex-start; flex-wrap:wrap; gap:4px 12px; }
.lan-toggle { display:inline-flex; align-items:center; justify-content:flex-start; gap:8px; min-height:36px; cursor:pointer; }
.lan-toggle input { width:16px; height:16px; flex:0 0 16px; margin:0; padding:0; accent-color:var(--accent); }
.lan-hint { color:var(--secondary); font-size:12px; font-weight:400; }
.local-ip-picker { display:flex; align-items:center; flex-wrap:wrap; gap:8px; margin:0 0 12px; padding:8px 12px; border-radius:7px; background:var(--muted); }
.local-ip-picker button { min-height:36px; padding:6px 10px; }
</style>
