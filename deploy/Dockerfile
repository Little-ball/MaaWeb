# MaaWeb 容器镜像
# 包含后端二进制 + 前端静态文件 + MaaCore 运行时
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# 基础依赖（MaaCore 需要 glibc、libstdc++ 等）
RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 \
    libgomp1 \
    ca-certificates \
    curl \
    adb \
    && rm -rf /var/lib/apt/lists/*

# 部署目录结构
WORKDIR /app

# 前端静态文件
COPY web/dist/ /app/web/dist/

# 后端二进制
COPY server/target/release/maaweb-server /app/maaweb-server

# MaaCore 运行时（构建时若存在则复制）
COPY core_runtime/ /app/core_runtime/

# 启动脚本
COPY deploy/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

EXPOSE 8080

ENTRYPOINT ["/app/entrypoint.sh"]
