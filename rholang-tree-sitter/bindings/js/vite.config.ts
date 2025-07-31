import path from "path";
import { defineConfig } from "vite";

export default defineConfig({
  build: {
    lib: {
      entry: path.resolve(__dirname, "src/index.ts"),
      fileName: (format) => `embers-client-sdk.${format}.js`,
      formats: ["es", "cjs", "umd"],
      name: "EmbersClientSdk",
    },
  },
});
