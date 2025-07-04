const HYDRA_URL = process.env["HYDRA_URL"] || "http://localhost:4001"

export async function loader() {
  const response = await fetch(`${HYDRA_URL}/snapshot/utxo`, {
    method: "GET",
    headers: {
      "Content-Type": "application/json"
    },
  })

  const utxos = await response.json()

  return new Response(JSON.stringify(utxos), {
    status: 200,
    headers: {
      "Content-Type": "application/json",
    },
  });
}

