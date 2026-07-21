import type { OverlayInteractiveRegion } from "@aip/contracts";

export const SPRITE_ALPHA_THRESHOLD = 128;

export type PixelRegion = OverlayInteractiveRegion;

export type SpriteMask = {
  width: number;
  height: number;
  regions: PixelRegion[];
};

type RectLike = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export function readSpriteMask(image: HTMLImageElement): SpriteMask | null {
  const width = image.naturalWidth;
  const height = image.naturalHeight;
  if (width <= 0 || height <= 0) return null;

  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (context === null) return null;
  let pixels: Uint8ClampedArray;
  try {
    context.clearRect(0, 0, width, height);
    context.drawImage(image, 0, 0, width, height);
    pixels = context.getImageData(0, 0, width, height).data;
  } catch {
    return null;
  }
  return {
    width,
    height,
    regions: alphaPixelsToRegions(pixels, width, height),
  };
}

export function alphaPixelsToRegions(
  rgba: ArrayLike<number>,
  width: number,
  height: number,
  threshold = SPRITE_ALPHA_THRESHOLD,
): PixelRegion[] {
  if (
    !Number.isInteger(width) ||
    !Number.isInteger(height) ||
    width <= 0 ||
    height <= 0 ||
    rgba.length !== width * height * 4
  ) {
    return [];
  }

  const completed: PixelRegion[] = [];
  let active = new Map<string, PixelRegion>();

  for (let y = 0; y < height; y += 1) {
    const runs: Array<{ x: number; width: number }> = [];
    let x = 0;
    while (x < width) {
      while (x < width && (rgba[(y * width + x) * 4 + 3] ?? 0) < threshold)
        x += 1;
      const start = x;
      while (x < width && (rgba[(y * width + x) * 4 + 3] ?? 0) >= threshold)
        x += 1;
      if (x > start) runs.push({ x: start, width: x - start });
    }

    const next = new Map<string, PixelRegion>();
    for (const run of runs) {
      const key = `${run.x}:${run.width}`;
      const previous = active.get(key);
      next.set(
        key,
        previous === undefined
          ? { x: run.x, y, width: run.width, height: 1 }
          : { ...previous, height: previous.height + 1 },
      );
    }
    for (const [key, region] of active) {
      if (!next.has(key)) completed.push(region);
    }
    active = next;
  }
  completed.push(...active.values());
  return completed;
}

export function buildInteractiveRegions(
  mask: SpriteMask | null,
  spriteBounds: RectLike | null,
  labelBounds: RectLike | null,
  thoughtBounds: RectLike | null,
): OverlayInteractiveRegion[] {
  const regions: OverlayInteractiveRegion[] = [];
  if (mask !== null && spriteBounds !== null) {
    for (const pixelRegion of mask.regions) {
      regions.push({
        x: spriteBounds.x + (pixelRegion.x / mask.width) * spriteBounds.width,
        y: spriteBounds.y + (pixelRegion.y / mask.height) * spriteBounds.height,
        width: (pixelRegion.width / mask.width) * spriteBounds.width,
        height: (pixelRegion.height / mask.height) * spriteBounds.height,
      });
    }
  }
  for (const bounds of [labelBounds, thoughtBounds]) {
    const valid = normalizeBounds(bounds);
    if (valid !== null) regions.push(valid);
  }
  return regions;
}

export function elementBounds(element: HTMLElement | null): RectLike | null {
  return element === null
    ? null
    : normalizeBounds(element.getBoundingClientRect());
}

function normalizeBounds(
  bounds: RectLike | null,
): OverlayInteractiveRegion | null {
  if (
    bounds === null ||
    ![bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite) ||
    bounds.x < 0 ||
    bounds.y < 0 ||
    bounds.width <= 0 ||
    bounds.height <= 0
  ) {
    return null;
  }
  return {
    x: bounds.x,
    y: bounds.y,
    width: bounds.width,
    height: bounds.height,
  };
}
