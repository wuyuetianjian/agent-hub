import type { Provider } from "../types";
export function AgentIcon({
  provider,
  size = 32,
}: {
  provider: Provider;
  size?: number;
}) {
  const color = ["claude", "gemini", "antigravity", "kiro"].includes(
    provider.id,
  );
  return (
    <span
      className="agent-icon"
      style={
        {
          width: size,
          height: size,
          "--agent-color": provider.color,
        } as React.CSSProperties
      }
    >
      <img
        src={provider.icon}
        width={size * 0.68}
        height={size * 0.68}
        className={color ? "color-mark" : "mono-mark"}
        alt={`${provider.name} icon`}
        draggable={false}
      />
    </span>
  );
}
