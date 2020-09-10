TAG=build-exe-unit

all: sgx-exe-unit nsjail ya-runtime-sgx-wasi

sgx-exe-unit: exe-unit.Dockerfile
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /usr/local/cargo/bin/exe-unit $@

nsjail:
	docker build -t $(TAG) --build-arg PROTOBUF_STATIC=yes --build-arg NL3_STATIC=yes github.com/golemfactory/nsjail
	$(SHELL) $(srcdir)extract.sh $(TAG) /bin/nsjail $@

ya-runtime-sgx-wasi: ya-runtime-wasi.Dockerfile
	tar c $^ | docker build -t $(TAG) -f $< -
	$(SHELL) $(srcdir)extract.sh $(TAG) /usr/local/cargo/bin/ya-runtime-wasi $@


	
