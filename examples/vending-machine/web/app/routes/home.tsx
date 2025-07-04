import type { Route } from "./+types/home";
import { useEffect, useState } from "react";

import { toast } from "react-toastify";

export function meta({ }: Route.MetaArgs) {
  return [
    { title: "Hydra Tx3 example" },
    { name: "description", content: "Welcome to Hydra Tx3 example!" },
  ];
}

export default function Home() {
  const [isRegistered, setIsRegistered] = useState(false);
  const [address, setAddress] = useState(null);
  const [registerLoading, setRegisterLoading] = useState(false);

  useEffect(() => {
    const checkSession = async () => {
      const response = await fetch("/api/check-session");
      const { isRegistered, address } = await response.json();
      setIsRegistered(isRegistered);
      setAddress(address);
    };

    checkSession();
  }, []);

  const register = async () => {
    setRegisterLoading(true)
    const response = await fetch("/api/register");

    if (response.ok) {
      toast.success("Wallet registered");
      setIsRegistered(true);
      setRegisterLoading(false)

      const payload = await response.json()
      setAddress(payload.address);
      return
    }

    toast.error("Fail to register a wallet");
    setRegisterLoading(false)
  };

  const claim = async () => {
    try {
      const response = await fetch("/api/claim", {
        method: "POST",
      });

      if (!response.ok) {
        toast.error("Transaction fail");
        return
      }

      toast.success("Transaction done");

    } catch (err) {
      console.error(err)
      toast.error("Transaction fail");
    }
  };

  const payBack = async () => {
    try {
      const response = await fetch("/api/pay", {
        method: "POST",
      });

      if (!response.ok) {
        toast.error("Transaction fail");
        return
      }

      toast.success("Transaction done");

    } catch (err) {
      console.error(err)
      toast.error("Transaction fail");
    }
  };

  return (
    <>
      <div className="container mx-auto p-4">
        <h1 className="text-xl font-bold mb-4">Create a Wallet</h1>

        <button
          onClick={register}
          className={`my-4 px-4 py-2 rounded-md ${!isRegistered
            ? "bg-blue-500 text-white cursor-pointer"
            : "bg-gray-500/50 cursor-not-allowed"
            }`}
          disabled={isRegistered || registerLoading}
        >
          Register
        </button>

        {
          isRegistered ?
            <div>
              <div>
                Your wallet address:
                <div>
                  {address}
                </div>
              </div>
              <div className="flex space-x-2">
                <button
                  onClick={claim}
                  className="mt-4 px-4 py-2 rounded-md bg-blue-500 text-white cursor-pointer"
                >
                  Claim
                </button>

                <button
                  onClick={payBack}
                  className="mt-4 px-4 py-2 rounded-md bg-blue-500 text-white cursor-pointer"
                >
                  Pay back
                </button>
              </div>
            </div>
            : <></>
        }
      </div>
    </>
  );
}
