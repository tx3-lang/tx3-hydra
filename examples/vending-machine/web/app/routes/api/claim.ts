import * as CSL from "@emurgo/cardano-serialization-lib-nodejs"
import cbor from "cbor"
import { randomUUID } from "crypto";
import { ArgValue } from "tx3-sdk/trp";

import { Client } from "~/tx3/protocol";
import { getAdminCredentials } from "~/utils";

const TRP_URL = process.env["TRP_URL"] || "http://localhost:8164"

export const action = async ({ request }: { request: Request }) => {
  const payload = await request.json();

  const client = new Client({
    endpoint: TRP_URL
  });

  const credentials = getAdminCredentials()
  const adminAddress = CSL.Address.from_bech32(credentials.address)
  const adminPrivateKey = CSL.PrivateKey.from_normal_bytes(
    cbor.decode(Buffer.from(credentials.privateKey, "hex"))
  );

  const response = await client.mintFromScriptTx({
    quantity: ArgValue.from(1),
    minter: ArgValue.from(adminAddress.to_hex()),
    receiver: ArgValue.from(payload.address)
  })

  const fixedTx = CSL.FixedTransaction.from_hex(response.tx)
  fixedTx.sign_and_add_vkey_signature(adminPrivateKey);
  const signedTx = fixedTx.to_hex();

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
      "id": randomUUID()
    })
  })


  return new Response(null, {
    status: 200,
    headers: {
      "Content-Type": "application/json",
    },
  });
};
