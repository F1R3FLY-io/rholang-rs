import path from "path";
import { defineConfig } from "vite";

export default defineConfig({
  build: {
    lib: {
      entry: path.resolve(__dirname, "src/index.ts"),
      fileName: (format) => `tree-sitter-rholang-js.${format}.js`,
      formats: ["es", "cjs", "umd"],
      name: "TreeSitterRholang",
    },
  },
});
