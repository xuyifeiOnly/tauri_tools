<script setup lang="ts">
import {ref,shallowRef,onMounted,onUnmounted,computed} from 'vue'
import {LayoutDashboard,FolderOpen,Settings,KeyRound,FileCode,ScrollText} from 'lucide-vue-next'
import appIcon from '../src-tauri/icons/mac-tunnel.svg'
import {defaults,type Config,type KeyInfo,type RuntimeStatus} from './types'
import {call,native,openAddress} from './api'
import ConnectionPanel from './components/ConnectionPanel.vue'
import MappingsPanel from './components/MappingsPanel.vue'
import KeysPanel from './components/KeysPanel.vue'
import DeploymentPanel from './components/DeploymentPanel.vue'
import AppToast from './components/AppToast.vue'
import {provideToast} from './composables/useToast'
import {useServiceControls} from './composables/useServiceControls'
import {statusTone} from './utils/statusTone'
const {toast,show,dismiss}=provideToast()
const config=ref<Config>(structuredClone(defaults))
const keys=ref<KeyInfo[]>([])
const tab=ref('概览'),busy=ref(false),dark=ref(false),ready=ref(!native)
const saved=shallowRef<Config>(structuredClone(defaults))
const dirty=computed(()=>JSON.stringify(saved.value)!==JSON.stringify(config.value))
const hasFolders=computed(()=>config.value.mappings.some(m=>m.enabled&&m.localPath.trim()))
const saveLabel=computed(()=>busy.value?'处理中…':state.value.local?'保存并停止共享':'保存配置')
const restart=ref(false)
const checkingPublic=ref(false)
async function checkPublic(){if(checkingPublic.value||busy.value)return;checkingPublic.value=true;try{const reachable=await call<boolean>('check_public');show(reachable?'公网验证通过，已匹配当前应用实例':'公网验证失败，请检查隧道与代理配置',reachable?'success':'error')}catch(e){show(String(e))}finally{await poll();checkingPublic.value=false}}
const state=ref<RuntimeStatus>({local:false,tunnel:'停止',public:'未检查',publicCheckedAt:null,logs:[]})
const tunnelActive=computed(()=>['连接中','重连中','重连等待','SSH进程运行中'].includes(state.value.tunnel))
const publicChecked=computed(()=>state.value.publicCheckedAt?new Date(state.value.publicCheckedAt*1000).toLocaleString('zh-CN',{month:'numeric',day:'numeric',hour:'2-digit',minute:'2-digit',second:'2-digit'}):'尚未验证')
const nav=[['概览',LayoutDashboard],['目录映射',FolderOpen],['连接配置',Settings],['密钥管理',KeyRound],['部署助手',FileCode],['运行日志',ScrollText]] as const
async function refreshKeys(created?:KeyInfo){try{keys.value=await call('list_keys');if(created)config.value.keyId=created.id;else if(!config.value.keyId&&keys.value.length===1)config.value.keyId=keys.value[0]!.id}catch(e){show(String(e))}}
async function action(name:string,label:string){if(busy.value)return;busy.value=true;try{await call(name);show(label,'success')}catch(e){show(String(e))}finally{await poll();busy.value=false}}
async function persistConfig(){const snapshot:Config=JSON.parse(JSON.stringify(config.value));await call('save_config',{config:snapshot});saved.value=snapshot}
async function save(){
  if(busy.value||!ready.value||!dirty.value)return
  const wasRunning=state.value.local
  busy.value=true
  try{await persistConfig();restart.value ||= wasRunning;show(wasRunning?'配置已保存，共享已停止':'配置已保存','success')}
  catch(e){show(String(e))}finally{await poll();busy.value=false}
}
async function setPassword(id:string,username:string,password:string){
  if(busy.value||!ready.value||dirty.value)return false
  const wasRunning=state.value.local
  busy.value=true
  try{
    await call('set_password',{id,username,password})
    const updated=structuredClone(saved.value)
    const mapping=updated.mappings.find(m=>m.id===id)
    if(mapping){mapping.username=username;mapping.passwordHash='protected'}
    const draft=config.value.mappings.find(m=>m.id===id)
    if(draft){draft.username=username;draft.passwordHash='protected'}
    saved.value=updated
    restart.value ||= wasRunning
    show(wasRunning?'密码已保存，共享已停止':'密码已保存','success')
    return true
  }catch(e){show(String(e));return false}finally{await poll();busy.value=false}
}
const {phase,startLabel,connectReason,startLocal}=useServiceControls({dirty,busy,local:computed(()=>state.value.local),config,
  save:async()=>{try{await persistConfig()}finally{await poll()}},
  start:async()=>{try{await call('start_local');restart.value=false;show('本地服务已启动','success')}finally{await poll()}},
  onError:error=>show(String(error)),
})
function navigate(name:string){if(busy.value||name===tab.value)return;if(dirty.value&&!confirm('有未保存的配置，切换后仍会保留。继续切换吗？'))return;tab.value=name}
let pollVersion=0
async function poll(){if(native){const version=++pollVersion;try{const next=await call<RuntimeStatus>('status');if(version===pollVersion)state.value=next}catch(e){if(version===pollVersion)show(String(e))}}}
const localUrl=computed(()=>`http://127.0.0.1:${saved.value.localPort}/`)
async function openLocal(){if(busy.value||!state.value.local)return;try{await openAddress(localUrl.value)}catch(e){show(`打开本地目录失败：${String(e)}`)}}
let timer:ReturnType<typeof setInterval>|undefined
onMounted(async()=>{dark.value=matchMedia('(prefers-color-scheme: dark)').matches;if(native)try{const current=await call<Config>('load_config');saved.value=current;config.value=structuredClone(current);await refreshKeys();await poll();ready.value=true}catch(e){show(String(e))}timer=setInterval(()=>{if(!busy.value&&!checkingPublic.value)void poll()},3000)})
onUnmounted(()=>clearInterval(timer))
</script>

<template>
  <div class="app" :class="{dark}">
    <AppToast :toast="toast" @dismiss="dismiss" />
    <aside>
      <div class="brand"><img :src="appIcon" alt="" width="32" height="32"><div>随享<small>本地内容 · 安全连接</small></div></div>
      <nav aria-label="主导航"><button v-for="[name,icon] in nav" :key="name" :class="{selected:tab===name}" :aria-current="tab===name?'page':undefined" @click="navigate(name)"><component :is="icon" :size="18" aria-hidden="true"/>{{name}}</button></nav>
      <div class="sidebar-bottom"><span class="status-dot" :class="{online:state.local}"></span>{{state.local?'本地服务运行中':'本地服务未启动'}}</div>
    </aside>
    <main>
      <header><strong>{{tab}}</strong><div class="header-actions"><span v-if="dirty" class="draft-label" role="status">有未保存修改</span><button @click="dark=!dark">{{dark?'切换亮色':'切换暗色'}}</button></div></header>
      <div class="content">
        <p v-if="!native" class="muted preview-note">界面预览模式 · 无原生后端，不能操作真实服务。请使用 Tauri 桌面客户端。</p>
        <div v-if="tab==='概览'" class="scroll-region" tabindex="0" role="region" aria-label="概览内容">
          <div class="section-head overview-heading"><div><h1>共享状态</h1><p class="muted">本机共享无需服务器。需要从外网访问时，再连接公网隧道。</p></div><span class="tag">只读访问</span></div>
          <section class="panel service-flow" aria-label="共享步骤" :aria-busy="busy">
            <article class="service-step"><span class="step-index" aria-hidden="true">01</span><div>
              <h2>本地服务 <span class="service-status" :class="`tone-${state.local?'success':'neutral'}`">{{state.local?'运行中':'未启动'}}</span></h2>
              <p class="muted" role="status">{{phase==='saving'?'正在保存配置…':phase==='starting'?'正在启动本地服务…':state.local?(dirty?'目前仍使用已保存的设置。':`${saved.mappings.filter(m=>m.enabled).length} 个目录已启用`):!hasFolders?'先选择至少一个要共享的目录。':restart?'配置已保存，可重新启动本地服务。':'启动后可在本机浏览共享目录。'}}</p>
            </div><div class="row service-actions">
              <button v-if="!state.local&&!hasFolders" class="primary" @click="navigate('目录映射')">添加共享目录</button>
              <button v-else-if="!state.local" class="primary" :disabled="busy||!ready" @click="startLocal">{{restart&&!dirty&&!busy?'重新启动本地服务':startLabel}}</button>
              <template v-else><a :href="localUrl" @click.prevent="openLocal">打开本机入口 ↗</a><button v-if="dirty" :disabled="busy" @click="save">{{saveLabel}}</button><button class="danger" :disabled="busy" @click="action('stop_all','共享已停止')">停止共享</button></template>
            </div></article>
            <article class="service-step"><span class="step-index" aria-hidden="true">02</span><div>
              <h2>SSH 隧道 <span class="service-status" :class="`tone-${statusTone(state.tunnel)}`">{{state.tunnel||'停止'}}</span></h2>
              <p class="muted">{{tunnelActive?(saved.host||'等待连接'):!config.publicOrigin?'公网访问需配置并保存公网源站。':connectReason||'连接前请确认服务器已授权，并完成主机指纹核对。'}}</p>
            </div><div class="row service-actions">
              <button v-if="tunnelActive" :disabled="busy" @click="action('disconnect','隧道已断开，本地服务继续运行')">断开隧道</button>
              <button v-else-if="!config.host||!config.keyId||!config.publicOrigin" :disabled="busy" @click="navigate('连接配置')">配置公网（可选）</button>
              <button v-else-if="!connectReason" :disabled="busy||!ready" @click="action('connect','已请求连接隧道')">连接隧道</button>
            </div></article>
            <article class="service-step"><span class="step-index" aria-hidden="true">03</span><div>
              <h2>公网验证 <span class="service-status" :class="`tone-${statusTone(state.public)}`">{{state.public||'未检查'}}</span></h2>
              <p class="muted">{{state.publicCheckedAt?`上次验证：${publicChecked}`:'隧道连接后自动验证，结果不代表持续可达。'}}</p>
            </div><button v-if="state.local&&state.tunnel==='SSH进程运行中'" :disabled="checkingPublic||busy" :aria-busy="checkingPublic" title="每 10 分钟自动验证一次；也可立即手动验证" @click="checkPublic">{{checkingPublic?'正在验证…':'验证公网'}}</button></article>
          </section>
          <details class="panel sharing-help"><summary>首次使用与访问说明</summary><p class="muted">添加目录并启动本地服务即可在本机访问。其他设备访问需为目录开启局域网；公网访问还需服务器授权、HTTPS 代理和主机指纹核对。未设置密码的目录可被能访问该服务的人读取。</p><button @click="navigate('部署助手')">查看部署助手</button></details>
        </div>
        <ConnectionPanel v-else-if="tab==='连接配置'" :config="config" :keys="keys" :dirty="dirty" :busy="busy||!ready" :running="state.local" :restart="restart&&hasFolders" :save-label="saveLabel" @save="save" @start="startLocal" @keys="navigate('密钥管理')" @deploy="navigate('部署助手')"/>
        <MappingsPanel v-else-if="tab==='目录映射'" :config="config" :saved="saved" :state="state" :dirty="dirty" :busy="busy||!ready" :restart="restart&&hasFolders" :save-label="saveLabel" :set-password="setPassword" @save="save" @start="startLocal" @error="show($event)"/>
        <KeysPanel v-else-if="tab==='密钥管理'" :keys="keys" @refresh="refreshKeys" @error="show($event)"/>
        <DeploymentPanel v-else-if="tab==='部署助手'" :config="config" @error="show($event)"/>
        <section v-else class="panel workspace-panel log-panel"><div class="workspace-heading"><h2>运行日志</h2><p class="muted">记录隧道错误，不包含私钥或目录访问密码。</p></div><pre class="scroll-region" tabindex="0" role="region" aria-label="运行日志内容">{{state.logs.join('\n')||'暂无日志'}}</pre></section>
      </div>
    </main>
  </div>
</template>

<style scoped>
.header-actions { display:flex; align-items:center; gap:8px; }
.draft-label { color:var(--secondary); margin-right:6px; }
.overview-heading h1 { margin:0 0 4px; }
.overview-heading p { margin:0; }
.service-flow { margin-top:20px; padding:0 20px; }
.service-step { display:grid; grid-template-columns:32px minmax(0,1fr) auto; align-items:center; gap:14px; padding:18px 0; }
.service-step + .service-step { border-top:1px solid var(--border); }
.step-index { font:12px ui-monospace,SFMono-Regular,monospace; color:var(--secondary); padding:8px 0; text-align:center; background:var(--muted); border-radius:6px; }
.service-step h2 { margin:0; font-size:16px; }
.service-step p { margin:4px 0 0; font-size:12px; overflow-wrap:anywhere; }
.service-step .service-status { font-size:12px; margin-left:6px; color:var(--secondary); }
.service-actions { justify-content:flex-end; }
.sharing-help { padding:14px 20px; }
.sharing-help summary { cursor:pointer; font-weight:600; }
.service-status.tone-error { color:var(--danger); }
.service-status.tone-success { color:#187348; }
.service-status.tone-pending { color:#946000; }
.dark .service-status.tone-success { color:#6dcea8; }
.dark .service-status.tone-pending { color:#e7b454; }
</style>
