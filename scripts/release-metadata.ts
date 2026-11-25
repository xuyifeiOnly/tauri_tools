//! 发布元数据：校验三处版本一致、格式合法，并派生 Release 标签。
//!
//! 手动触发（非标签 ref）使用源码版本生成 `vX.Y.Z`；标签触发要求标签与源码版本完全一致。
//! 任何不一致都在构建前失败，绝不自动改版本。

export interface ReleaseMetadata {
  version: string
  tag: string
  sha: string
}

const VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/

/**
 * @param versions 依次为 package.json、tauri.conf.json、Cargo.toml 的版本号
 * @param ref GitHub 的 `GITHUB_REF`（如 `refs/tags/v0.1.2`、`refs/heads/main`）
 * @param sha 本轮源码 commit
 */
export function releaseMetadata(
  versions: string[],
  ref: string,
  sha: string,
): ReleaseMetadata {
  if (versions.length !== 3 || !versions.every((value) => value === versions[0])) {
    throw new Error(
      '三处版本号不一致：package.json、tauri.conf.json、Cargo.toml 必须相同',
    )
  }
  const version = versions[0]
  if (!VERSION_PATTERN.test(version)) {
    throw new Error(`版本号必须为 X.Y.Z 格式，实际为 ${JSON.stringify(version)}`)
  }
  const tag = `v${version}`
  if (ref.startsWith('refs/tags/')) {
    const pushed = ref.slice('refs/tags/'.length)
    if (pushed !== tag) {
      throw new Error(`标签 ${pushed} 与源码版本 ${tag} 不一致`)
    }
  }
  return { version, tag, sha }
}

if (import.meta.main) {
  const { readFileSync, appendFileSync } = await import('node:fs')
  const read = (path: string) => readFileSync(path, 'utf8')
  const pkgVersion = JSON.parse(read('package.json')).version as string
  const tauriVersion = JSON.parse(read('src-tauri/tauri.conf.json')).version as string
  const cargoVersion =
    read('src-tauri/Cargo.toml').match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? ''

  const metadata = releaseMetadata(
    [pkgVersion, tauriVersion, cargoVersion],
    process.env.GITHUB_REF ?? '',
    process.env.GITHUB_SHA ?? '',
  )

  const output = process.env.GITHUB_OUTPUT
  if (output) {
    appendFileSync(
      output,
      `version=${metadata.version}\ntag=${metadata.tag}\nsha=${metadata.sha}\n`,
    )
  }
  console.log(JSON.stringify(metadata))
}
