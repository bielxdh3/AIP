import { describe, expect, it } from "vitest";
import {
  alphaPixelsToRegions,
  buildBubbleInteractiveRegions,
  buildInteractiveRegions,
  bubbleWindowSize,
} from "./overlay-input";

function rgbaMask(rows: string[]): Uint8ClampedArray {
  const rowWidth = rows[0]?.length ?? 0;
  const pixels = new Uint8ClampedArray(rows.length * rowWidth * 4);
  rows.forEach((row, y) =>
    [...row].forEach((value, x) => {
      pixels[(y * row.length + x) * 4 + 3] = value === "#" ? 255 : 0;
    }),
  );
  return pixels;
}

describe("sprite alpha regions", () => {
  it("keeps painted pixels interactive and transparent sprite pixels absent", () => {
    const pixels = rgbaMask(["....", ".##.", ".##.", "...."]);
    const regions = alphaPixelsToRegions(pixels, 4, 4);
    expect(regions).toEqual([{ x: 1, y: 1, width: 2, height: 2 }]);

    const projected = buildInteractiveRegions(
      { width: 4, height: 4, regions },
      { x: 10, y: 20, width: 40, height: 40 },
      null,
      null,
    );
    expect(projected).toEqual([{ x: 20, y: 30, width: 20, height: 20 }]);
  });

  it("preserves transparent holes and disconnected painted regions", () => {
    const regions = alphaPixelsToRegions(
      rgbaMask(["####", "#..#", "####"]),
      4,
      3,
    );
    expect(regions).toEqual([
      { x: 0, y: 0, width: 4, height: 1 },
      { x: 0, y: 1, width: 1, height: 1 },
      { x: 3, y: 1, width: 1, height: 1 },
      { x: 0, y: 2, width: 4, height: 1 },
    ]);
    expect(
      buildInteractiveRegions(
        { width: 4, height: 3, regions },
        { x: 0, y: 0, width: 40, height: 30 },
        null,
        null,
      ),
    ).not.toContainEqual({ x: 10, y: 10, width: 20, height: 10 });
  });

  it("does not turn an empty alpha mask into a full sprite hitbox", () => {
    expect(
      buildInteractiveRegions(
        { width: 64, height: 64, regions: [] },
        { x: 10, y: 20, width: 128, height: 128 },
        null,
        null,
      ),
    ).toEqual([]);
  });

  it("derives selection regions from visible sprite pixels, not the WebView box", () => {
    const regions = buildInteractiveRegions(
      { width: 4, height: 4, regions: [{ x: 1, y: 1, width: 1, height: 1 }] },
      { x: 30, y: 40, width: 128, height: 128 },
      null,
      null,
    );
    expect(regions).toEqual([{ x: 62, y: 72, width: 32, height: 32 }]);
    expect(regions).not.toContainEqual({
      x: 0,
      y: 0,
      width: 180,
      height: 192,
    });
  });

  it("adds label and only the currently visible thought rectangle", () => {
    const label = { x: 5, y: 6, width: 20, height: 8 };
    const thought = { x: 30, y: 2, width: 16, height: 10 };
    expect(buildInteractiveRegions(null, null, label, null)).toEqual([label]);
    expect(buildInteractiveRegions(null, null, label, thought)).toEqual([
      label,
      thought,
    ]);
  });

  it("rejects malformed masks instead of falling back to the full image box", () => {
    expect(alphaPixelsToRegions(new Uint8ClampedArray(3), 4, 4)).toEqual([]);
  });

  it("uses the explicit alpha threshold", () => {
    const pixels = new Uint8ClampedArray(8);
    pixels[3] = 127;
    pixels[7] = 128;
    expect(alphaPixelsToRegions(pixels, 2, 1)).toEqual([
      { x: 1, y: 0, width: 1, height: 1 },
    ]);
  });

  it("unites visible custom pixels with the base mask at the alpha threshold", () => {
    expect(
      buildInteractiveRegions(
        { width: 4, height: 3, regions: [{ x: 0, y: 0, width: 1, height: 1 }] },
        { x: 10, y: 20, width: 40, height: 30 },
        null,
        null,
        [
          { x: 1, y: 1, color: "#fff" },
          { x: 2, y: 1, color: "#ffffff80" },
          { x: 0, y: 2, color: "#ffffff7f" },
          { x: 3, y: 2, color: "#f00" },
        ],
      ),
    ).toEqual([
      { x: 10, y: 20, width: 10, height: 10 },
      { x: 20, y: 30, width: 20, height: 10 },
      { x: 40, y: 40, width: 10, height: 10 },
    ]);
  });

  it("adds and removes the bubble region with visibility", () => {
    const bounds = { x: 8, y: 8, width: 344, height: 92 };
    expect(buildBubbleInteractiveRegions(true, bounds)).toEqual([bounds]);
    expect(buildBubbleInteractiveRegions(false, bounds)).toEqual([]);
    expect(buildBubbleInteractiveRegions(true, null)).toEqual([]);
  });

  it("keeps native bubble geometry tied to the visible DOM bounds", () => {
    expect(bubbleWindowSize({ x: 8, y: 8, width: 344, height: 92 })).toEqual({
      width: 360,
      height: 108,
    });
    expect(bubbleWindowSize({ x: 8, y: 8, width: 0, height: 92 })).toBeNull();
    expect(
      bubbleWindowSize({ x: 8, y: 8, width: 5000, height: 92 }),
    ).toBeNull();
  });
});
