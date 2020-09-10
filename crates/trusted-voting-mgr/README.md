
## Develop

Install wasmtime

```
curl https://wasmtime.dev/install.sh -sSf | bash
```

Install wasm32-wasi target

```
rustup target add wasm32-wasi
```

Run simple flow.

```
$ mkdir data
$ cargo run init aea5db67524e02a263b9339fe6667d6b577f3d4c 1
OK 83c2cc708440afa4d6f7d19f76455e7a7e529b87
$ cargo run debug
OK 83c2cc708440afa4d6f7d19f76455e7a7e529b87 aea5db67524e02a263b9339fe6667d6b577f3d4c 1
```

## Testing on Golem


```
$ cargo install --git https://github.com/golemfactory/cargo-ya-wasi-pkg.git
$ cargo ya-wasi-pkg --publish
...
    Finished release [optimized] target(s) in 27.73s
generated package: /home/reqc/workspace/ya/ya-runtime-sgx/target/ya-pkg/trusted-voting-mgr.ywasi
pushing image to repo
published package hash:sha3:d1899ee0daaadac8a2bf9cae4c8a44fdd3d7780cd097e2336ba1f948:http://yacn.dev.golem.network.:8000/trusted-voting-mgr-780cd097e2336ba1f948.ywasi

```

