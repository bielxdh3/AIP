import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { forwardRef, useEffect, useState } from "react";
import astraSprite from "../assets/agent-astra.svg";
import lumaSprite from "../assets/agent-luma.svg";

const sprites = { astra: astraSprite, luma: lumaSprite } as const;

type AgentSpriteProps = {
  agentId: string;
  spriteKey: keyof typeof sprites;
  name: string;
  onLoad?: (image: HTMLImageElement) => void;
  onPixelsChange?: (pixels: PixelOverlay[]) => void;
};

export type PixelOverlay = { x: number; y: number; color: string };

export function pixelOverlays(source: string): PixelOverlay[] {
  try {
    const parsed = JSON.parse(source) as {
      layers?: Array<{
        visible?: boolean;
        pixels?: Array<[number, number, string]>;
      }>;
    };
    return (parsed.layers ?? []).flatMap((layer) =>
      layer.visible === false
        ? []
        : (layer.pixels ?? []).flatMap(([x, y, color]) =>
            Number.isInteger(x) &&
            Number.isInteger(y) &&
            x >= 0 &&
            x < 64 &&
            y >= 0 &&
            y < 64 &&
            /^#(?:[0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(
              color,
            )
              ? [{ x, y, color }]
              : [],
          ),
    );
  } catch {
    return [];
  }
}

const AgentSprite = forwardRef<HTMLImageElement, AgentSpriteProps>(
  function AgentSprite(
    { agentId, spriteKey, name, onLoad, onPixelsChange },
    ref,
  ) {
    const [pixels, setPixels] = useState<PixelOverlay[]>([]);

    useEffect(() => {
      onPixelsChange?.(pixels);
    }, [onPixelsChange, pixels]);

    useEffect(() => {
      let active = true;
      const refresh = () =>
        void invoke<string>("load_pixel_document", { agentId })
          .then((source) => active && setPixels(pixelOverlays(source)))
          .catch(() => active && setPixels([]));
      refresh();
      const unlisten = listen<string>("pixel-document-updated", (event) => {
        if (event.payload === agentId) refresh();
      });
      return () => {
        active = false;
        void unlisten.then((dispose) => dispose());
      };
    }, [agentId]);

    return (
      <span className="agent-sprite">
        <img
          ref={ref}
          className="agent-sprite-base"
          src={sprites[spriteKey]}
          width="64"
          height="64"
          alt={`Visual de ${name}`}
          draggable="false"
          onLoad={(event) => onLoad?.(event.currentTarget)}
        />
        {pixels.length > 0 ? (
          <svg
            className="agent-sprite-custom"
            viewBox="0 0 64 64"
            aria-hidden="true"
          >
            {pixels.map((pixel) => (
              <rect
                key={`${pixel.x}:${pixel.y}:${pixel.color}`}
                x={pixel.x}
                y={pixel.y}
                width="1"
                height="1"
                fill={pixel.color}
              />
            ))}
          </svg>
        ) : null}
      </span>
    );
  },
);

export default AgentSprite;
