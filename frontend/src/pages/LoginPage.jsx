import { loginWithOsu } from "../api.js";

export default function LoginPage() {
  return (
    <div className="page">
      <div className="card">
        <h1>Sign in</h1>
        <p className="subtitle">Log in with your osu! account to continue.</p>
        <button className="osu-btn" onClick={loginWithOsu}>
          Log in with osu!
        </button>
      </div>
    </div>
  );
}
