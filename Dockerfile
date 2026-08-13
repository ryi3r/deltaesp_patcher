FROM almalinux:9

RUN dnf install -y \
    wget clang \
    gcc gcc-c++ cmake \
    libX11-devel libXcursor-devel libXrandr-devel libXinerama-devel \
    libXi-devel mesa-libGL-devel

ENV LIBCLANG_PATH=/usr/lib64
