import type { CanvasKit, Image } from 'canvaskit-wasm';
import type { LayerImageOp } from '../../core/types.ts';
import { applyLayerImageEffectPixels } from '../image-effect-pixels.ts';

export function canvasKitImageEffectKey(op: LayerImageOp): string | null {
  if (op.bakedWatermark) return '';
  const effect = op.effect ?? 'realPic';
  const brightness = op.brightness ?? 0;
  const contrast = op.contrast ?? 0;
  if (!['realPic', 'grayScale', 'blackWhite', 'pattern8x8'].includes(effect)
    || ![brightness, contrast].every(value => Number.isFinite(value) && value >= -100 && value <= 100)) return null;
  return effect === 'realPic' && brightness === 0 && contrast === 0
    ? '' : JSON.stringify([effect, brightness, contrast]);
}

/** 원본 alpha를 유지한 unpremultiplied 픽셀 변환. 실패하면 원본을 성공으로 반환하지 않는다. */
export function makeCanvasKitEffectImage(kit: CanvasKit, original: Image, op: LayerImageOp): Image | null {
  const width = original.width();
  const height = original.height();
  const info = {
    width, height,
    colorType: kit.ColorType.RGBA_8888,
    alphaType: kit.AlphaType.Unpremul,
    colorSpace: kit.ColorSpace.SRGB,
  };
  try {
    const pixels = original.readPixels(0, 0, info);
    if (!(pixels instanceof Uint8Array) || pixels.length !== width * height * 4) return null;
    applyLayerImageEffectPixels(pixels, width, op.effect, 0, 0, op.brightness ?? 0, op.contrast ?? 0);
    return kit.MakeImage(info, pixels, width * 4);
  } catch {
    return null;
  }
}
