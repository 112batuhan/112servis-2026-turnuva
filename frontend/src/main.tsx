import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { App } from "./App";
import { AuthProvider } from "./auth/AuthContext";
import { LiveProvider } from "./live/LiveContext";
import { PlayerProvider } from "./player/PlayerContext";
import "./styles/fonts.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/controls.css";
import "./styles/layout.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <AuthProvider>
        <LiveProvider>
          <PlayerProvider>
            <App />
          </PlayerProvider>
        </LiveProvider>
      </AuthProvider>
    </BrowserRouter>
  </React.StrictMode>,
);
