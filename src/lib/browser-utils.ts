/**
 * Browser utility functions
 * Centralized helpers for browser name mapping, icons, etc.
 */

import { FaChrome, FaExclamationTriangle, FaFirefox } from "react-icons/fa";

/**
 * Map internal browser names to display names
 */
export function getBrowserDisplayName(browserType: string): string {
  const browserNames: Record<string, string> = {
    firefox: "Firefox",
    "firefox-developer": "Firefox Developer Edition",
    zen: "Zen Browser",
    brave: "Brave",
    chromium: "Chromium",
    camoufox: "Firefox (Camoufox)",
    wayfern: "Wayfern",
    orbita: "Orbita",
    cloakbrowser: "Foxia Browser",
  };

  return browserNames[browserType] || browserType;
}

/**
 * Get the appropriate icon component for a browser type
 * Anti-detect browsers get their base browser icons
 * Other browsers get a warning icon to indicate they're not anti-detect
 */
export function getBrowserIcon(browserType: string) {
  switch (browserType) {
    case "camoufox":
      return FaFirefox; // Firefox-based anti-detect browser
    case "wayfern":
    case "orbita":
    case "cloakbrowser":
      return FaChrome; // Chromium-based anti-detect browser
    default:
      // All other browsers get a warning icon
      return FaExclamationTriangle;
  }
}

export const getCurrentOS = () => {
  if (typeof window !== "undefined") {
    const userAgent = window.navigator.userAgent;
    if (userAgent.includes("Win")) return "windows";
    if (userAgent.includes("Mac")) return "macos";
    if (userAgent.includes("Linux")) return "linux";
  }
  return "unknown";
};

export function generateCloakUserAgent(
  platform: string,
  chromeVersion: string,
): string {
  const major = chromeVersion.split(".")[0] || "125";
  const full =
    chromeVersion.split(".").length >= 4 ? chromeVersion : `${major}.0.0.0`;

  switch (platform) {
    case "macos": {
      const macVer = `${10 + (Number(major) % 5)}_${Number(major) % 10}_0`;
      return `Mozilla/5.0 (Macintosh; Intel Mac OS X ${macVer}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${full} Safari/537.36`;
    }
    case "linux":
      return `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${full} Safari/537.36`;
    case "android":
      return `Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${full} Mobile Safari/537.36`;
    default: {
      const winVer = `10.0`;
      return `Mozilla/5.0 (Windows NT ${winVer}; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${full} Safari/537.36`;
    }
  }
}
