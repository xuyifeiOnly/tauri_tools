<script setup lang="ts">
import type { Config,KeyInfo } from '../types'
import { ref,onMounted } from 'vue'
import { enable,disable,isEnabled } from '@tauri-apps/plugin-autostart'
import { native } from '../api'
import { useToast } from '../composables/useToast'
import SettingsFooter from './SettingsFooter.vue'
const { show } = useToast()
const autostart=ref(false)
onMounted(async()=>{if(native)try{autostart.value=await isEnabled()}catch(e){show(String(e))}})
async function toggleAutostart(){try{if(!native)throw new Error('请在桌面客户端中设置');if(autostart.value)await disable();else await enable();autostart.value=await isEnabled();show('设置已更新；登录后启动客户端，服务仍需手动启动','info')}catch(e){show(String(e))}}
defineProps<{config:Config;keys:KeyInfo[];dirty:boolean;busy:boolean;running:boolean;restart:boolean;saveLabel:string}>()
defineEmits<{save:[];start:[];keys:[];deploy:[]}>()
</script>
<template>
  <section class="panel workspace-panel connection-panel">
    <div class="workspace-heading"><h2>连接配置</h2><p class="muted">只在本机或局域网共享时，无需配置公网连接。</p></div>
    <div class="scroll-region" tabindex="0" role="region" aria-label="连接配置表单">
      <h3>本地服务</h3>
      <div class="fields"><label>本地监听端口<input v-model.number="config.localPort" type="number" :disabled="busy"><small>默认仅本机可访问。局域网访问在目录映射中单独开启。</small></label></div>
      <hr><h3>公网连接 <small class="optional-label">可选</small></h3>
      <div class="fields">
        <label>SSH 服务器<input v-model="config.host" :disabled="busy" placeholder="example.com"></label>
        <label>隧道账号<input v-model="config.user" :disabled="busy"></label>
        <label>电脑私钥<select v-model="config.keyId" :disabled="busy"><option value="">请选择密钥</option><option v-for="key in keys" :key="key.id" :value="key.id">{{key.fingerprint}}</option></select></label>
        <label>公网源站<input v-model="config.publicOrigin" :disabled="busy" placeholder="https://example.com"></label>
      </div>
      <div class="row settings-links"><button v-if="!config.keyId" @click="$emit('keys')">前往密钥管理</button><button @click="$emit('deploy')">打开部署助手</button></div>
      <details><summary>高级设置 <small class="optional-label">SSH 端口、转发端口、访问前缀</small></summary><div class="fields">
        <label>SSH 端口<input v-model.number="config.sshPort" :disabled="busy" type="number"></label>
        <label>服务器转发端口<input v-model.number="config.remotePort" :disabled="busy" type="number"><small>每台电脑使用独立端口，避免冲突。</small></label>
        <label>公网访问前缀<input v-model="config.basePath" :disabled="busy" placeholder="/my_mac"></label>
      </div></details>
      <hr><h3>后台运行</h3><p class="muted">关闭窗口隐藏至菜单栏，使用菜单栏“停止服务并退出”结束客户端。</p>
      <button @click="toggleAutostart">{{autostart?'关闭登录启动':'开启登录启动'}}</button>
    </div>
    <SettingsFooter :label="saveLabel" :dirty="dirty" :busy="busy" :running="running" :restart="restart" @save="$emit('save')" @start="$emit('start')"/>
  </section>
</template>

<style scoped>
.connection-panel .fields { gap:12px 20px; margin:12px 0 16px; }
.connection-panel summary { cursor:pointer; font-weight:600; }
.optional-label { display:inline; margin-left:6px; font-weight:400; }
.settings-links { margin-bottom:12px; }
.connection-panel .fields label { align-content:start; overflow-wrap:anywhere; }
.connection-panel select { width:100%; min-width:0; }
.connection-panel hr { border:0; border-top:1px solid var(--border); margin:20px 0 16px; }
.connection-panel h3 { margin:0 0 8px; }
.connection-panel .scroll-region > p { margin:0 0 12px; }
</style>
