FROM rust:1.46

RUN  OPENSSL_STATIC=yes cargo install \
	--git https://github.com/golemfactory/yagna.git --branch exe-unit/sgx-poc \
	--features sgx \
	ya-exe-unit --bin exe-unit

