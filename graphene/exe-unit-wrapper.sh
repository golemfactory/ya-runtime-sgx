#!/bin/bash

set -e

default_path="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

run_in_nsjail() {
    "$NSJAIL_PATH" -Mo -R /bin -R /lib -R /lib64 -R /usr -R /etc -R /run -R "$ENCLAVE_KEY_PATH" -R "$GRAPHENE_DIR:/graphene" -R /dev/urandom -B /var/run/aesmd/aesm.socket -B "`pwd`:/work" --cwd /work -E "PATH=$default_path" -- $@
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

    mkdir "${work_dir}/protected"

    exec "$NSJAIL_PATH" -Q -Mo -R /lib64/ld-linux-x86-64.so.2 -R /lib -R /usr -R /dev/urandom -R /sys/devices/system/cpu/online -R /dev/isgx -R /dev/gsgx -R "$GRAPHENE_DIR:/graphene" -R /etc/resolv.conf \
        -B "${work_dir}:/work" \
        -R "$YAGNA_DIR/root.crt:/work/root.crt" \
        -R "${agreement_path}:/work/agreement.json" \
        -R "$YAGNA_DIR/exe-unit:/work/exe-unit" \
        -R "$YAGNA_DIR/exe-unit.sig:/work/exe-unit.sig" \
        -R "$YAGNA_DIR/exe-unit.token:/work/exe-unit.token" \
        -R "$YAGNA_DIR/exe-unit.manifest.sgx:/work/exe-unit.manifest.sgx" \
        -R "$YAGNA_DIR/ya-runtime-wasi:/work/ya-runtime-wasi" \
        -R "$YAGNA_DIR/ya-runtime-wasi.manifest.sgx:/work/ya-runtime-wasi.manifest.sgx" \
        -R "$YAGNA_DIR/ya-runtime-wasi.sig:/work/ya-runtime-wasi.sig" \
        -R "$YAGNA_DIR/ya-runtime-wasi.token:/work/ya-runtime-wasi.token" \
        --cwd /work \
        -E "PATH=$default_path" \
        --rlimit_as hard \
        --rlimit_cpu hard \
        --rlimit_fsize hard \
        --rlimit_nofile hard \
        --rlimit_nproc hard \
        --rlimit_stack hard \
        -N \
        -- /graphene/Runtime/pal-Linux-SGX /graphene/Runtime/libpal-Linux-SGX.so init exe-unit.manifest.sgx -b ./ya-runtime-wasi -c protected/cache -w . -a agreement.json ${rest}
}

sign() {
    if [ ! -v ENCLAVE_KEY_PATH ]; then
        echo "You need to specify path to the enclave key!"
        exit 1
    fi

    echo "SIGNING ya-runtime-wasi"
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-sign --output ya-runtime-wasi.manifest.sgx --libpal /graphene/Runtime/libpal-Linux-SGX.so --key $ENCLAVE_KEY_PATH --manifest ya-runtime-wasi.manifest --exec ya-runtime-wasi
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-get-token --sig ya-runtime-wasi.sig --output ya-runtime-wasi.token

    echo "SIGNING exe-unit"
    run_in_nsjail /graphene/Pal/src/host/Linux-SGX/signer/pal-sgx-sign --output exe-unit.manifest.sgx --libpal /graphene/Runtime/libpal-Linux-SGX.so --key $ENCLAVE_KEY_PATH --manifest exe-unit.manifest --exec exe-unit
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
