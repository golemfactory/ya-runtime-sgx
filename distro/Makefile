TAG=build-exe-unit
SGX_DRIVER=sgx_driver_2.6

all: yagna/sgx-exe-unit nsjail yagna/ya-runtime-sgx-wasi yagna/libgcc_s.so.1 graphene/.ready

yagna/sgx-exe-unit: exe-unit.Dockerfile
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /usr/local/cargo/bin/exe-unit $@

yagna/libgcc_s.so.1: exe-unit.Dockerfile
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /lib/x86_64-linux-gnu/libgcc_s.so.1 $@

nsjail:
	docker build -t $(TAG) --build-arg PROTOBUF_STATIC=yes --build-arg NL3_STATIC=yes github.com/golemfactory/nsjail
	$(SHELL) $(srcdir)extract.sh $(TAG) /bin/nsjail $@

yagna/ya-runtime-sgx-wasi: ya-runtime-wasi.Dockerfile
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /usr/local/cargo/bin/ya-runtime-wasi $@

graphene/.ready: graphene.Dockerfile sgx.h
	tar c $^ | docker build -t $(TAG) --build-arg SGX_DRIVER=$(sgx_driver_2.6) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /leeroy/graphene/runtime.tar.gz graphene.tar.gz
	mkdir -p graphene
	tar -vxf graphene.tar.gz -C graphene
	$(RM) graphene.tar.gz
	touch $@

