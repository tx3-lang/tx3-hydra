import * as CSL from "@emurgo/cardano-serialization-lib-nodejs"

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

    return new Response(JSON.stringify({
      tx: response.tx
    }), {
      status: 204,
      headers: {
        "Content-Type": "application/json",
      },
    });
  }

  if (request.method === 'PUT') {
    const payload = await request.json();

    const fixedTx = CSL.FixedTransaction.from_hex(payload.tx)
    const witness = CSL.Vkeywitness.from_bytes(
      Buffer.from(payload.signature, "hex")
    );
    fixedTx.add_vkey_witness(witness)
    const signedTx = fixedTx.to_hex();

    console.log(signedTx)

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
      })
    })
  }

  return new Response(null, {
    status: 404,
    statusText: "Not found",
  });
};
