<script setup lang="ts">
import {ref,watch} from 'vue'
import {Check} from 'lucide-vue-next'
import { useCopyFeedback } from '../composables/useCopyFeedback'
import type{Config}from '../types'
import{call}from '../api'
import { useToast } from '../composables/useToast'
const { show } = useToast()
const props=defineProps<{config:Config}>();const emit=defineEmits<{error:[message:string]}>();const snippets=ref<{authorization:string;nginx:string}|null>(null);const fingerprint=ref('')
async function generate(){if(!props.config.keyId){show('请先在连接配置中选择电脑私钥');return}try{snippets.value=await call('deployment',{config:props.config});show('部署脚本已生成','success')}catch(e){emit('error',String(e))}}
const trusted=ref(false),trusting=ref(false),scanning=ref(false)
let revision=0
watch(()=>[props.config.host,props.config.sshPort],()=>{revision++;fingerprint.value='';trusted.value=false})
async function scan(){if(scanning.value||trusting.value)return;const current=++revision;trusted.value=false;fingerprint.value='';scanning.value=true;try{const value=await call<string>('scan_host');if(current===revision)fingerprint.value=value}catch(e){emit('error',String(e))}finally{scanning.value=false}}
async function trust(){if(trusting.value||trusted.value||!fingerprint.value)return;const current=revision;trusting.value=true;try{await call('trust_host');if(current===revision){trusted.value=true;show('主机公钥已保存','success')}}catch(e){emit('error',String(e))}finally{trusting.value=false}}
const {copy}=useCopyFeedback(message=>emit('error',message))
</script>
<template>
<section class="panel workspace-panel">
  <div class="workspace-heading"><h2>部署助手</h2><p class="muted">以下操作由你在服务器执行。客户端不会自动修改服务器或保存 root 密码。</p><button class="primary" @click="generate">根据当前配置生成脚本</button></div>
  <div class="scroll-region" tabindex="0" role="region" aria-label="部署内容">
    <template v-if="snippets"><h3>1 · 公钥授权</h3><p class="muted">仅追加公钥。目标隧道账号必须预先创建并配置 SSH 转发权限。</p><pre tabindex="0" aria-label="公钥授权脚本">{{snippets.authorization}}</pre><button @click="copy(snippets.authorization)">复制授权脚本</button><h3>2 · HTTPS Nginx 配置</h3><pre tabindex="0" aria-label="Nginx 配置">{{snippets.nginx}}</pre><button @click="copy(snippets.nginx)">复制 Nginx 配置</button></template>
    <h3>3 · 核对服务器身份</h3><p class="muted">先保存服务器配置。扫描结果不代表可信，请与服务器控制台的主机指纹比对后确认。指纹改变时不要直接信任。</p><button :disabled="scanning||trusting" @click="scan">{{scanning?'正在读取指纹…':'读取服务器指纹'}}</button><pre v-if="fingerprint" tabindex="0" aria-label="服务器指纹">{{fingerprint}}</pre><button v-if="fingerprint" class="trust-button" :class="{'is-trusted':trusted}" :disabled="trusting||trusted" @click="trust"><Check v-if="trusted" :size="16" aria-hidden="true"/>{{trusted?'已确认信任':trusting?'正在保存信任…':'已通过独立渠道核对，确认信任'}}</button>
  </div>
</section></template>

<style scoped>
.trust-button { display:inline-flex; align-items:center; gap:6px; }
.trust-button.is-trusted:disabled { appearance:none; -webkit-appearance:none; color:#187348; background:#eaf6ef; border-color:#8fc6a5; opacity:1; cursor:default; }
.dark .trust-button.is-trusted:disabled { color:#8ce0b5; background:#163b2b; border-color:#397c59; }
</style>
