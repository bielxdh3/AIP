import { describe, expect, it } from "vitest";
import { nextLayerId, parsePixelDocument, updatePixelLayer } from "./pixel-document";

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
