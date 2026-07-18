import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App.jsx";
import { AuthProvider } from "./AuthContext.jsx";
import { PlayerProvider } from "./PlayerContext.jsx";
// Shared styles, loaded first so component/page styles can build on them.
import "./styles/base.css";
import "./styles/buttons.css";
import "./styles/layout.css";
import "./styles/pool.css";

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <BrowserRouter>
      <AuthProvider>
        <PlayerProvider>
          <App />
        </PlayerProvider>
      </AuthProvider>
    </BrowserRouter>
  </React.StrictMode>
);
