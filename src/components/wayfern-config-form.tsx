"use client";

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { HiSparkles } from "react-icons/hi2";
import { LuLock } from "react-icons/lu";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import type {
  WayfernConfig,
  WayfernFingerprintConfig,
  WayfernOS,
} from "@/types";

interface WayfernConfigFormProps {
  config: WayfernConfig;
  onConfigChange: (key: keyof WayfernConfig, value: unknown) => void;
  className?: string;
  isCreating?: boolean;
  readOnly?: boolean;
}

const isFingerprintEditingDisabled = (_config: WayfernConfig): boolean => {
  return false;
};

const getCurrentOS = (): WayfernOS => {
  if (typeof navigator === "undefined") return "linux";
  const platform = navigator.platform.toLowerCase();
  if (platform.includes("win")) return "windows";
  if (platform.includes("mac")) return "macos";
  return "linux";
};

const osLabels: Record<WayfernOS, string> = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
  android: "Android",
  ios: "iOS",
};

export function WayfernConfigForm({
  config,
  onConfigChange,
  className = "",
  isCreating = false,
  readOnly = false,
}: WayfernConfigFormProps) {
  const { t } = useTranslation();
  const [fingerprintConfig, setFingerprintConfig] =
    useState<WayfernFingerprintConfig>({});
  const [currentOS] = useState<WayfernOS>(getCurrentOS);

  const selectedOS = config.os || currentOS;

  useEffect(() => {
    if (isCreating && typeof window !== "undefined") {
      const screenWidth = window.screen.width;
      const screenHeight = window.screen.height;

      if (!config.screen_max_width) {
        onConfigChange("screen_max_width", screenWidth);
      }
      if (!config.screen_max_height) {
        onConfigChange("screen_max_height", screenHeight);
      }
    }
  }, [
    isCreating,
    config.screen_max_width,
    config.screen_max_height,
    onConfigChange,
  ]);

  useEffect(() => {
    if (config.fingerprint) {
      try {
        const parsed = JSON.parse(
          config.fingerprint,
        ) as WayfernFingerprintConfig;
        setFingerprintConfig(parsed);
      } catch (error) {
        console.error("Failed to parse fingerprint config:", error);
        setFingerprintConfig({});
      }
    } else {
      setFingerprintConfig({});
    }
  }, [config.fingerprint]);

  const updateFingerprintConfig = (
    key: keyof WayfernFingerprintConfig,
    value: unknown,
  ) => {
    const newConfig = { ...fingerprintConfig };

    if (
      value === undefined ||
      value === "" ||
      (Array.isArray(value) && value.length === 0)
    ) {
      delete newConfig[key];
    } else {
      (newConfig as Record<string, unknown>)[key] = value;
    }

    setFingerprintConfig(newConfig);

    try {
      const jsonString = JSON.stringify(newConfig);
      onConfigChange("fingerprint", jsonString);
    } catch (error) {
      console.error("Failed to serialize fingerprint config:", error);
    }
  };

  const _isAutoLocationEnabled = config.geoip !== false;

  const _handleAutoLocationToggle = (enabled: boolean) => {
    if (enabled) {
      onConfigChange("geoip", true);
    } else {
      onConfigChange("geoip", false);
    }
  };

  // Generate random user agent based on OS
  const generateRandomUserAgent = () => {
    const os = config.os || currentOS;

    // Random Chrome versions between 120-131
    const chromeVersion = Math.floor(Math.random() * 12) + 120;
    const chromePatch = Math.floor(Math.random() * 1000);

    // Random OS versions
    const windowsVersions = ["10.0", "11.0"];
    const macVersions = ["10_15_7", "11_0_0", "12_0_0", "13_0_0", "14_0_0"];

    let userAgent = "";
    let platform = "";

    if (os === "windows") {
      const winVersion =
        windowsVersions[Math.floor(Math.random() * windowsVersions.length)];
      userAgent = `Mozilla/5.0 (Windows NT ${winVersion}; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${chromeVersion}.0.0.${chromePatch} Safari/537.36`;
      platform = "Win32";
    } else if (os === "macos") {
      const macVersion =
        macVersions[Math.floor(Math.random() * macVersions.length)];
      userAgent = `Mozilla/5.0 (Macintosh; Intel Mac OS X ${macVersion}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${chromeVersion}.0.0.${chromePatch} Safari/537.36`;
      platform = "MacIntel";
    } else {
      // Linux, Android, iOS
      userAgent = `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${chromeVersion}.0.0.${chromePatch} Safari/537.36`;
      platform = "Linux x86_64";
    }

    // Update both fields at once to avoid state update race condition
    const newConfig = { ...fingerprintConfig, userAgent, platform };
    setFingerprintConfig(newConfig);
    try {
      const jsonString = JSON.stringify(newConfig);
      onConfigChange("fingerprint", jsonString);
    } catch (error) {
      console.error("Failed to serialize fingerprint config:", error);
    }
  };

  const isEditingDisabled = isFingerprintEditingDisabled(config) || readOnly;

  const renderAdvancedForm = () => (
    <div className="space-y-6">
      {/* Operating System Selection */}
      <div className="space-y-3">
        <Label>{t("createProfile.fingerprint.osSelection.label")}</Label>
        <Select
          value={selectedOS}
          onValueChange={(value: WayfernOS) => onConfigChange("os", value)}
          disabled={readOnly}
        >
          <SelectTrigger>
            <SelectValue
              placeholder={t(
                "createProfile.fingerprint.osSelection.placeholder",
              )}
            />
          </SelectTrigger>
          <SelectContent>
            {(
              ["windows", "macos", "linux", "android", "ios"] as WayfernOS[]
            ).map((os) => {
              const isDisabled = os !== currentOS;
              return (
                <SelectItem key={os} value={os} disabled={isDisabled}>
                  <span className="flex items-center gap-2">
                    {osLabels[os]}
                    {isDisabled && (
                      <LuLock className="w-3 h-3 text-muted-foreground" />
                    )}
                  </span>
                </SelectItem>
              );
            })}
          </SelectContent>
        </Select>
      </div>

      {isEditingDisabled ? (
        <Alert>
          <AlertDescription>
            {t("createProfile.fingerprint.warnings.editingDisabled")}
          </AlertDescription>
        </Alert>
      ) : (
        <Alert>
          <AlertDescription>
            {t("createProfile.fingerprint.warnings.advancedWarning")}
          </AlertDescription>
        </Alert>
      )}

      <fieldset disabled={isEditingDisabled} className="space-y-6">
        {/* User Agent and Platform */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <Label>
              {t("createProfile.fingerprint.userAgentPlatform.title")}
            </Label>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={generateRandomUserAgent}
              disabled={readOnly}
              className="gap-2"
            >
              <HiSparkles className="w-4 h-4" />
              {t("createProfile.fingerprint.generateRandom")}
            </Button>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2 col-span-2">
              <Label htmlFor="user-agent">
                {t("createProfile.fingerprint.userAgentPlatform.userAgent")}
              </Label>
              <Input
                id="user-agent"
                value={fingerprintConfig.userAgent || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "userAgent",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.userAgentPlatform.userAgentPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="platform">
                {t("createProfile.fingerprint.userAgentPlatform.platform")}
              </Label>
              <Input
                id="platform"
                value={fingerprintConfig.platform || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "platform",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.userAgentPlatform.platformPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="platform-version">
                {t(
                  "createProfile.fingerprint.userAgentPlatform.platformVersion",
                )}
              </Label>
              <Input
                id="platform-version"
                value={fingerprintConfig.platformVersion || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "platformVersion",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.userAgentPlatform.platformVersionPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="brand">
                {t("createProfile.fingerprint.userAgentPlatform.brand")}
              </Label>
              <Input
                id="brand"
                value={fingerprintConfig.brand || ""}
                onChange={(e) =>
                  updateFingerprintConfig("brand", e.target.value || undefined)
                }
                placeholder={t(
                  "createProfile.fingerprint.userAgentPlatform.brandPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="brand-version">
                {t("createProfile.fingerprint.userAgentPlatform.brandVersion")}
              </Label>
              <Input
                id="brand-version"
                value={fingerprintConfig.brandVersion || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "brandVersion",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.userAgentPlatform.brandVersionPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* Hardware Properties */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.hardware.title")}</Label>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="hardware-concurrency">
                {t("createProfile.fingerprint.hardware.concurrency")}
              </Label>
              <Input
                id="hardware-concurrency"
                type="number"
                value={fingerprintConfig.hardwareConcurrency || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "hardwareConcurrency",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.hardware.concurrencyPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="max-touch-points">
                {t("createProfile.fingerprint.hardware.maxTouchPoints")}
              </Label>
              <Input
                id="max-touch-points"
                type="number"
                value={fingerprintConfig.maxTouchPoints || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "maxTouchPoints",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.hardware.maxTouchPointsPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="device-memory">
                {t("createProfile.fingerprint.hardware.deviceMemory")}
              </Label>
              <Input
                id="device-memory"
                type="number"
                value={fingerprintConfig.deviceMemory || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "deviceMemory",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.hardware.deviceMemoryPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* Screen Properties */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.screen.title")}</Label>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="screen-width">
                {t("createProfile.fingerprint.screen.width")}
              </Label>
              <Input
                id="screen-width"
                type="number"
                value={fingerprintConfig.screenWidth || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screenWidth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.widthPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="screen-height">
                {t("createProfile.fingerprint.screen.height")}
              </Label>
              <Input
                id="screen-height"
                type="number"
                value={fingerprintConfig.screenHeight || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screenHeight",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.heightPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="device-pixel-ratio">
                {t("createProfile.fingerprint.screen.pixelRatio")}
              </Label>
              <Input
                id="device-pixel-ratio"
                type="number"
                step="0.1"
                value={fingerprintConfig.devicePixelRatio || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "devicePixelRatio",
                    e.target.value ? parseFloat(e.target.value) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.pixelRatioPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="screen-avail-width">
                {t("createProfile.fingerprint.screen.availWidth")}
              </Label>
              <Input
                id="screen-avail-width"
                type="number"
                value={fingerprintConfig.screenAvailWidth || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screenAvailWidth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.availWidthPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="screen-avail-height">
                {t("createProfile.fingerprint.screen.availHeight")}
              </Label>
              <Input
                id="screen-avail-height"
                type="number"
                value={fingerprintConfig.screenAvailHeight || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screenAvailHeight",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.availHeightPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="screen-color-depth">
                {t("createProfile.fingerprint.screen.colorDepth")}
              </Label>
              <Input
                id="screen-color-depth"
                type="number"
                value={fingerprintConfig.screenColorDepth || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screenColorDepth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.colorDepthPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* Window Properties */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.window.title")}</Label>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="window-outer-width">
                {t("createProfile.fingerprint.window.outerWidth")}
              </Label>
              <Input
                id="window-outer-width"
                type="number"
                value={fingerprintConfig.windowOuterWidth || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "windowOuterWidth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.outerWidthPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="window-outer-height">
                {t("createProfile.fingerprint.window.outerHeight")}
              </Label>
              <Input
                id="window-outer-height"
                type="number"
                value={fingerprintConfig.windowOuterHeight || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "windowOuterHeight",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.outerHeightPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="window-inner-width">
                {t("createProfile.fingerprint.window.innerWidth")}
              </Label>
              <Input
                id="window-inner-width"
                type="number"
                value={fingerprintConfig.windowInnerWidth || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "windowInnerWidth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.innerWidthPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="window-inner-height">
                {t("createProfile.fingerprint.window.innerHeight")}
              </Label>
              <Input
                id="window-inner-height"
                type="number"
                value={fingerprintConfig.windowInnerHeight || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "windowInnerHeight",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.innerHeightPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="screen-x">
                {t("createProfile.fingerprint.window.screenX")}
              </Label>
              <Input
                id="screen-x"
                type="number"
                value={fingerprintConfig.screenX || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screenX",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.screenXPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="screen-y">
                {t("createProfile.fingerprint.window.screenY")}
              </Label>
              <Input
                id="screen-y"
                type="number"
                value={fingerprintConfig.screenY || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screenY",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.screenYPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* Language & Locale */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.language.title")}</Label>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="language">
                {t("createProfile.fingerprint.language.primary")}
              </Label>
              <Input
                id="language"
                value={fingerprintConfig.language || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "language",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.language.primaryPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="languages">
                {t("createProfile.fingerprint.language.languages")}
              </Label>
              <Input
                id="languages"
                value={
                  Array.isArray(fingerprintConfig.languages)
                    ? JSON.stringify(fingerprintConfig.languages)
                    : ""
                }
                onChange={(e) => {
                  if (!e.target.value) {
                    updateFingerprintConfig("languages", undefined);
                    return;
                  }
                  try {
                    const parsed = JSON.parse(e.target.value);
                    if (Array.isArray(parsed)) {
                      updateFingerprintConfig("languages", parsed);
                    }
                  } catch {
                    // Invalid JSON, keep current value
                  }
                }}
                placeholder={t(
                  "createProfile.fingerprint.language.languagesPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="do-not-track">
                {t("createProfile.fingerprint.language.doNotTrack")}
              </Label>
              <Select
                value={fingerprintConfig.doNotTrack || ""}
                onValueChange={(value) =>
                  updateFingerprintConfig("doNotTrack", value || undefined)
                }
              >
                <SelectTrigger>
                  <SelectValue
                    placeholder={t(
                      "createProfile.fingerprint.language.doNotTrackPlaceholder",
                    )}
                  />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="0">
                    {t(
                      "createProfile.fingerprint.language.doNotTrackOptions.allowed",
                    )}
                  </SelectItem>
                  <SelectItem value="1">
                    {t(
                      "createProfile.fingerprint.language.doNotTrackOptions.notAllowed",
                    )}
                  </SelectItem>
                  <SelectItem value="unspecified">
                    {t(
                      "createProfile.fingerprint.language.doNotTrackOptions.unspecified",
                    )}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>

        {/* Timezone and Geolocation */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.timezone.title")}</Label>
          <p className="text-sm text-muted-foreground">
            {t("createProfile.fingerprint.timezone.description")}
          </p>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="timezone">
                {t("createProfile.fingerprint.timezone.timezone")}
              </Label>
              <Input
                id="timezone"
                value={fingerprintConfig.timezone || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "timezone",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.timezone.timezonePlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="timezone-offset">
                {t("createProfile.fingerprint.timezone.offset")}
              </Label>
              <Input
                id="timezone-offset"
                type="number"
                value={fingerprintConfig.timezoneOffset ?? ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "timezoneOffset",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.timezone.offsetPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="latitude">
                {t("createProfile.fingerprint.timezone.latitude")}
              </Label>
              <Input
                id="latitude"
                type="number"
                step="any"
                value={fingerprintConfig.latitude || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "latitude",
                    e.target.value ? parseFloat(e.target.value) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.timezone.latitudePlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="longitude">
                {t("createProfile.fingerprint.timezone.longitude")}
              </Label>
              <Input
                id="longitude"
                type="number"
                step="any"
                value={fingerprintConfig.longitude || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "longitude",
                    e.target.value ? parseFloat(e.target.value) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.timezone.longitudePlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="accuracy">
                {t("createProfile.fingerprint.timezone.accuracy")}
              </Label>
              <Input
                id="accuracy"
                type="number"
                value={fingerprintConfig.accuracy || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "accuracy",
                    e.target.value ? parseFloat(e.target.value) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.timezone.accuracyPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* WebGL Properties */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.webgl.title")}</Label>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="webgl-vendor">
                {t("createProfile.fingerprint.webgl.vendor")}
              </Label>
              <Input
                id="webgl-vendor"
                value={fingerprintConfig.webglVendor || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "webglVendor",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.webgl.vendorPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="webgl-renderer">
                {t("createProfile.fingerprint.webgl.renderer")}
              </Label>
              <Input
                id="webgl-renderer"
                value={fingerprintConfig.webglRenderer || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "webglRenderer",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.webgl.rendererPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* WebGL Parameters (JSON) */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.webgl.parameters")}</Label>
          <Textarea
            value={fingerprintConfig.webglParameters || ""}
            onChange={(e) =>
              updateFingerprintConfig(
                "webglParameters",
                e.target.value || undefined,
              )
            }
            placeholder={t(
              "createProfile.fingerprint.webgl.parametersPlaceholder",
            )}
            className="font-mono text-sm"
            rows={4}
          />
        </div>

        {/* Canvas Noise Seed */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.canvas.title")}</Label>
          <div className="space-y-2">
            <Label htmlFor="canvas-noise-seed">
              {t("createProfile.fingerprint.canvas.noiseSeed")}
            </Label>
            <Input
              id="canvas-noise-seed"
              value={fingerprintConfig.canvasNoiseSeed || ""}
              onChange={(e) =>
                updateFingerprintConfig(
                  "canvasNoiseSeed",
                  e.target.value || undefined,
                )
              }
              placeholder={t(
                "createProfile.fingerprint.canvas.noiseSeedPlaceholder",
              )}
            />
            <p className="text-sm text-muted-foreground">
              {t("createProfile.fingerprint.canvas.noiseSeedDescription")}
            </p>
          </div>
        </div>

        {/* Fonts (JSON) */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.fonts.title")}</Label>
          <Textarea
            value={fingerprintConfig.fonts || ""}
            onChange={(e) =>
              updateFingerprintConfig("fonts", e.target.value || undefined)
            }
            placeholder={t("createProfile.fingerprint.fonts.placeholder")}
            className="font-mono text-sm"
            rows={3}
          />
        </div>

        {/* Audio */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.audio.title")}</Label>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="audio-sample-rate">
                {t("createProfile.fingerprint.audio.sampleRate")}
              </Label>
              <Input
                id="audio-sample-rate"
                type="number"
                value={fingerprintConfig.audioSampleRate || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "audioSampleRate",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.audio.sampleRatePlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="audio-max-channel-count">
                {t("createProfile.fingerprint.audio.maxChannelCount")}
              </Label>
              <Input
                id="audio-max-channel-count"
                type="number"
                value={fingerprintConfig.audioMaxChannelCount || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "audioMaxChannelCount",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.audio.maxChannelCountPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* Battery */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.battery.title")}</Label>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="battery-charging"
                  checked={fingerprintConfig.batteryCharging || false}
                  onCheckedChange={(checked) =>
                    updateFingerprintConfig(
                      "batteryCharging",
                      checked || undefined,
                    )
                  }
                />
                <Label htmlFor="battery-charging">
                  {t("createProfile.fingerprint.battery.charging")}
                </Label>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="battery-level">
                {t("createProfile.fingerprint.battery.level")}
              </Label>
              <Input
                id="battery-level"
                type="number"
                step="0.01"
                min="0"
                max="1"
                value={fingerprintConfig.batteryLevel || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "batteryLevel",
                    e.target.value ? parseFloat(e.target.value) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.battery.levelPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* Vendor Info */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.vendor.title")}</Label>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="vendor">
                {t("createProfile.fingerprint.vendor.vendor")}
              </Label>
              <Input
                id="vendor"
                value={fingerprintConfig.vendor || ""}
                onChange={(e) =>
                  updateFingerprintConfig("vendor", e.target.value || undefined)
                }
                placeholder={t(
                  "createProfile.fingerprint.vendor.vendorPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="vendor-sub">
                {t("createProfile.fingerprint.vendor.vendorSub")}
              </Label>
              <Input
                id="vendor-sub"
                value={fingerprintConfig.vendorSub || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "vendorSub",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.vendor.vendorSubPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="product-sub">
                {t("createProfile.fingerprint.vendor.productSub")}
              </Label>
              <Input
                id="product-sub"
                value={fingerprintConfig.productSub || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "productSub",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.vendor.productSubPlaceholder",
                )}
              />
            </div>
          </div>
        </div>
      </fieldset>
    </div>
  );

  return <div className={`space-y-6 ${className}`}>{renderAdvancedForm()}</div>;
}
