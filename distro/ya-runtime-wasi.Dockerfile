FROM rust:1.46

RUN  cargo install \
	--git https://github.com/golemfactory/ya-runtime-wasi.git --branch prekucki/sgx-profile \
	--features sgx \
	ya-runtime-wasi-cli --bin ya-runtime-wasi

