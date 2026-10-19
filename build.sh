
export PYO3_PYTHON="$(command -v python)"
export PYO3_BUILD_EXTENSION_MODULE=1

cargo build --release \
    --features pyo3/extension-module \
    --target armv7-linux-androideabi
