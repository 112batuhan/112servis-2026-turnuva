import { IconButton } from "./IconButton";

interface Props {
  message: string | null;
  onDismiss?: () => void;
}

export function ErrorAlert({ message, onDismiss }: Props) {
  if (!message) return null;
  return (
    <div className="alert alert-error" role="alert">
      <span className="alert-message">{message}</span>
      {onDismiss && <IconButton icon="close" label="Hatayı kapat" onClick={onDismiss} />}
    </div>
  );
}
