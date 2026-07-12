FROM ghcr.io/cross-rs/x86_64-unknown-linux-gnu:edge

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    wget ca-certificates \
    libc6-dev \
    libclang-14-dev \
    libx11-dev libxcursor-dev libxrandr-dev libxinerama-dev \
    libxi-dev libgl1-mesa-dev && \
    wget -q https://github.com/Kitware/CMake/releases/download/v3.31.7/cmake-3.31.7-linux-x86_64.tar.gz -O /tmp/cmake.tar.gz && \
    tar -xzf /tmp/cmake.tar.gz -C /opt/ && \
    ln -sf /opt/cmake-3.31.7-linux-x86_64/bin/* /usr/local/bin/

ENV LIBCLANG_PATH=/usr/lib/llvm-14/lib
