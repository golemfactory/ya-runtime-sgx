TAG=build-exe-unit
SGX_DRIVER=sgx_driver_2.6
EXE_UNIT_VER=142baf46fa2dd30c74895a444560225838318765
WASI_VER=dcbdf9b581d01b26806c4697e793d9690e6550c3

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

.PHONY: release
release: $(ALL_TARGETS)
	./exe-unit-wrapper.sh sign
	tar -vczf release.tar.gz --exclude graphene/.ready yagna nsjail graphene exe-unit-wrapper.sh

.PHONY: gen_key
gen_key:
	openssl genrsa -3 -out enclave-key.pem 3072

.PHONY: clean
clean:
	$(RM) -rf $(ALL_TARGETS) graphene
