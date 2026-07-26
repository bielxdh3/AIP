import { describe, expect, it } from "vitest";
import { floodFillLayer, nextLayerId, parsePixelDocument, updatePixelLayer } from "./pixel-document";

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
