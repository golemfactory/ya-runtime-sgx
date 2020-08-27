#!/bin/bash

set -e

run_in_nsjail() {
    "$NSJAIL_PATH" -Mo -R /bin -R /lib -R /lib64 -R /usr -R /etc/passwd -R "$ENCLAVE_KEY_PATH" -R "$GRAPHENE_DIR:/graphene" -R /dev/urandom -B /var/run/aesmd/aesm.socket -B "`pwd`:/work" --cwd /work -- $@
}

run() {
    if [ ! -v YAGNA_DIR ]; then
        echo "you need to specify path to the yagna directory!"
        exit 1
    fi

    local agreement_path=""
    local work_dir=""
    while getopts "a:b:c:w:" name; do
        case "${name}" in
        a)
            agreement_path=${OPTARG}
            ;;
        b)
            ;;
        c)
            ;;
        w)
            work_dir=${OPTARG}
            ;;
        *)
            ;;
        esac
    done

    local rest=${@:OPTIND}

    exec "$NSJAIL_PATH" -Mo -R /lib64/ld-linux-x86-64.so.2 -R /lib -R /usr -R /dev/urandom -R /sys/devices/system/cpu/online -R /dev/isgx -R /dev/gsgx -R "$GRAPHENE_DIR:/graphene" \
        -B "${work_dir}:/work" \
        -R "${agreement_path}:/work/agreement.json" \
        -R "$YAGNA_DIR/target/release/exe-unit:/work/exe-unit" \
        -R "$YAGNA_DIR/exe-unit.sig:/work/exe-unit.sig" \
        -R "$YAGNA_DIR/exe-unit.token:/work/exe-unit.token" \
        -R "$YAGNA_DIR/exe-unit.manifest.sgx:/work/exe-unit.manifest.sgx" \
        -R "$YAGNA_DIR/ya-runtime-sgx:/work/ya-runtime-sgx" \
        -R "$YAGNA_DIR/ya-runtime-sgx.manifest.sgx:/work/ya-runtime-sgx.manifest.sgx" \
        -R "$YAGNA_DIR/ya-runtime-sgx.sig:/work/ya-runtime-sgx.sig" \
        -R "$YAGNA_DIR/ya-runtime-sgx.token:/work/ya-runtime-sgx.token" \
        -R "$YAGNA_DIR/hello:/work/hello" \
        -R "$YAGNA_DIR/hello.sig:/work/hello.sig" \
        -R "$YAGNA_DIR/hello.token:/work/hello.token" \
        -R "$YAGNA_DIR/hello.manifest.sgx:/work/hello.manifest.sgx" \
        --cwd /work \
        --rlimit_as hard \
        --rlimit_cpu hard \
        --rlimit_fsize hard \
        --rlimit_nofile hard \
        --rlimit_nproc hard \
        --rlimit_stack hard \
        -N \
        -- /graphene/Runtime/pal-Linux-SGX /graphene/Runtime/libpal-Linux-SGX.so init exe-unit.manifest.sgx -b ./ya-runtime-sgx -c cache -w . -a agreement.json ${rest}
}

sign() {
    if [ ! -v ENCLAVE_KEY_PATH ]; then
        echo "You need to specify path to the enclave key!"
        exit 1
    fi

    echo "SIGNING hello"
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-sign --output hello.manifest.sgx --libpal /graphene/Runtime/libpal-Linux-SGX.so --key $ENCLAVE_KEY_PATH --manifest hello.manifest --exec hello
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-get-token --sig hello.sig --output hello.token

    echo "SIGNING ya-runtime-sgx"
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-sign --output ya-runtime-sgx.manifest.sgx --libpal /graphene/Runtime/libpal-Linux-SGX.so --key $ENCLAVE_KEY_PATH --manifest ya-runtime-sgx.manifest --exec ya-runtime-sgx
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-get-token --sig ya-runtime-sgx.sig --output ya-runtime-sgx.token

    echo "SIGNING exe-unit"
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-sign --output exe-unit.manifest.sgx --libpal /graphene/Runtime/libpal-Linux-SGX.so --key $ENCLAVE_KEY_PATH --manifest exe-unit.manifest --exec target/release/exe-unit
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-get-token --sig exe-unit.sig --output exe-unit.token
}

main() {
    if [ ! -v NSJAIL_PATH ]; then
        echo "You need to specify path to the nsjail binary!"
        exit 1
    fi
    if [ ! -v GRAPHENE_DIR ]; then
        echo "You need to specify path to the Graphene directory!"
        exit 1
    fi

    case $1
    in
        sign)
            shift
            sign $@
            ;;
        *)
            run $@
            ;;
    esac
}

main $@
