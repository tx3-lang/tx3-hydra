# Basic example

Follow this hydra [Getting Started](https://hydra.family/head-protocol/docs/getting-started) tutorial to run tx3-hydra. Use the `run-docker.sh` shortcut in the Hydra demo for convenience.

With Hydra running, the default port 4001 will be available to tx3-hydra to connect and integrate. 

## Running tx3-hydra
Create a `config.toml` file in the root of the execution or set the env with the path `TRP_HYDRA_CONFIG` using the [Example Config](config.toml).

With the config created, execute the command below to run. 
```sh
cargo run
```

## Resolving a tx3 file
With hydra and tx3-hydra running, you will be able to execute RPC calls and resolve tx3 transactions, there are some Bingen like ts, rust, go, python that you can use not to call the RPC directly which you can check here [trix](https://github.com/tx3-lang/trix), for trix you should configure the `trp` in `trix.toml` to use the tx3-hydra url.

Example using curl, change the params to use your params and the args from your hydra environment.
```sh
curl --location 'http://0.0.0.0:5000' \
--header 'Content-Type: application/json' \
--data '{
    "jsonrpc": "2.0",
    "method": "trp.resolve",
    "params": {
        "tir": {
            "bytecode": "12000106736f7572636501010d0673656e64657205010c0100000d087175616e74697479020000000002010d0872656365697665720500010c0100000d087175616e7469747902010d0673656e64657205000111111006736f757263650c0100000d087175616e7469747902011201000000000000",
            "encoding": "hex",
            "version": "v1alpha5"
        },
        "args": {
            "sender": "addr_test1vp5cxztpc6hep9ds7fjgmle3l225tk8ske3rmwr9adu0m6qchmx5z",
            "receiver": "addr_test1vqx5tu4nzz5cuanvac4t9an4djghrx7hkdvjnnhstqm9kegvm6g6c",
            "quantity": 2000000
        }
    },
    "id": "8ee35fe0-6752-4b8e-8903-53ed231f8ba8"
}'
```

## Submit a transaction
After resolving the transaction, you can sign and Submit using tx3-hydra.

Example using curl, change the payload to use your cbor hex transaction.

```sh
curl --location 'http://0.0.0.0:5000' \
--header 'Content-Type: application/json' \
--data '{
    "jsonrpc": "2.0",
    "method": "trp.submit",
    "params": {
        "tx": {
            "payload": "84a400d9010281825820c9a5fb7ca6f55f07facefccb7c5d824eed00ce18719d28ec4c4a2e4041e85d97000182a200581d600d45f2b310a98e766cee2ab2f6756c91719bd7b35929cef058365b65011a001e8480a200581d6069830961c6af9095b0f2648dff31fa9545d8f0b6623db865eb78fde8011a05d75c8002000f00a100d9010281825820f953b2d6b6f319faa9f8462257eb52ad73e33199c650f0755e279e21882399c05840efe4ec0c8c905d4c39c2182088bf39c91357c3b9a9a1289ad45418fa2cd2ea7153c85801532c0a5ce47cb9ba25cd0cdaebb25842d6e349107c15854d0729e006f5f6",
            "encoding": "hex",
            "version": "v1alpha5"
        }
    },
    "id": "b103f8d2-ccf6-4a2e-9a4d-b69c280175ce"
}'
```
