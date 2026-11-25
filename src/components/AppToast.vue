<script setup lang="ts">
import { Check, CircleAlert, Info, X } from 'lucide-vue-next'
import type { ToastKind } from '../composables/useToast'
defineProps<{ toast: { message: string; kind: ToastKind } | null }>()
defineEmits<{ dismiss: [] }>()
</script>

<template>
  <div class="toast-host">
    <div v-if="toast" class="app-toast" :class="toast.kind" :role="toast.kind==='error'?'alert':'status'" aria-atomic="true">
      <component :is="toast.kind==='error'?CircleAlert:toast.kind==='success'?Check:Info" :size="18" class="toast-icon" aria-hidden="true" />
      <span>{{toast.message}}</span>
      <button class="toast-dismiss" aria-label="关闭提示" @click="$emit('dismiss')"><X :size="16" aria-hidden="true" /></button>
    </div>
  </div>
</template>

<style scoped>
.toast-host { position: fixed; bottom: 28px; left: 50%; transform: translateX(-50%); z-index: 100; max-width: min(520px, calc(100vw - 48px)); width: max-content; pointer-events: none; }
.app-toast { display: flex; align-items: center; gap: 10px; padding: 10px 12px 10px 16px; color: var(--text); background: var(--surface); border: 1px solid var(--border); border-radius: 9px; box-shadow: 0 6px 24px #0002; pointer-events: auto; overflow-wrap: anywhere; }
.toast-icon { color: var(--accent); flex-shrink: 0; }
.error .toast-icon { color: var(--danger); }
.toast-dismiss { display: flex; align-items: center; justify-content: center; min-width: 36px; min-height: 36px; padding: 6px; border: 0; flex-shrink: 0; }
</style>
