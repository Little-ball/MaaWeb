# MaaWeb 容器镜像
# 基于 Ubuntu 24.04（glibc 2.39，MaaCore v6.16.2 要求 glibc >= 2.38）
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# 基础依赖：MaaCore 需要 libatomic、libstdc++、libgomp、opencv 相关库等
RUN apt-get update && apt-get install -y --no-install-recommends \
    libstdc++6 \
    libgomp1 \
    libatomic1 \
    ca-certificates \
    curl \
    adb \
    && rm -rf /var/lib/apt/lists/*

# MaaCore 依赖其兄弟动态库（libMaaUtils.so 等），需通过 LD_LIBRARY_PATH 指定
ENV LD_LIBRARY_PATH=/app/core_runtime:${LD_LIBRARY_PATH}

# 前端静态文件
COPY web/dist/ /app/web/dist/

# 后端二进制
COPY server/maaweb-server /app/maaweb-server
RUN chmod +x /app/maaweb-server

# MaaCore 运行时（挂载卷，不复制进镜像）

EXPOSE 8080

# 直接启动服务端，core_runtime 通过卷挂载到 /app/core_runtime
# 注意：resource-dir 必须指向包含 resource/ 子目录的根目录（MaaCore 要求）
CMD ["/app/maaweb-server", \
     "--core-lib", "/app/core_runtime/libMaaCore.so", \
     "--resource-dir", "/app/core_runtime", \
     "--user-dir", "/app/core_runtime", \
     "--web-dir", "/app/web/dist", \
     "--bind", "0.0.0.0:8080"]
