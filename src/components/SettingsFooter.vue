<script setup lang="ts">
defineProps<{label:string;dirty:boolean;busy:boolean;running:boolean;restart:boolean}>()
defineEmits<{save:[];start:[]}>()
</script>

<template>
  <div class="workspace-footer">
    <button class="primary" :disabled="busy||!dirty" @click="$emit('save')">{{label}}</button>
    <button v-if="restart&&!running&&!dirty" :disabled="busy" @click="$emit('start')">重新启动本地服务</button>
    <p class="muted" role="status">{{dirty?(running?'保存会同时停止本地服务和 SSH 隧道。':'有未保存修改，保存后才会生效。'):restart&&!running?'配置已保存。公网隧道需手动连接。':'没有未保存的修改'}}</p>
  </div>
</template>
