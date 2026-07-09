#!/bin/bash

export CMAKE_PREFIX_PATH=/usr/x86_64-w64-mingw32
export PKG_CONFIG_PATH=/usr/x86_64-w64-mingw32/lib/pkgconfig
cargo +stable build --target x86_64-pc-windows-gnu -r
#cp /usr/x86_64-w64-mingw32/bin/SDL3.dll ./target/x86_64-pc-windows-gnu/release # sdl
#cp /usr/x86_64-w64-mingw32/bin/libssp-0.dll ./target/x86_64-pc-windows-gnu/release # sdl
mkdir packaged
cp data.bin ./packaged/
#cp ./target/x86_64-pc-windows-gnu/release/*.dll ./packaged/
cp ./target/x86_64-pc-windows-gnu/release/*.exe ./packaged/deltaesp_patcher.exe
cargo +stable build --target x86_64-pc-windows-gnu -r --features uninstaller
cp ./target/x86_64-pc-windows-gnu/release/*.exe ./packaged/deltaesp_uninstaller.exe
mkdir packaged/patch
rm packaged/*.dll # non sdl
cargo +stable build --target x86_64-unknown-linux-gnu -r
cp ./target/x86_64-unknown-linux-gnu/release/*_patcher ./packaged/deltaesp_patcher
cargo +stable build --target x86_64-unknown-linux-gnu -r --features uninstaller
cp ./target/x86_64-unknown-linux-gnu/release/*_patcher ./packaged/deltaesp_uninstaller
