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

export type CustomSpritePixel = {
  x: number;
  y: number;
  color: string;
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
  customPixels: readonly CustomSpritePixel[] = [],
): OverlayInteractiveRegion[] {
  const regions: OverlayInteractiveRegion[] = [];
  const spriteMask =
    mask ??
    (customPixels.length > 0
      ? { width: 64, height: 64, regions: [] as PixelRegion[] }
      : null);
  if (spriteMask !== null && spriteBounds !== null) {
    for (const pixelRegion of mergeCustomSpriteRegions(
      spriteMask,
      customPixels,
    )) {
      regions.push({
        x:
          spriteBounds.x +
          (pixelRegion.x / spriteMask.width) * spriteBounds.width,
        y:
          spriteBounds.y +
          (pixelRegion.y / spriteMask.height) * spriteBounds.height,
        width: (pixelRegion.width / spriteMask.width) * spriteBounds.width,
        height: (pixelRegion.height / spriteMask.height) * spriteBounds.height,
      });
    }
  }
  for (const bounds of [labelBounds, thoughtBounds]) {
    const valid = normalizeBounds(bounds);
    if (valid !== null) regions.push(valid);
  }
  return regions;
}

function mergeCustomSpriteRegions(
  mask: SpriteMask,
  customPixels: readonly CustomSpritePixel[],
): PixelRegion[] {
  if (customPixels.length === 0) return mask.regions;

  const rgba = new Uint8ClampedArray(mask.width * mask.height * 4);
  for (const region of mask.regions) {
    const startX = Math.max(0, Math.floor(region.x));
    const endX = Math.min(mask.width, Math.ceil(region.x + region.width));
    const startY = Math.max(0, Math.floor(region.y));
    const endY = Math.min(mask.height, Math.ceil(region.y + region.height));
    for (let y = startY; y < endY; y += 1) {
      for (let x = startX; x < endX; x += 1) {
        rgba[(y * mask.width + x) * 4 + 3] = 255;
      }
    }
  }
  for (const pixel of customPixels) {
    if (
      !Number.isInteger(pixel.x) ||
      !Number.isInteger(pixel.y) ||
      pixel.x < 0 ||
      pixel.x >= mask.width ||
      pixel.y < 0 ||
      pixel.y >= mask.height ||
      colorAlpha(pixel.color) < SPRITE_ALPHA_THRESHOLD
    ) {
      continue;
    }
    rgba[(pixel.y * mask.width + pixel.x) * 4 + 3] = 255;
  }
  return alphaPixelsToRegions(rgba, mask.width, mask.height);
}

function colorAlpha(color: string): number {
  if (/^#[0-9a-f]{3}$/i.test(color) || /^#[0-9a-f]{6}$/i.test(color))
    return 255;
  if (/^#[0-9a-f]{4}$/i.test(color))
    return Number.parseInt(color.slice(4), 16) * 17;
  if (/^#[0-9a-f]{8}$/i.test(color)) return Number.parseInt(color.slice(7), 16);
  return 0;
}

export function elementBounds(element: HTMLElement | null): RectLike | null {
  return element === null
    ? null
    : normalizeBounds(element.getBoundingClientRect());
}

export function buildBubbleInteractiveRegions(
  visible: boolean,
  bounds: RectLike | null,
): OverlayInteractiveRegion[] {
  if (!visible) return [];
  const normalized = normalizeBounds(bounds);
  return normalized === null ? [] : [normalized];
}

export function bubbleWindowSize(
  bounds: RectLike | null,
): { width: number; height: number } | null {
  const normalized = normalizeBounds(bounds);
  if (normalized === null) return null;
  const width = Math.ceil(normalized.x + normalized.width + 8);
  const height = Math.ceil(normalized.y + normalized.height + 8);
  return width > 0 && height > 0 && width <= 4096 && height <= 4096
    ? { width, height }
    : null;
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
