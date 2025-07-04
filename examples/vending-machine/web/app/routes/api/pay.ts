import * as CSL from "@emurgo/cardano-serialization-lib-nodejs"
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
  const sessionPrivateKey = CSL.PrivateKey.from_hex(session.get("privateKey")!);

  const client = new Client({
    endpoint: TRP_URL
  });

  const credentials = getAdminCredentials()
  const response = await client.transferTx({
    quantity: 1_000_000,
    receiver: credentials.address,
    sender: sessionAddressHex
  })

  const fixedTx = CSL.FixedTransaction.from_hex(response.tx)
  fixedTx.sign_and_add_vkey_signature(sessionPrivateKey);
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
