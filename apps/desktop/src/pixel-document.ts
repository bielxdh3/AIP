export type Pixel = [number, number, string];

export type PixelLayer = {
  id: string;
  name: string;
  visible: boolean;
  locked: boolean;
  pixels: Pixel[];
};

export type PixelDocument = {
  layers: PixelLayer[];
  attachmentPoints: Record<string, { x: number; y: number }>;
};

export function parsePixelDocument(source: string): PixelDocument | null {
  try {
    const value = JSON.parse(source) as Partial<PixelDocument>;
    if (!Array.isArray(value.layers) || value.layers.length === 0) return null;
    return {
      layers: value.layers.flatMap((layer, index) =>
        typeof layer?.id === "string" && Array.isArray(layer.pixels)
          ? [{
              id: layer.id,
              name: typeof layer.name === "string" ? layer.name : `Layer ${index + 1}`,
              visible: layer.visible !== false,
              locked: layer.locked === true,
              pixels: layer.pixels.filter(
                (pixel): pixel is Pixel =>
                  Array.isArray(pixel) &&
                  pixel.length === 3 &&
                  Number.isInteger(pixel[0]) &&
                  Number.isInteger(pixel[1]) &&
                  typeof pixel[2] === "string",
              ),
            }]
          : [],
      ),
      attachmentPoints:
        value.attachmentPoints && typeof value.attachmentPoints === "object"
          ? value.attachmentPoints
          : {},
    };
  } catch {
    return null;
  }
}

export function nextLayerId(document: PixelDocument): string {
  let index = document.layers.length + 1;
  while (document.layers.some((layer) => layer.id === `layer-${index}`)) index += 1;
  return `layer-${index}`;
}

export function updatePixelLayer(
  document: PixelDocument,
  layerId: string,
  update: (layer: PixelLayer) => PixelLayer,
): PixelDocument {
  return {
    ...document,
    layers: document.layers.map((layer) => (layer.id === layerId ? update(layer) : layer)),
  };
}
