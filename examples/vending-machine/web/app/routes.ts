import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("api/claim", "routes/api/claim.ts"),
  route("api/utxos", "routes/api/utxos.ts"),
] satisfies RouteConfig;
