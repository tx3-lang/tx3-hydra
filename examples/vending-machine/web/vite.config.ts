import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

export default defineConfig({
  plugins: [wasm(), tailwindcss(), reactRouter(), tsconfigPaths(), topLevelAwait()],
  assetsInclude: ['**/*.wasm'],
  build: {
    target: 'esnext',
  }
});
