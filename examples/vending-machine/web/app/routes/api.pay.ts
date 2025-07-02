export const loader = async () => {
  return new Response(JSON.stringify({ message: "world" }), {
    headers: {
      "Content-Type": "application/json",
    },
  });
};
