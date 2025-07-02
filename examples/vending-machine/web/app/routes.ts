import { type RouteConfig, index, route } from "@react-router/dev/routes";

export default [
  index("routes/home.tsx"),
  route("api/register", "routes/api.register.ts"),
  route("api/world", "routes/api.pay.ts"),
] satisfies RouteConfig;
