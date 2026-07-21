import { forwardRef } from "react";
import astraSprite from "../assets/agent-astra.svg";
import lumaSprite from "../assets/agent-luma.svg";

const sprites = { astra: astraSprite, luma: lumaSprite } as const;

type AgentSpriteProps = {
  spriteKey: keyof typeof sprites;
  name: string;
  onLoad?: (image: HTMLImageElement) => void;
};

const AgentSprite = forwardRef<HTMLImageElement, AgentSpriteProps>(
  function AgentSprite({ spriteKey, name, onLoad }, ref) {
    return (
      <img
        ref={ref}
        className="agent-sprite"
        src={sprites[spriteKey]}
        width="64"
        height="64"
        alt={`Visual provisório de ${name}`}
        draggable="false"
        onLoad={(event) => onLoad?.(event.currentTarget)}
      />
    );
  },
);

export default AgentSprite;
