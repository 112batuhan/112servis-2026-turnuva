import { useEffect, useRef, useState } from "react";
import "./Countdown.css";

interface Props {
  /** ISO time to count down to. */
  to: string;
  /** Text after the time left, e.g. "kaldı". */
  suffix?: string;
  className?: string;
  /** Called once when the countdown reaches zero while on screen. */
  onElapsed?: () => void;
}

/** Live time left until `to`, ticking every second. Renders nothing once it's reached. */
export function Countdown({ to, suffix, className, onElapsed }: Props) {
  const target = new Date(to).getTime();
  const [now, setNow] = useState(() => Date.now());
  const elapsed = now >= target;

  useEffect(() => {
    if (elapsed) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [elapsed]);

  // Only a countdown that was running when it hit zero reports it, so a time that
  // had already passed on mount can't trigger a reload loop.
  const wasElapsed = useRef(elapsed);
  const onElapsedRef = useRef(onElapsed);
  useEffect(() => {
    onElapsedRef.current = onElapsed;
  });
  useEffect(() => {
    if (elapsed && !wasElapsed.current) onElapsedRef.current?.();
    wasElapsed.current = elapsed;
  }, [elapsed]);

  if (elapsed) return null;
  return (
    <span className={className ? `countdown ${className}` : "countdown"} role="timer">
      {formatTimeLeft(target - now)}
      {suffix && ` ${suffix}`}
    </span>
  );
}

/** 93784000 -> "1g 02sa 03dk"; seconds only show in the last hour, and leading zero
    units are dropped. */
function formatTimeLeft(ms: number): string {
  const total = Math.ceil(ms / 1000);
  const days = Math.floor(total / 86400);
  const hours = Math.floor((total % 86400) / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  if (days > 0) return `${days}g ${pad(hours)}sa ${pad(minutes)}dk`;
  if (hours > 0) return `${hours}sa ${pad(minutes)}dk`;
  return `${minutes}dk ${pad(seconds)}sn`;
}
