import type { Route } from "./+types/home";
import { useEffect, useState } from "react";

import type { CardanoWalletInfo, CardanoWalletAPI } from "../@types/cardano";
import { toast } from "react-toastify";

export function meta({ }: Route.MetaArgs) {
  return [
    { title: "New React Router App" },
    { name: "description", content: "Welcome to React Router!" },
  ];
}

export default function Home() {
  const [connectedWallet, setConnectedWallet] =
    useState<CardanoWalletAPI | null>(null);
  const [connectedWalletInfo, setConnectedWalletInfo] =
    useState<CardanoWalletInfo | null>(null);
  const [wallets, setWallets] = useState<CardanoWalletInfo[]>([]);
  const [walletAddress, setWalletAddress] = useState<string | null>(null);


  useEffect(() => {
    if (typeof window !== "undefined" && window.cardano) {
      const loadedWallets: CardanoWalletInfo[] = Object.entries(
        window.cardano
      ).map(([_, wallet]) => wallet);
      setWallets(loadedWallets);
    }
  }, []);

  useEffect(() => {
    const getAddress = async () => {
      if (connectedWallet) {
        const hexAddress = await connectedWallet.getChangeAddress();
        setWalletAddress(hexAddress);
      } else {
        setWalletAddress(null);
      }
    };

    getAddress();
  }, [connectedWallet]);

  const connectWallet = async (walletInfo: CardanoWalletInfo) => {
    try {
      const api = await walletInfo.enable();
      setConnectedWallet(api);
      setConnectedWalletInfo(walletInfo);
      console.log("Wallet connected:", walletInfo.name);
    } catch (error) {
      console.error("Error connecting wallet:", error);
    }
  };

  const register = async () => {
    const response = await fetch("/api/register", {
      method: "POST",
      body: JSON.stringify({
        address: walletAddress
      })
    });

    if (response.ok) {
      toast.success("Transaction done");
      return
    }

    toast.error("Transaction fail");
  };

  return (
    <>

      <div className="container mx-auto p-4">
        <h1 className="text-xl font-bold mb-4">Connect Wallet</h1>

        <div className="flex space-x-2 mb-4">
          {wallets.map((w: CardanoWalletInfo) => (
            <button
              key={w.name}
              onClick={() => connectWallet(w)}
              className={`flex space-x-2 px-12 py-2 rounded-md ${connectedWalletInfo?.name == w.name
                ? "bg-green-500/50"
                : "bg-gray-500/50"
                }`}
              disabled={!!connectedWalletInfo}
            >
              <img src={w.icon} alt={`${w.name} icon`} width="24" height="24" />
              <span>{w.name}</span>
            </button>
          ))}
        </div>

        <div>{walletAddress}</div>

        <button
          onClick={register}
          className={`mt-4 px-4 py-2 rounded-md ${walletAddress
            ? "bg-blue-500 text-white"
            : "bg-gray-500/50 cursor-not-allowed"
            }`}
          disabled={!walletAddress}
        >
          Get Tx
        </button>
      </div>
    </>
  );
}

