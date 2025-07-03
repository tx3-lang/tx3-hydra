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

    // TODO: import from the file
    const address = CSL.Address.from_bech32("addr_test1vz5yzy8fttld8yprtzhsz5kuwk46xs9npnfdh3ajaggm5ccyg00d6")
    const privateKey = CSL.PrivateKey.from_normal_bytes(
      cbor.decode(Buffer.from("582088c48ee7d969d49a161e469added3af9c4a337064c7a79734fa1d1094decf0e4", "hex"))
    );

    // TODO: change the params later to use mint token tx3 params
    const response = await client.transferTx({
      quantity: 1000000,
      receiver: payload.address,
      sender: address.to_hex()
    })

    const fixedTx = CSL.FixedTransaction.from_hex(response.tx)
    fixedTx.sign_and_add_vkey_signature(privateKey);
    const signedTx = fixedTx.to_hex();

    console.log(signedTx)

    // TODO: add support for submit in trp?
    // TODO: validate errors
    await fetch(TRP_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
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
