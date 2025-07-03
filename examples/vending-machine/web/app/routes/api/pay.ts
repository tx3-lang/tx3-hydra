import * as CSL from "@emurgo/cardano-serialization-lib-nodejs"
import cbor from "cbor"

import { Client } from "~/tx3/protocol";

const TRP_URL = process.env["TRP_URL"] || "http://localhost:8164"

export const action = async ({ request }: { request: Request }) => {
  if (request.method === 'POST') {
    const payload = await request.json();

    const client = new Client({
      endpoint: TRP_URL
    });

    const response = await client.transferTx({
      quantity: 1000000,
      receiver: "addr_test1vz5yzy8fttld8yprtzhsz5kuwk46xs9npnfdh3ajaggm5ccyg00d6",
      sender: payload.address
    })

    const fixedTx = CSL.FixedTransaction.from_hex(response.tx)
    const hash = fixedTx.transaction_hash().to_hex()
    console.log("TX HASH TO SIGN: ", hash)

    return new Response(JSON.stringify({
      tx: response.tx,
      hash
    }), {
      status: 200,
      headers: {
        "Content-Type": "application/json",
      },
    });
  }

  if (request.method === 'PUT') {
    const payload = await request.json();


    const decodedKey = await cbor.decodeFirst(Buffer.from(payload.sign.key, "hex"));
    const rawPubKeyBytes = decodedKey.get(-2); // Buffer of 32 bytes
    const publicKey = CSL.PublicKey.from_bytes(rawPubKeyBytes);

    const decodedSig = await cbor.decodeFirst(Buffer.from(payload.sign.signature, "hex"));
    const rawSigBytes = decodedSig[3]; // Buffer of 64 bytes
    const signature = CSL.Ed25519Signature.from_bytes(rawSigBytes);
    // console.dir(decodedSig, { depth: null });

    const vKey = CSL.Vkey.new(publicKey)
    const witness = CSL.Vkeywitness.new(vKey, signature);

    const fixedTx = CSL.FixedTransaction.from_hex(payload.tx)
    fixedTx.add_vkey_witness(witness)
    const signedTx = fixedTx.to_hex();

    console.log("SIGNED TX: ", signedTx)

    // TODO: add support for submit in trp?
    // TODO: validate errors
    await fetch(TRP_URL, {
      headers: {
        "Content-Type": "application/json"
      },
      method: "post", body: JSON.stringify({
        "jsonrpc": "2.0",
        "method": "trp.submit",
        "params": {
          "tx": {
            "payload": signedTx,
            "encoding": "hex",
            "version": "v1alpha5"
          }
        },
        "id": "0"
      })
    })

    return new Response(null, {
      status: 200,
      headers: {
        "Content-Type": "application/json",
      },
    });
  }

  return new Response(null, {
    status: 404,
    statusText: "Not found",
  });
};
