import astraSprite from "../assets/agent-astra.svg";
import lumaSprite from "../assets/agent-luma.svg";

const sprites = { astra: astraSprite, luma: lumaSprite } as const;

export default function AgentSprite({
  spriteKey,
  name,
}: {
  spriteKey: keyof typeof sprites;
  name: string;
}) {
  return (
    <img
      className="agent-sprite"
      src={sprites[spriteKey]}
      width="64"
      height="64"
      alt={`Visual provisório de ${name}`}
      draggable="false"
    />
  );
}
