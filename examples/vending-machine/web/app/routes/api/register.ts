import * as CSL from "@emurgo/cardano-serialization-lib-nodejs"
import { getSession, commitSession } from "~/sessions.server"; // Assuming sessions.server.ts is in the app directory
import type { Route } from "../+types/home";

export async function loader({ request }: Route.LoaderArgs) {
  const session = await getSession(request.headers.get("Cookie"));

  if (session.has("privateKey")) {
    return new Response(JSON.stringify({ message: "Wallet already registered" }), {
      status: 400,
      headers: {
        "Content-Type": "application/json",
      },
    });
  }

  const privateKey = CSL.PrivateKey.generate_ed25519()
  const publicKey = privateKey.to_public();
  const address = CSL.BaseAddress.new(
    CSL.NetworkInfo.testnet_preview().network_id(),
    CSL.Credential.from_keyhash(publicKey.hash()),
    CSL.Credential.from_keyhash(publicKey.hash())
  ).to_address();


  session.set("privateKey", privateKey.to_hex());
  session.set("address", address.to_hex());

  const response = {
    address: address.to_bech32(),
  }

  return new Response(JSON.stringify(response), {
    status: 200,
    headers: {
      "Content-Type": "application/json",
      "Set-Cookie": await commitSession(session),
    },
  });
}

