import * as CSL from "@emurgo/cardano-serialization-lib-nodejs"
import cbor from "cbor"
import { randomUUID } from "crypto";
import { getSession } from "~/sessions.server";

import { Client } from "~/tx3/protocol";
import { getAdminCredentials } from "~/utils";

const TRP_URL = process.env["TRP_URL"] || "http://localhost:8164"

export const action = async ({ request }: { request: Request }) => {
  const session = await getSession(request.headers.get("Cookie"));

  if (!session.has("privateKey")) {
    return new Response(JSON.stringify({ message: "Register a wallet first" }), {
      status: 406,
      headers: {
        "Content-Type": "application/json",
      },
    });
  }

  const sessionAddressHex = session.get("address")!;

  const client = new Client({
    endpoint: TRP_URL
  });

  const credentials = getAdminCredentials()
  const adminAddress = CSL.Address.from_bech32(credentials.address)
  const adminPrivateKey = CSL.PrivateKey.from_normal_bytes(
    cbor.decode(Buffer.from(credentials.privateKey, "hex"))
  );

  // TODO: change the params later to use mint token tx3 params
  const response = await client.transferTx({
    quantity: 2_000_000,
    receiver: sessionAddressHex,
    sender: adminAddress.to_hex()
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
