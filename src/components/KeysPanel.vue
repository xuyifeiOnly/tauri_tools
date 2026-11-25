<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'
import { useCopyFeedback } from '../composables/useCopyFeedback'
import type {KeyInfo} from '../types'
import {call} from '../api'
import { useToast } from '../composables/useToast'
const { show } = useToast()
defineProps<{keys:KeyInfo[]}>()
const emit=defineEmits<{refresh:[key:KeyInfo];error:[message:string]}>()
async function create(imported=false){try{let source=null;if(imported){source=await open({multiple:false});if(!source)return}const key=await call<KeyInfo>('generate_key',{source});emit('refresh',key);show('密钥已生成并选用于当前连接，请保存连接配置','info')}catch(e){emit('error',String(e))}}
const {copy}=useCopyFeedback(message=>emit('error',message))
</script>
<template>
  <section class="panel workspace-panel">
    <div class="workspace-heading"><h2>密钥管理</h2><p class="muted">每台电脑使用独立密钥。私钥保存在应用数据目录，不上传服务器。当前版本支持无口令私钥。</p>
    <div class="row"><button class="primary" @click="create()">生成 Ed25519 密钥</button><button @click="create(true)">导入私钥</button></div>
    </div>
    <div class="scroll-region" tabindex="0" role="region" aria-label="密钥列表">
    <div v-if="!keys.length" class="empty">尚无密钥</div>
    <article v-for="key in keys" :key="key.id" class="key">
      <strong>{{key.fingerprint}}</strong>
      <div class="key-path"><span>私钥位置</span><code>{{key.privateKeyPath}}</code><button @click="copy(key.privateKeyPath,'私钥路径已复制')">复制路径</button></div>
      <div class="key-path"><span>公钥位置</span><code>{{key.publicKeyPath}}</code><button @click="copy(key.publicKeyPath,'公钥路径已复制')">复制路径</button></div>
      <pre>{{key.publicKey}}</pre><button @click="copy(key.publicKey,'公钥已复制')">复制公钥</button>
    </article>
    </div>
  </section>
</template>
<style scoped>
.key-path { display: grid; grid-template-columns: 64px minmax(0,1fr) auto; align-items: center; gap: 12px; margin-top: 12px; }
.key-path code { overflow-wrap: anywhere; font-size: 12px; color: var(--secondary); }
</style>
