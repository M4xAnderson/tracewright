import "@testing-library/jest-dom/vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// React Flow measures its viewport and nodes; jsdom has no layout. These stand-ins follow React
// Flow's testing guide: every element reports the size its style gives it (a 1440 × 900 window by
// default), and a resize observer reports each element once when it starts observing.
class ResizeObserverStandIn {
  readonly #callback: ResizeObserverCallback;
  constructor(callback: ResizeObserverCallback) {
    this.#callback = callback;
  }
  observe(target: Element) {
    this.#callback([{ target } as ResizeObserverEntry], this);
  }
  unobserve() {}
  disconnect() {}
}

class DOMMatrixReadOnlyStandIn {
  m22: number;
  constructor(transform?: string) {
    const scale = transform?.match(/scale\(([\d.]+)\)/)?.[1];
    this.m22 = scale === undefined ? 1 : Number(scale);
  }
}

globalThis.ResizeObserver = ResizeObserverStandIn;
Object.assign(globalThis, { DOMMatrixReadOnly: DOMMatrixReadOnlyStandIn });
// A size in px from the element's inline style, else the window's.
function styleSize(value: string, window: number): number {
  return value.endsWith("px") ? Number.parseFloat(value) : window;
}
Object.defineProperties(HTMLElement.prototype, {
  offsetHeight: {
    get(this: HTMLElement) {
      return styleSize(this.style.height, 900);
    },
  },
  offsetWidth: {
    get(this: HTMLElement) {
      return styleSize(this.style.width, 1440);
    },
  },
});
Object.assign(SVGElement.prototype, {
  getBBox: () => ({ x: 0, y: 0, width: 0, height: 0 }),
});

afterEach(() => {
  cleanup();
  clearMocks();
});
