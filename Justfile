mxd:
  cargo zigbuild \
    --release \
    --target x86_64-unknown-linux-musl \
    --target aarch64-unknown-linux-musl \
    --target x86_64-pc-windows-gnu \
    --bin mxd

mxa:
  cargo zigbuild \
    --release \
    --target x86_64-unknown-linux-musl \
    --target aarch64-unknown-linux-musl \
    --bin mxa