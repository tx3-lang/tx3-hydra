import * as CSL from "@emurgo/cardano-serialization-lib-nodejs"
import { getSession } from "~/sessions.server";
import type { Route } from "../+types/home";

export async function loader({ request }: Route.LoaderArgs) {
  const session = await getSession(request.headers.get("Cookie"));
  const isRegistered = session.has("privateKey");

  const sessionAddressHex = session.get("address")!;
  const address = CSL.Address.from_hex(sessionAddressHex)

  return new Response(JSON.stringify({ isRegistered, address: address.to_bech32() }), {
    status: 200,
    headers: {
      "Content-Type": "application/json",
    },
  });
};
