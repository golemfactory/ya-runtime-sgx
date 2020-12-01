TAG=build-exe-unit
SGX_DRIVER=sgx_driver_2.6
EXE_UNIT_VER=9fd74a641439e9c337a7c5a24d2435ef9c91710d
WASI_VER=08bcda4f4ef19a45d9d267d0d15a2c7b560504c5

ALL_TARGETS=yagna/sgx-exe-unit nsjail yagna/ya-runtime-sgx-wasi yagna/libgcc_s.so.1 yagna/liblzma.so.5 graphene/.ready
all: $(ALL_TARGETS)

yagna/sgx-exe-unit yagna/libgcc_s.so.1 yagna/liblzma.so.5: exe-unit.Dockerfile
	tar c $^ | docker build -t $(TAG) --build-arg EXE_UNIT_VERSION=$(EXE_UNIT_VER) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /usr/local/cargo/bin/exe-unit yagna/sgx-exe-unit
	$(SHELL) $(srcdir)extract.sh $(TAG) /lib/x86_64-linux-gnu/libgcc_s.so.1 yagna/libgcc_s.so.1
	$(SHELL) $(srcdir)extract.sh $(TAG) /lib/x86_64-linux-gnu/liblzma.so.5 yagna/liblzma.so.5

nsjail:
	docker build -t $(TAG) --build-arg PROTOBUF_STATIC=yes --build-arg NL3_STATIC=yes github.com/golemfactory/nsjail
	$(SHELL) $(srcdir)extract.sh $(TAG) /bin/nsjail $@

yagna/ya-runtime-sgx-wasi: ya-runtime-wasi.Dockerfile
	tar c $^ | docker build -t $(TAG) --build-arg WASI_VERSION=$(WASI_VER) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /usr/local/cargo/bin/ya-runtime-wasi $@

graphene/.ready: graphene.Dockerfile
	tar c $^ | docker build -t $(TAG) --build-arg SGX_DRIVER=$(sgx_driver_2.6) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /leeroy/graphene/runtime.tar.gz graphene.tar.gz
	mkdir -p graphene
	tar -vxf graphene.tar.gz -C graphene
	$(RM) graphene.tar.gz
	touch $@

ya-runtime-sgx.tar.gz: $(ALL_TARGETS) ya-runtime-sgx.json exe-unit-wrapper.sh
	./exe-unit-wrapper.sh sign
	tar -vczf $@ --exclude graphene/.ready $^ graphene sgx_config.json

ya-runtime-sgx-artifacts.tar.gz: $(ALL_TARGETS) ya-runtime-sgx.json exe-unit-wrapper.sh
	tar -vczf $@ --exclude graphene/.ready $^ graphene

.PHONY: build
build: ya-runtime-sgx-artifacts.tar.gz

.PHONY: release
release: ya-runtime-sgx.tar.gz

.PHONY: gen_key
gen_key:
	openssl genrsa -3 -out enclave-key.pem 3072

.PHONY: clean
clean:
	$(RM) -rf $(ALL_TARGETS) graphene ya-runtime-sgx.tar.gz ya-runtime-sgx-artifacts.tar.gz
