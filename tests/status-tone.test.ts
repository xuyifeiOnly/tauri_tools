import { expect, test } from 'bun:test'
import { statusTone } from '../src/utils/statusTone'

test('errors are red, running and reachable green, transitions amber, idle neutral', () => {
  expect(['主机身份验证失败，已停止','未验证 / 不可达','连接错误'].map(statusTone)).toEqual(['error','error','error'])
  expect(['运行中','SSH进程运行中','可访问'].map(statusTone)).toEqual(['success','success','success'])
  expect(['连接中','重连中','重连等待'].map(statusTone)).toEqual(['pending','pending','pending'])
  expect(['未启动','停止','未检查',''].map(statusTone)).toEqual(['neutral','neutral','neutral','neutral'])
})
