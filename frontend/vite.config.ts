import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

// The single shared .env lives at the repo root. In Docker builds VITE_API_URL
// and DEV_LOGIN come from build args instead, which Vite still picks up.
const ENV_DIR = "..";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, ENV_DIR, "");
  return {
    plugins: [react()],
    envDir: ENV_DIR,
    // DEV_LOGIN is shared with the backend, so it's injected by hand rather than
    // renamed to VITE_*. Off, the account switcher is left out of the bundle.
    define: {
      __DEV_LOGIN__: JSON.stringify(env.DEV_LOGIN === "true"),
    },
    server: {
      port: 5173,
    },
  };
});
