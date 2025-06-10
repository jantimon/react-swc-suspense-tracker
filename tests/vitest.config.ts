import { defineConfig } from "vitest/config";
import swc from "unplugin-swc";
import path from "path";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [
    tsconfigPaths(),
    swc.vite({
      exclude: ["**/dist/**", "**/node_modules/**"],
      jsc: {
        parser: {
          syntax: "typescript",
          tsx: true,
        },
        experimental: {
          plugins: [
            [
              path.resolve("../react_swc_suspense_tracker.wasm"),
              {
                enabled: true,
              },
            ],
          ],
        },
      },
    }),
  ],
});
