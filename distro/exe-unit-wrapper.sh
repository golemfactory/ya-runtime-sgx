#!/bin/bash

set -e

default_path="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

NSJAIL_PATH=${NSJAIL_PATH:-"/opt/yagna-wasi-sgx/nsjail"}
GRAPHENE_DIR=${GRAPHENE_DIR:-"/opt/yagna-wasi-sgx/graphene/"}
YAGNA_DIR=${YAGNA_DIR:-"/opt/yagna-wasi-sgx/yagna"}

run_in_nsjail() {
    "$NSJAIL_PATH" -Mo -R /bin -R /lib -R /lib64 -R /usr -R /etc -R /run ${ENCLAVE_KEY_PATH:+ -R "${ENCLAVE_KEY_PATH}"} -R "$GRAPHENE_DIR:/graphene" -R /dev/urandom -B /var/run/aesmd/aesm.socket -B "$YAGNA_DIR:/work" --cwd /work -E "PATH=$default_path" -- $@
}

run() {
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

    exec "$NSJAIL_PATH" -Q -Mo -R /lib64/ld-linux-x86-64.so.2 -R /lib -R /usr -R /dev/urandom -R /sys/devices/system/cpu/online -R /dev/isgx -R /dev/gsgx -R "$GRAPHENE_DIR:/graphene" \
        -B "${work_dir}:/work" \
        -R "$YAGNA_DIR/resolv.conf:/work/resolv.conf" \
        -R "${agreement_path}:/work/agreement.json" \
        -R "$YAGNA_DIR/sgx-exe-unit:/work/sgx-exe-unit" \
        -R "$YAGNA_DIR/sgx-exe-unit.sig:/work/sgx-exe-unit.sig" \
        -R "$YAGNA_DIR/sgx-exe-unit.token:/work/sgx-exe-unit.token" \
        -R "$YAGNA_DIR/sgx-exe-unit.manifest.sgx:/work/sgx-exe-unit.manifest.sgx" \
        -R "$YAGNA_DIR/ya-runtime-sgx-wasi:/work/ya-runtime-sgx-wasi" \
        -R "$YAGNA_DIR/ya-runtime-sgx-wasi.manifest.sgx:/work/ya-runtime-sgx-wasi.manifest.sgx" \
        -R "$YAGNA_DIR/ya-runtime-sgx-wasi.sig:/work/ya-runtime-sgx-wasi.sig" \
        -R "$YAGNA_DIR/ya-runtime-sgx-wasi.token:/work/ya-runtime-sgx-wasi.token" \
        -R "$YAGNA_DIR/libgcc_s.so.1:/work/libgcc_s.so.1" \
        --cwd /work \
        -E "PATH=$default_path" \
        --rlimit_as hard \
        --rlimit_cpu hard \
        --rlimit_fsize hard \
        --rlimit_nofile hard \
        --rlimit_nproc hard \
        --rlimit_stack hard \
        -N \
        -- /graphene/Runtime/pal-Linux-SGX /graphene/Runtime/libpal-Linux-SGX.so init sgx-exe-unit.manifest.sgx -b ./ya-runtime-sgx-wasi -c protected/cache -w . -a agreement.json --requestor-pub-key ${rest}
}

sign() {
    run_in_nsjail /graphene/scripts/pal-sgx-sign --output ya-runtime-sgx-wasi.manifest.sgx --libpal /graphene/Runtime/libpal-Linux-SGX.so --key $ENCLAVE_KEY_PATH --manifest ya-runtime-sgx-wasi.manifest --exec ya-runtime-sgx-wasi

    run_in_nsjail /graphene/scripts/pal-sgx-sign --output sgx-exe-unit.manifest.sgx --libpal /graphene/Runtime/libpal-Linux-SGX.so --key $ENCLAVE_KEY_PATH --manifest sgx-exe-unit.manifest --exec sgx-exe-unit
}

token() {
    run_in_nsjail /graphene/scripts/pal-sgx-get-token --sig ya-runtime-sgx-wasi.sig --output ya-runtime-sgx-wasi.token
    run_in_nsjail /graphene/scripts/pal-sgx-get-token --sig sgx-exe-unit.sig --output sgx-exe-unit.token
}

main() {
    case $1
    in
        sign)
            shift
            sign $@
            ;;
        token)
            token
            ;;
        *)
            local args=$@
            run ${args/--requestor-pub-key/}
            ;;
    esac
}

main $@
