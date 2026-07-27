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

export function floodFillLayer(
  layer: PixelLayer,
  startX: number,
  startY: number,
  replacement: string,
): PixelLayer {
  if (layer.locked || startX < 0 || startX >= 64 || startY < 0 || startY >= 64) return layer;
  const colors = new Map(layer.pixels.map(([x, y, color]) => [`${x},${y}`, color]));
  const target = colors.get(`${startX},${startY}`) ?? null;
  if (target === replacement) return layer;
  const pending: Pixel[] = [[startX, startY, replacement]];
  const visited = new Set<string>();
  while (pending.length > 0) {
    const [x, y] = pending.pop()!;
    const key = `${x},${y}`;
    if (visited.has(key) || (colors.get(key) ?? null) !== target) continue;
    visited.add(key);
    colors.set(key, replacement);
    const neighbors: Array<[number, number]> = [[x - 1, y], [x + 1, y], [x, y - 1], [x, y + 1]];
    for (const [nextX, nextY] of neighbors) {
      if (nextX >= 0 && nextX < 64 && nextY >= 0 && nextY < 64) pending.push([nextX, nextY, replacement]);
    }
  }
  return {
    ...layer,
    pixels: [...colors.entries()].map(([key, color]) => {
      const [x, y] = key.split(",").map(Number);
      return [x, y, color] as Pixel;
    }),
  };
}

export function rasterLine(
  fromX: number,
  fromY: number,
  toX: number,
  toY: number,
): Array<[number, number]> {
  const cells: Array<[number, number]> = [];
  let x = fromX;
  let y = fromY;
  const dx = Math.abs(toX - fromX);
  const sx = fromX < toX ? 1 : -1;
  const dy = -Math.abs(toY - fromY);
  const sy = fromY < toY ? 1 : -1;
  let error = dx + dy;
  while (true) {
    if (x >= 0 && x < 64 && y >= 0 && y < 64) cells.push([x, y]);
    if (x === toX && y === toY) break;
    const twice = 2 * error;
    if (twice >= dy) { error += dy; x += sx; }
    if (twice <= dx) { error += dx; y += sy; }
  }
  return cells;
}
