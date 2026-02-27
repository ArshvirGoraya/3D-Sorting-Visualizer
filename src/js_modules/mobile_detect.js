// use separate file and import only necessary things for tree-shaking

import Bowser from "bowser";

export function isMobileDevice() {
  const parser = Bowser.getParser(window.navigator.userAgent);
  const type = parser.getPlatformType(); // "mobile", "tablet", "desktop"
  return type === "mobile" || type === "tablet";
}

// @ts-ignore
window.isMobileDevice = isMobileDevice; // used in rust with wasm_bindgen
