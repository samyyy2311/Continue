/// <reference types="vitest" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    hmr: {
      overlay: false,
    },
    // The desktop shares Instrument Sans with the Android app instead of keeping a copy.
    fs: {
      allow: [".", "../android/app/src/main/res/font"],
    },
  },
  test: {
    globals: true,
  },
});
