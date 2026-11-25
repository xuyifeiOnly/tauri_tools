import { expect, test } from 'bun:test'
import { fileKind, fileLink, shareBlockReason } from '../src/utils/fileLinks'
import { defaults } from '../src/types'
const mapping = { id:'a', name:'a', localPath:'/', route:'docs', enabled:true, allowLan:true, username:'', passwordHash:'' }
const config = {...defaults, publicOrigin:'https://example.com/'}
const state = {local:true, public:'可访问'}
test('classifies extensions case-insensitively with safe fallback', () => {
  expect(['X.HTML','x.pdf','x.png','x.docx','x.xlsx','x.zip','x.mp4','x.mp3','x.unknown'].map(fileKind)).toEqual(['code','pdf','image','document','sheet','archive','video','audio','file'])
})
test('independent LAN and public links encode path segments', () => {
  expect(fileLink('lan',config,mapping,state,'','192.168.1.2')).toBe('http://192.168.1.2:20005/docs/')
  expect(fileLink('lan',{...config,localPort:21000},{...mapping,route:'测试 #'},state,'','10.0.0.2')).toBe('http://10.0.0.2:21000/%E6%B5%8B%E8%AF%95%20%23/')
  expect(fileLink('lan',config,mapping,state,'目录/a #.html','192.168.1.2')).toBe('http://192.168.1.2:20005/docs/%E7%9B%AE%E5%BD%95/a%20%23.html')
  expect(fileLink('public',config,{...mapping,allowLan:false},state,'index.html')).toBe('https://example.com/my_mac/docs/index.html')
  expect(fileLink('lan',config,mapping,{...state,public:'未检查'},'a','192.168.1.2')).toContain('192.168.1.2')
})
test('rejects unavailable channels and missing prerequisites', () => {
  const link = (channel:'lan'|'public', c=config,m=mapping,s=state,ip='192.168.1.2') => fileLink(channel,c,m,s,'a',ip)
  expect(()=>link('lan',config,{...mapping,enabled:false})).toThrow('未启用')
  expect(()=>link('public',config,mapping,{...state,local:false})).toThrow('启动')
  expect(()=>link('lan',config,{...mapping,allowLan:false})).toThrow('局域网')
  expect(()=>link('lan',config,mapping,state,'')).toThrow('IPv4')
  expect(()=>link('public',{...config,publicOrigin:''})).toThrow('公网源站')
  expect(()=>link('public',config,mapping,{...state,public:'未检查'})).toThrow('可达')
})
test('local address stays on loopback and independent of LAN or public settings', () => {
  expect(fileLink('local',defaults,{...mapping,allowLan:false}, {local:true,public:'未检查'},'')).toBe('http://127.0.0.1:20005/docs/')
  expect(shareBlockReason('local', defaults, undefined, state)).toBe('请先保存此目录')
  expect(shareBlockReason('public', config, mapping, {local:true,public:'未检查'})).toContain('尚未确认')
})
