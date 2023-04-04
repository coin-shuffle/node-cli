# node

Implementation of partipant in coin shuffle process

## Running

Generate RSA private key:

```bash
openssl genrsa > rsa-key.pem
```

Write ECDSA private key to file:

```bash
echo "<private-ley>" > ecdsa.key
```

To start shuffling:

```bash
cargo run -- shuffle --utxo-id 199 --service-url http://127.0.0.1:8080 --rpc-url https://goerli.blockpi.network/v1/rpc/public --ecdsa-priv-path ./ecdca.key --rsa-priv-path ./rsa-key.pem --utxo-address 0x4C0d116d9d028E60904DCA468b9Fa7537Ef8Cd5f --output-address 0xdC230332Bd602EC4E286D2A59878A9DF52aB62ef
```
