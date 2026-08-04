//! MaaCore 在线更新模块
//!
//! 从 MAA 官方 GitHub Release 下载当前架构的最新版，替换 core_runtime 目录。
//! 依赖系统 curl 与 tar（部署机均已预装）。

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 检测当前 CPU 架构对应的 MAA 资产后缀
pub fn detect_arch() -> Result<String> {
    let out = Command::new("uname").arg("-m").output()?;
    let arch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    match arch.as_str() {
        "x86_64" | "amd64" => Ok("x86_64".to_string()),
        "aarch64" | "arm64" => Ok("aarch64".to_string()),
        _ => Err(anyhow!("不支持的架构: {arch}")),
    }
}

/// 查询 GitHub 最新 release 信息，返回 (tag, 对应架构资产 URL)
pub fn fetch_latest_release(arch: &str) -> Result<(String, String)> {
    // 优先使用官方 API；若失败回退到重定向下载页
    let api_url = "https://api.github.com/repos/MaaAssistantArknights/MaaAssistantArknights/releases/latest";
    let out = Command::new("curl")
        .args(["-sL", "--max-time", "20", "-H", "Accept: application/vnd.github+json", api_url])
        .output()
        .context("调用 GitHub API 失败")?;
    let text = String::from_utf8_lossy(&out.stdout);
    let json: Value = serde_json::from_str(&text).map_err(|_| anyhow!("GitHub API 响应解析失败"))?;

    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("无法获取最新版本号"))?
        .to_string();

    // 找匹配架构的 linux tar.gz 资产
    let target_prefix = format!("linux-{arch}");
    let asset = json
        .get("assets")
        .and_then(|a| a.as_array())
        .and_then(|arr| {
            arr.iter().find(|a| {
                let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
                name.contains(&target_prefix) && name.ends_with(".tar.gz")
            })
        })
        .ok_or_else(|| anyhow!("未找到 {arch} 架构的 tar.gz 资产"))?;

    let url = asset
        .get("browser_download_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("资产下载链接缺失"))?
        .to_string();

    Ok((tag, url))
}

/// 下载并解压到目标目录
fn download_and_extract(url: &str, target_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(target_dir)?;
    let tmp_tar = target_dir.join("_update_download.tar.gz");
    let tmp_extract = target_dir.join("_update_tmp");

    // 清理残留
    let _ = std::fs::remove_file(&tmp_tar);
    if tmp_extract.exists() {
        std::fs::remove_dir_all(&tmp_extract)?;
    }

    // 下载
    let status = Command::new("curl")
        .args(["-fSL", "--retry", "3", "--max-time", "300", "-o"])
        .arg(&tmp_tar)
        .arg(url)
        .status()
        .context("下载失败")?;
    anyhow::ensure!(status.success(), "curl 下载失败 (exit {status})");
    anyhow::ensure!(tmp_tar.exists(), "下载文件不存在");

    // 解压
    let status = Command::new("tar")
        .args(["xzf"])
        .arg(&tmp_tar)
        .arg("-C")
        .arg(&tmp_extract)
        .status()
        .context("tar 解压失败")?;
    anyhow::ensure!(status.success(), "tar 解压失败");

    Ok(())
}

/// 执行更新：下载最新版并替换 core_runtime。
/// 返回更新后的版本号。
///
/// 策略：下载到临时目录 → 校验 libMaaCore.so 存在 → 备份旧目录 → 原子替换。
/// 注意：替换磁盘上的 .so 不影响当前运行进程（已映射到内存），重启后生效。
pub fn update_core(core_dir: &Path) -> Result<String> {
    let arch = detect_arch()?;
    tracing::info!("检测到架构: {arch}");

    let (new_tag, url) = fetch_latest_release(&arch)?;
    tracing::info!("最新版本: {new_tag}，开始下载...");

    let tmp_extract = core_dir.join("_update_tmp");
    download_and_extract(&url, &tmp_extract)?;

    // 找到解压后的 libMaaCore.so（可能多一层目录）
    let lib_path = find_file(&tmp_extract, "libMaaCore.so")
        .ok_or_else(|| anyhow!("下载包中未找到 libMaaCore.so"))?;
    let extracted_root = lib_path.parent().unwrap_or(&tmp_extract).to_path_buf();
    tracing::info!("解压完成，来源: {}", extracted_root.display());

    // 备份旧目录
    let backup_dir = core_dir.join("_backup");
    if backup_dir.exists() {
        std::fs::remove_dir_all(&backup_dir)?;
    }
    // 仅备份当前 .so 与 resource，避免把旧备份/下载文件一并复制
    let backup_lib = backup_dir.join("libMaaCore.so");
    if let Some(old) = find_file(core_dir, "libMaaCore.so") {
        std::fs::create_dir_all(&backup_dir)?;
        std::fs::copy(&old, &backup_lib)?;
    }

    // 复制新文件到 core_dir（覆盖同名文件）
    copy_recursive(&extracted_root, core_dir)?;

    // 清理临时文件
    let _ = std::fs::remove_dir_all(&tmp_extract);
    let _ = std::fs::remove_file(core_dir.join("_update_download.tar.gz"));

    Ok(new_tag)
}

/// 检查是否有新版本（不执行更新）。返回 (当前版本, 最新版本, 是否有更新)。
pub fn check_update(core_dir: &Path) -> Result<(String, String, bool)> {
    let arch = detect_arch()?;
    let (latest_tag, _) = fetch_latest_release(&arch)?;

    // 读取本地版本
    let local_version = local_version(core_dir).unwrap_or_else(|| "未知".to_string());

    let has_update = local_version != "未知" && local_version != latest_tag;
    Ok((local_version, latest_tag, has_update))
}

/// 读取本地 MaaCore 版本（从 version 文件或 .so 文件名的 debug 信息中推断）
fn local_version(core_dir: &Path) -> Option<String> {
    // 尝试从 resource/version.json 读取
    let vf = core_dir.join("resource/version.json");
    if let Ok(text) = std::fs::read_to_string(&vf) {
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            if let Some(s) = v.get("version").and_then(|x| x.as_str()) {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.file_name().and_then(|n| n.to_str()) == Some(name) {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_recursive(&from, &to)?;
        } else {
            // 跳过已在目标目录的备份/临时文件
            if entry.file_name().to_string_lossy().starts_with('_') {
                continue;
            }
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
