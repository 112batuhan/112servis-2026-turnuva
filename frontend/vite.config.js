import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Load the single shared .env at the repo root (only VITE_-prefixed vars are
  // exposed to the client). During the Docker build VITE_API_URL comes from a
  // build arg / process env instead, which Vite still picks up.
  envDir: "..",
  server: {
    port: 5173,
  },
});
