import type { Route } from "./+types/home";
import { useEffect, useRef, useState } from "react";

import { toast } from "react-toastify";
import { Client } from "~/tx3/protocol";

const TRP_URL = import.meta.env.VITE_TRP_URL
const VM_ADDRESS = import.meta.env.VITE_VM_ADDRESS

export function meta({ }: Route.MetaArgs) {
  return [
    { title: "Hydra Tx3 example" },
    { name: "description", content: "Welcome to Hydra Tx3 example!" },
  ];
}

export default function Home() {
  const [address, setAddress] = useState<string>();
  const [privateKey, setPrivateKey] = useState<string>();
  const [quantityTokens, setQuantityTokens] = useState<number>();
  const [utxos, setUtxos] = useState<object>();

  const CSL = useRef<typeof import('@emurgo/cardano-serialization-lib-browser') | null>(null);
  async function loadCardanoWasm() {
    if (!CSL.current) {
      CSL.current = await import('@emurgo/cardano-serialization-lib-browser')
      console.log("Cardano WASM loaded:", CSL.current)
    }
  }

  useEffect(() => {
    loadUtxos();

    const privateKey = localStorage.getItem("privateKey");
    const address = localStorage.getItem("address");

    if (privateKey && address) {
      setPrivateKey(privateKey);
      setAddress(address);
    }

  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      loadUtxos()
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  async function register() {
    await loadCardanoWasm();

    if (localStorage.getItem("privateKey")) {
      return
    }

    const privateKey = CSL.current!.PrivateKey.generate_ed25519();
    const publicKey = privateKey.to_public();

    const address = CSL.current!.BaseAddress.new(
      CSL.current!.NetworkInfo.testnet_preview().network_id(),
      CSL.current!.Credential.from_keyhash(publicKey.hash()),
      CSL.current!.Credential.from_keyhash(publicKey.hash())
    ).to_address();

    localStorage.setItem("privateKey", privateKey.to_hex());
    localStorage.setItem("address", address.to_bech32());

    setPrivateKey(privateKey.to_hex());
    setAddress(address.to_bech32());
  }

  async function loadUtxos() {
    const response = await fetch("/api/utxos");

    if (!response.ok) {
      return
    }

    const utxos = await response.json()
    setUtxos(utxos)
  };


  async function claim() {
    const response = await fetch("/api/claim", {
      method: "POST",
      body: JSON.stringify({
        address
      })
    });

    if (!response.ok) {
      toast.error("Claim Transaction fail");
      return
    }

    toast.success("Claim Transaction submitted");
  };

  async function sendTokens() {
    try {
      if (!quantityTokens || quantityTokens < 1) {
        toast.warning("Type a quantity of tokens to send");
        return
      }

      await loadCardanoWasm();

      const client = new Client({
        endpoint: TRP_URL
      });

      const response = await client.transferTx({
        quantity: quantityTokens,
        receiver: VM_ADDRESS,
        sender: address!
      })

      console.info("TX CBOR: ", response.tx)

      const fixedTx = CSL.current!.FixedTransaction.from_hex(response.tx)
      fixedTx.sign_and_add_vkey_signature(CSL.current!.PrivateKey.from_hex(privateKey!));
      const signedTx = fixedTx.to_hex();

      const responseSubmit = await fetch(TRP_URL, {
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

      if (!responseSubmit.ok) {
        toast.error("Send Tokens Transaction fail");
        return
      }

      toast.success("Send Tokens Transaction submitted");
    } catch (err) {
      console.error(err)
      toast.error("Send Tokens Transaction fail");
    }
  };

  return (
    <>
      <div className="container mx-auto p-4">
        <h1 className="text-xl font-bold mb-4">Create a Wallet</h1>

        <button
          type="button"
          onClick={register}
          className={`my-4 px-4 py-2 rounded-md ${!(privateKey && address)
            ? "bg-blue-500 text-white cursor-pointer"
            : "bg-gray-500/50 cursor-not-allowed"
            }`}
          disabled={!!(privateKey && address)}
        >
          Register
        </button>

        {
          (privateKey && address) &&
          <div>
            <div className="space-y-2">
              <div>
                <div className="font-bold">
                  Vending Machine Address:
                </div>

                <div>
                  {VM_ADDRESS}
                </div>
              </div>

              <div>
                <div className="font-bold">
                  Your Wallet Address:
                </div>
                <div>
                  {address}
                </div>
              </div>
            </div>

            <div className="flex justify-between">
              <div className="flex space-x-2">
                <div>
                  <button
                    type="button"
                    onClick={claim}
                    className="mt-4 px-4 py-2 rounded-md bg-blue-500 text-white cursor-pointer border border-blue-500"
                  >
                    Claim Tokens
                  </button>
                </div>
                <div className="space-x-1">

                  <button
                    type="button"
                    onClick={sendTokens}
                    className="mt-4 px-4 py-2 rounded-md bg-blue-500 text-white cursor-pointer border border-blue-500"
                  >
                    Send Tokens
                  </button>

                  <input
                    type="number"
                    id="quantityTokens"
                    name="quantityTokens"
                    placeholder="Quantity of tokens"
                    className="px-4 py-2 rounded-md border border-gray-300 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-transparent transition duration-200 shadow-sm placeholder-gray-400"
                    value={quantityTokens}
                    onChange={(e) => {
                      const parsed = parseFloat(e.target.value);
                      if (!isNaN(parsed)) {
                        setQuantityTokens(parsed)
                      } else {
                        setQuantityTokens(undefined)
                      }
                    }}
                  />
                </div>
              </div>

              <div>
                <button
                  type="button"
                  onClick={loadUtxos}
                  className="mt-4 px-4 py-2 rounded-md bg-blue-500 text-white cursor-pointer border border-blue-500"
                >
                  Refresh Utxos
                </button>
              </div>
            </div>
          </div>
        }

        <div>
          <h2 className="my-4 text-lg font-bold"> Utxos </h2>
          {
            utxos &&
            Object.entries(utxos).map(([hash, utxo]) => {
              return (
                <div className="bg-gray-800/40 rounded-md my-2 p-2" key={hash}>
                  {
                    utxo.address == address &&
                    <div className="bg-green-600 w-10 flex justify-center rounded-md text-sm font-semibold mb-2"> own </div>
                  }

                  {
                    utxo.address == VM_ADDRESS &&
                    <div className="bg-blue-600 w-32 flex justify-center rounded-md text-sm font-semibold mb-2"> Vending Machine </div>
                  }

                  <div> {hash} </div>
                  <div> {Object.entries(utxo.value).map(([coin, amount], idx) => (
                    <span key={`${hash}-${idx}`}>
                      {coin}: {amount}
                    </span>
                  ))}
                  </div>
                </div>
              )
            })
          }
        </div>

      </div>
    </>
  );
}
