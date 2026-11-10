import { useSearchParams } from "react-router-dom";
import { LOGIN_ERROR_TEXT, textFor } from "../api/errors";
import { loginWithOsu } from "../api/auth";
import { useAuth } from "../auth/AuthContext";
import { IconButton } from "./IconButton";
import "./AuthNotice.css";

/**
 * Says, over the page, that the session ran out, or why the osu! login came back without
 * signing in (`?login_error=`, set by the backend's callback).
 */
export function AuthNotice() {
  const { sessionExpired, dismissSessionExpired } = useAuth();
  const [params, setParams] = useSearchParams();
  const loginError = params.get("login_error");

  const dropLoginError = () => {
    setParams(
      (current) => {
        const next = new URLSearchParams(current);
        next.delete("login_error");
        return next;
      },
      { replace: true },
    );
  };

  if (loginError) {
    return (
      <div className="auth-notice alert alert-error" role="alert">
        <span className="alert-message">{textFor(LOGIN_ERROR_TEXT, loginError, "failed")}</span>
        <IconButton icon="close" label="Kapat" onClick={dropLoginError} />
      </div>
    );
  }
  if (sessionExpired) {
    return (
      <div className="auth-notice alert alert-error" role="alert">
        <span className="alert-message">Oturumun sona erdi; devam etmek için yeniden giriş yap.</span>
        <button type="button" className="btn btn-secondary btn-sm" onClick={loginWithOsu}>
          Giriş yap
        </button>
        <IconButton icon="close" label="Kapat" onClick={dismissSessionExpired} />
      </div>
    );
  }
  return null;
}
