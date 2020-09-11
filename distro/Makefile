TAG=build-exe-unit

all: sgx-exe-unit nsjail ya-runtime-sgx-wasi libgcc_s.so.1 graphene/.ready

sgx-exe-unit: exe-unit.Dockerfile
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /usr/local/cargo/bin/exe-unit $@

libgcc_s.so.1: exe-unit.Dockerfile
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /lib/x86_64-linux-gnu/libgcc_s.so.1 $@

nsjail:
	docker build -t $(TAG) --build-arg PROTOBUF_STATIC=yes --build-arg NL3_STATIC=yes github.com/golemfactory/nsjail
	$(SHELL) $(srcdir)extract.sh $(TAG) /bin/nsjail $@

ya-runtime-sgx-wasi: ya-runtime-wasi.Dockerfile
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /usr/local/cargo/bin/ya-runtime-wasi $@

graphene/.ready: graphene.Dockerfile sgx.h
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /leeroy/graphene/runtime.tar.gz graphene.tar.gz
	mkdir -p graphene
	tar -vxf graphene.tar.gz -C graphene
	$(RM) graphene.tar.gz
	touch $@
