#!/usr/bin/env bash
# ============================================================
# MaaWeb MaaCore 更新脚本
# 用法:
#   ./update-maa.sh             # 检查并更新到最新版
#   ./update-maa.sh --check     # 仅检查版本
#   ./update-maa.sh --arch aarch64  # 指定架构（默认自动检测）
#   ./update-maa.sh --dir /opt/maaweb/core_runtime  # 指定目录
# ============================================================
set -euo pipefail

# 默认配置
CORE_DIR="${MAAWEB_CORE_DIR:-/opt/maaweb/core_runtime}"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) DOWNLOAD_ARCH="x86_64" ;;
  aarch64|arm64) DOWNLOAD_ARCH="aarch64" ;;
  *) echo "❌ 不支持的架构: $ARCH" >&2; exit 1 ;;
esac
CHECK_ONLY=0

# 解析参数
while [[ $# -gt 0 ]]; do
  case "$1" in
    --check) CHECK_ONLY=1; shift ;;
    --arch) DOWNLOAD_ARCH="$2"; shift 2 ;;
    --dir) CORE_DIR="$2"; shift 2 ;;
    *) echo "未知参数: $1" >&2; exit 1 ;;
  esac
done

echo "=============================================="
echo "  MaaWeb MaaCore 更新工具"
echo "  架构: $DOWNLOAD_ARCH"
echo "  目录: $CORE_DIR"
echo "=============================================="

# 检查依赖
for cmd in curl tar jq; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "❌ 缺少依赖: $cmd" >&2
    exit 1
  fi
done

# 获取最新版本信息
echo "📡 查询 MAA 最新版本..."
API_URL="https://api.github.com/repos/MaaAssistantArknights/MaaAssistantArknights/releases/latest"
RELEASE_JSON="$(curl -sL --max-time 20 -H 'Accept: application/vnd.github+json' "$API_URL")"

LATEST_TAG="$(echo "$RELEASE_JSON" | jq -r '.tag_name' 2>/dev/null || echo '')"
if [[ -z "$LATEST_TAG" || "$LATEST_TAG" == "null" ]]; then
  echo "❌ 无法获取最新版本（网络或 API 问题）" >&2
  exit 1
fi

# 查找下载 URL
DOWNLOAD_URL="$(echo "$RELEASE_JSON" | jq -r --arg arch "$DOWNLOAD_ARCH" \
  '.assets[] | select(.name | contains("linux-" + $arch)) | select(.name | endswith(".tar.gz")) | .browser_download_url' \
  2>/dev/null | head -1)"
if [[ -z "$DOWNLOAD_URL" || "$DOWNLOAD_URL" == "null" ]]; then
  echo "❌ 未找到 $DOWNLOAD_ARCH 架构的 tar.gz 资产" >&2
  exit 1
fi

# 本地版本
LOCAL_VERSION=""
if [[ -f "$CORE_DIR/resource/version.json" ]]; then
  LOCAL_VERSION="$(cat "$CORE_DIR/resource/version.json" | jq -r '.version // empty' 2>/dev/null || echo '')"
fi
if [[ -z "$LOCAL_VERSION" ]]; then
  LOCAL_VERSION="未知"
fi

echo "  本地版本: $LOCAL_VERSION"
echo "  最新版本: $LATEST_TAG"
echo "  下载地址: $DOWNLOAD_URL"

if [[ "$CHECK_ONLY" == "1" ]]; then
  if [[ "$LOCAL_VERSION" != "未知" && "$LOCAL_VERSION" != "$LATEST_TAG" ]]; then
    echo "🔄 检测到新版本，可执行更新"
  else
    echo "✅ 已是最新版本"
  fi
  exit 0
fi

# 确认更新
read -r -p "确认更新到 $LATEST_TAG? [y/N] " CONFIRM
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
  echo "已取消"
  exit 0
fi

# 下载
echo "📥 下载中..."
TMP_TAR="$CORE_DIR/_update_download.tar.gz"
TMP_DIR="$CORE_DIR/_update_tmp"
rm -f "$TMP_TAR"
rm -rf "$TMP_DIR"
mkdir -p "$CORE_DIR" "$TMP_DIR"

if ! curl -fSL --retry 3 --max-time 300 -o "$TMP_TAR" "$DOWNLOAD_URL"; then
  echo "❌ 下载失败" >&2
  exit 1
fi

# 解压
echo "📦 解压中..."
if ! tar xzf "$TMP_TAR" -C "$TMP_DIR"; then
  echo "❌ 解压失败" >&2
  exit 1
fi

# 找到 libMaaCore.so
LIB_PATH="$(find "$TMP_DIR" -name 'libMaaCore.so' | head -1)"
if [[ -z "$LIB_PATH" ]]; then
  echo "❌ 下载包中未找到 libMaaCore.so" >&2
  exit 1
fi
EXTRACT_ROOT="$(dirname "$LIB_PATH")"

# 备份旧版本
if [[ -f "$CORE_DIR/libMaaCore.so" ]]; then
  echo "💾 备份旧版本..."
  mkdir -p "$CORE_DIR/_backup"
  cp "$CORE_DIR/libMaaCore.so" "$CORE_DIR/_backup/libMaaCore.so"
fi

# 替换文件
echo "🔁 替换文件..."
cp -r "$EXTRACT_ROOT"/. "$CORE_DIR/"

# 清理临时文件
rm -rf "$TMP_DIR" "$TMP_TAR"

echo ""
echo "✅ 更新完成！"
echo "  新版本: $LATEST_TAG"
echo "  ⚠️  重启 MaaWeb 服务后生效"
echo ""
echo "  重启命令示例:"
echo "  pkill -f maaweb-server"
echo "  cd /opt/maaweb && nohup env LD_LIBRARY_PATH=/opt/maaweb/core_runtime \\"
echo "    ./server/maaweb-server --core-lib core_runtime/libMaaCore.so \\"
echo "    --resource-dir core_runtime --user-dir core_runtime \\"
echo "    --web-dir web/dist --bind 0.0.0.0:18080 &"
