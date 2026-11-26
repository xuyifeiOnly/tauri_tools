import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const pattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/

/** 先验证并生成全部内容，避免输入或文件格式错误时只更新部分文件。 */
export function versionUpdates(texts: string[], explicit?: string) {
  const versions = texts.slice(0, 2).map(text => JSON.parse(text).version)
  const section = /^\[package\][^\S\r\n]*\r?\n([\s\S]*?)(?=^\[|$(?![\s\S]))/m.exec(texts[2])
  const cargo = section && /^version\s*=\s*"([^"]+)"/m.exec(section[1])
  if (!section || !cargo) throw new Error('Cargo.toml 缺少 [package] version')
  versions.push(cargo[1])
  if (versions.some(version => typeof version !== 'string' || !pattern.test(version))) {
    throw new Error('当前版本必须为 X.Y.Z 稳定版')
  }
  if (!explicit && new Set(versions).size !== 1) {
    throw new Error('三处版本不一致，请用 --version X.Y.Z 显式统一')
  }
  const parts = versions[0].split('.')
  const target = explicit ?? `${parts[0]}.${parts[1]}.${BigInt(parts[2]) + 1n}`
  if (!pattern.test(target)) throw new Error('指定版本必须为 X.Y.Z 稳定版')
  const updated = texts.slice(0, 2).map((text, index) => {
    const regex = /("version"\s*:\s*")([^"]+)(")/g
    const matches = [...text.matchAll(regex)]
    if (matches.length !== 1 || matches[0][2] !== versions[index]) {
      throw new Error('JSON version 匹配不唯一，未修改文件')
    }
    return text.replace(regex, (_match, prefix, _version, suffix) => `${prefix}${target}${suffix}`)
  })
  const start = section.index + section[0].indexOf(section[1]) + cargo.index + cargo[0].indexOf('"') + 1
  updated.push(texts[2].slice(0, start) + target + texts[2].slice(start + cargo[1].length))
  return { versions, target, updated }
}

if (import.meta.main) {
  try {
    const args = process.argv.slice(2)
    let explicit: string | undefined
    let dryRun = false
    for (let i = 0; i < args.length; i++) {
      if (args[i] === '--dry-run') dryRun = true
      else if (args[i] === '--version' && args[i + 1] && !explicit) explicit = args[++i]
      else if (args[i] === '--help' || args[i] === '-h') {
        console.log('用法：bun scripts/bump-version.ts [--version X.Y.Z] [--dry-run]\n默认补丁版本 +1，只同步版本，不打包。')
        process.exit(0)
      } else throw new Error(`未知选项或缺少参数：${args[i]}`)
    }
    const root = fileURLToPath(new URL('../', import.meta.url))
    const paths = ['package.json', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml'].map(path => resolve(root, path))
    const texts = paths.map(path => readFileSync(path, 'utf8'))
    const { versions, target, updated } = versionUpdates(texts, explicit)
    if (!dryRun) {
      const written: number[] = []
      try {
        paths.forEach((path, index) => {
          if (texts[index] === updated[index]) return
          written.push(index)
          writeFileSync(path, updated[index])
        })
      } catch (error) {
        for (const index of written.reverse()) writeFileSync(paths[index], texts[index])
        throw error
      }
    }
    console.log(`版本：${versions.join(' / ')} → ${target}${dryRun ? '（预览，不写入）' : ''}`)
  } catch (error) {
    console.error(`错误：${error instanceof Error ? error.message : String(error)}`)
    process.exitCode = 1
  }
}
