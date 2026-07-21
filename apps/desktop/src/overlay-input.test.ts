import { describe, expect, it } from "vitest";
import { alphaPixelsToRegions, buildInteractiveRegions } from "./overlay-input";

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
});
