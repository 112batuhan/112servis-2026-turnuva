import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // The single shared .env lives at the repo root. In Docker builds VITE_API_URL
  // comes from the build arg instead, which Vite still picks up.
  envDir: "..",
  server: {
    port: 5173,
  },
});
