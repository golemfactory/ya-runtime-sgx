FROM rust:1.46

ARG EXE_UNIT_VERSION
RUN OPENSSL_STATIC=yes cargo install \
	--git https://github.com/golemfactory/yagna.git --rev $EXE_UNIT_VERSION \
	--features sgx \
	ya-exe-unit --bin exe-unit
