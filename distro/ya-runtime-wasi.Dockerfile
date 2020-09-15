FROM rust:1.46

ARG WASI_VERSION
RUN  cargo install \
	--git https://github.com/golemfactory/ya-runtime-wasi.git --rev $WASI_VERSION \
	--features sgx \
	ya-runtime-wasi-cli --bin ya-runtime-wasi

