import { describe, expect, it } from "vitest";
import { floodFillLayer, nextLayerId, paintPixelLayer, parsePixelDocument, rasterLine, rgbaToHex, selectionRectangle, updatePixelLayer } from "./pixel-document";

describe("pixel documents", () => {
  it("normalizes layers and updates only the selected layer", () => {
    const document = parsePixelDocument(
      '{"layers":[{"id":"body","pixels":[[1,2,"#fff"]]},{"id":"hair","name":"Hair","visible":false,"locked":true,"pixels":[]}],"attachmentPoints":{}}',
    );
    expect(document).not.toBeNull();
    if (document === null) return;
    expect(nextLayerId(document)).toBe("layer-3");
    expect(updatePixelLayer(document, "hair", (layer) => ({ ...layer, visible: true }))).toMatchObject({
      layers: [{ id: "body", visible: true }, { id: "hair", visible: true, locked: true }],
    });
  });

  it("keeps pixel edits immutable and isolates unrelated layers", () => {
    const body = {
      id: "body",
      name: "Body",
      visible: true,
      locked: false,
      pixels: [[1, 1, "#111111"]] as [number, number, string][],
    };
    const hair = {
      id: "hair",
      name: "Hair",
      visible: true,
      locked: false,
      pixels: [[2, 2, "#222222"]] as [number, number, string][],
    };
    const document = {
      layers: [body, hair],
      attachmentPoints: {},
      semanticParts: {},
    };
    const painted = paintPixelLayer(
      body,
      3,
      3,
      3,
      3,
      "pencil",
      "#57d8bd",
      false,
    );
    const updated = updatePixelLayer(document, "body", () => painted);

    expect(painted).not.toBe(body);
    expect(body.pixels).toEqual([[1, 1, "#111111"]]);
    expect(painted.pixels).toContainEqual([3, 3, "#57d8bd"]);
    expect(updated.layers[0]).toBe(painted);
    expect(updated.layers[1]).toBe(hair);
  });
});

it("rasterizes every cell crossed by a fast stroke", () => {
  expect(rasterLine(1, 1, 5, 3)).toEqual([[1, 1], [2, 2], [3, 2], [4, 3], [5, 3]]);
});

describe("pixel flood fill", () => {
  it("fills only the connected enclosed region", () => {
    const layer = {
      id: "body", name: "Body", visible: true, locked: false,
      pixels: [[0, 0, "#000"], [1, 0, "#000"], [2, 0, "#000"], [0, 1, "#000"], [2, 1, "#000"], [0, 2, "#000"], [1, 2, "#000"], [2, 2, "#000"], [4, 1, "#fff"]] as [number, number, string][],
    };
    const filled = floodFillLayer(layer, 1, 1, "#f00");
    expect(filled.pixels).toContainEqual([1, 1, "#f00"]);
    expect(filled.pixels).toContainEqual([4, 1, "#fff"]);
    expect(layer.pixels).not.toContainEqual([1, 1, "#f00"]);
  });

  it("keeps boundaries, identical colors, locked layers, and other layers untouched", () => {
    const layer = { id: "body", name: "Body", visible: true, locked: false, pixels: [[1, 1, "#abc"]] as [number, number, string][] };
    expect(floodFillLayer(layer, 1, 1, "#abc")).toBe(layer);
    expect(floodFillLayer({ ...layer, locked: true }, 1, 1, "#def")).toMatchObject({ pixels: [[1, 1, "#abc"]] });
    expect(floodFillLayer(layer, -1, 1, "#def")).toBe(layer);
  });
});

describe("pixel alpha and selection helpers", () => {
  it("preserves PNG alpha while keeping opaque pixels compact", () => {
    expect(rgbaToHex(0x11, 0x22, 0x33, 0)).toBeNull();
    expect(rgbaToHex(0x11, 0x22, 0x33, 255)).toBe("#112233");
    expect(rgbaToHex(0x11, 0x22, 0x33, 128)).toBe("#11223380");
  });

  it("anchors rectangles at pointer-down and clamps both ends to the canvas", () => {
    expect(selectionRectangle(2, 3, 5, 7)).toEqual({
      x: 2,
      y: 3,
      width: 4,
      height: 5,
    });
    expect(selectionRectangle(5, 7, 2, 3)).toEqual({
      x: 2,
      y: 3,
      width: 4,
      height: 5,
    });
    expect(selectionRectangle(31, 31, 32, 32)).toEqual({
      x: 31,
      y: 31,
      width: 2,
      height: 2,
    });
    expect(selectionRectangle(-4, 70, 66, -2)).toEqual({
      x: 0,
      y: 0,
      width: 64,
      height: 64,
    });
  });
});
