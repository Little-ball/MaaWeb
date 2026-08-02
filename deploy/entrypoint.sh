#!/bin/bash
# MaaWeb 容器入口脚本
# 若 MaaCore 缺失则尝试下载官方 Linux x86_64 运行时
set -e

CORE_DIR=/app/core_runtime
LIB_PATH="$CORE_DIR/libMaaCore.so"

if [ ! -f "$LIB_PATH" ]; then
    echo "[entrypoint] MaaCore 缺失，尝试下载官方运行时..."
    mkdir -p "$CORE_DIR"
    # 从 MAA 官方 Release 下载（x86_64）
    # 注意：这里需要根据实际可用版本调整
    LATEST_VERSION="v6.16.2"
    URL="https://github.com/MaaAssistantArknights/MaaAssistantArknights/releases/download/${LATEST_VERSION}/MAA-${LATEST_VERSION}-linux-x86_64.tar.gz"
    echo "[entrypoint] 下载 $URL"
    curl -fSL --retry 3 -o /tmp/maa.tar.gz "$URL" \
        && tar xzf /tmp/maa.tar.gz -C "$CORE_DIR" --strip-components=1 \
        && rm -f /tmp/maa.tar.gz \
        && echo "[entrypoint] MaaCore 下载完成" \
        || echo "[entrypoint] MaaCore 下载失败（WebUI 仍可显示，但任务功能不可用）"
fi

if [ ! -f "$LIB_PATH" ]; then
    # 尝试在解压后的目录中查找
    FOUND=$(find "$CORE_DIR" -name "libMaaCore.so" 2>/dev/null | head -1)
    if [ -n "$FOUND" ]; then
        echo "[entrypoint] 找到 MaaCore: $FOUND"
        LIB_PATH="$FOUND"
    fi
fi

echo "[entrypoint] 启动 MaaWeb 服务端..."
echo "[entrypoint] MaaCore: $LIB_PATH"
echo "[entrypoint] 前端: /app/web/dist"

exec /app/maaweb-server \
    --core-lib "$LIB_PATH" \
    --resource-dir "$CORE_DIR/resource" \
    --web-dir /app/web/dist \
    --bind 0.0.0.0:8080
