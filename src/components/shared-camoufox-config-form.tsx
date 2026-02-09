"use client";

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { HiSparkles } from "react-icons/hi2";
import MultipleSelector, { type Option } from "@/components/multiple-selector";
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
  CamoufoxConfig,
  CamoufoxFingerprintConfig,
  CamoufoxOS,
} from "@/types";

interface SharedCamoufoxConfigFormProps {
  config: CamoufoxConfig;
  onConfigChange: (key: keyof CamoufoxConfig, value: unknown) => void;
  className?: string;
  isCreating?: boolean; // Flag to indicate if this is for creating a new profile
  readOnly?: boolean; // Flag to indicate if the form should be read-only
  browserType?: "camoufox" | "wayfern"; // Browser type to customize form options
}

// Determine if fingerprint editing should be disabled
const isFingerprintEditingDisabled = (_config: CamoufoxConfig): boolean => {
  return false;
};

// Detect the current operating system
const getCurrentOS = (): CamoufoxOS => {
  if (typeof navigator === "undefined") return "linux";
  const platform = navigator.platform.toLowerCase();
  if (platform.includes("win")) return "windows";
  if (platform.includes("mac")) return "macos";
  return "linux";
};

// OS display labels
const osLabels: Record<CamoufoxOS, string> = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
};

// Component for editing nested objects like webGl:parameters
interface ObjectEditorProps {
  value: Record<string, unknown> | undefined;
  onChange: (value: Record<string, unknown> | undefined) => void;
  title: string;
  readOnly?: boolean;
}

function ObjectEditor({
  value,
  onChange,
  title,
  readOnly = false,
}: ObjectEditorProps) {
  const [jsonString, setJsonString] = useState("");

  useEffect(() => {
    setJsonString(JSON.stringify(value || {}, null, 2));
  }, [value]);

  const handleChange = (newValue: string) => {
    if (readOnly) return;
    setJsonString(newValue);
    try {
      if (newValue.trim() === "" || newValue.trim() === "{}") {
        onChange(undefined); // Treat empty objects as undefined
        return;
      }
      const parsed = JSON.parse(newValue);
      if (
        typeof parsed === "object" &&
        parsed !== null &&
        Object.keys(parsed).length === 0
      ) {
        onChange(undefined);
        return;
      }
      onChange(parsed as Record<string, unknown>);
    } catch (err) {
      console.warn("Invalid JSON:", err);
    }
  };

  return (
    <div className="space-y-2">
      <Label>{title}</Label>
      <Textarea
        value={jsonString}
        onChange={(e) => handleChange(e.target.value)}
        placeholder={`Enter ${title} as JSON`}
        className="font-mono text-sm"
        rows={6}
        disabled={readOnly}
      />
    </div>
  );
}

export function SharedCamoufoxConfigForm({
  config,
  onConfigChange,
  className = "",
  isCreating = false,
  readOnly = false,
  browserType = "camoufox",
}: SharedCamoufoxConfigFormProps) {
  const { t } = useTranslation();
  const [fingerprintConfig, setFingerprintConfig] =
    useState<CamoufoxFingerprintConfig>({});
  const [currentOS] = useState<CamoufoxOS>(getCurrentOS);

  // Get selected OS (defaults to current OS)
  const selectedOS = config.os || currentOS;
  const isOSDifferent = selectedOS !== currentOS;

  // Set screen resolution to user's screen size when creating a new profile
  useEffect(() => {
    if (isCreating && typeof window !== "undefined") {
      const screenWidth = window.screen.width;
      const screenHeight = window.screen.height;

      // Only set if not already configured
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

  // Parse fingerprint config when component mounts or config changes
  useEffect(() => {
    if (config.fingerprint) {
      try {
        const parsed = JSON.parse(
          config.fingerprint,
        ) as CamoufoxFingerprintConfig;
        setFingerprintConfig(parsed);
      } catch (error) {
        console.error("Failed to parse fingerprint config:", error);
        setFingerprintConfig({});
      }
    } else {
      // Initialize with empty config if no fingerprint is set
      setFingerprintConfig({});
    }
  }, [config.fingerprint]);

  // Update fingerprint config and serialize it
  const updateFingerprintConfig = (
    key: keyof CamoufoxFingerprintConfig,
    value: unknown,
  ) => {
    const newConfig = { ...fingerprintConfig };

    // Remove undefined values to keep the config clean
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

    // Validate that the config can be serialized to JSON
    try {
      const jsonString = JSON.stringify(newConfig);
      onConfigChange("fingerprint", jsonString);
    } catch (error) {
      console.error("Failed to serialize fingerprint config:", error);
      // Don't update if serialization fails
    }
  };

  // Determine if automatic location configuration is enabled
  const _isAutoLocationEnabled = config.geoip !== false;

  // Handle automatic location configuration toggle
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
      // Linux
      userAgent = `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${chromeVersion}.0.0.${chromePatch} Safari/537.36`;
      platform = "Linux x86_64";
    }

    // Update both fields at once to avoid state update race condition
    const newConfig = {
      ...fingerprintConfig,
      "navigator.userAgent": userAgent,
      "navigator.platform": platform,
    };
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
          onValueChange={(value: CamoufoxOS) => onConfigChange("os", value)}
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
            <SelectItem value="windows">{osLabels.windows}</SelectItem>
            <SelectItem value="macos">{osLabels.macos}</SelectItem>
            <SelectItem value="linux">{osLabels.linux}</SelectItem>
          </SelectContent>
        </Select>
        {isOSDifferent && (
          <Alert className="border-yellow-500/50 bg-yellow-500/10">
            <AlertDescription className="text-yellow-600 dark:text-yellow-400">
              {t("createProfile.fingerprint.warnings.osDifferent", {
                currentOS: osLabels[currentOS],
              })}
            </AlertDescription>
          </Alert>
        )}
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
        {/* Blocking Options - Only available for Camoufox */}
        {browserType === "camoufox" && (
          <div className="space-y-3">
            <Label>{t("createProfile.fingerprint.blocking.title")}</Label>
            <div className="space-y-2">
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="block-images"
                  checked={config.block_images || false}
                  onCheckedChange={(checked) =>
                    onConfigChange("block_images", checked)
                  }
                />
                <Label htmlFor="block-images">
                  {t("createProfile.fingerprint.blocking.blockImages")}
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="block-webrtc"
                  checked={config.block_webrtc || false}
                  onCheckedChange={(checked) =>
                    onConfigChange("block_webrtc", checked)
                  }
                />
                <Label htmlFor="block-webrtc">
                  {t("createProfile.fingerprint.blocking.blockWebRTC")}
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="block-webgl"
                  checked={config.block_webgl || false}
                  onCheckedChange={(checked) =>
                    onConfigChange("block_webgl", checked)
                  }
                />
                <Label htmlFor="block-webgl">
                  {t("createProfile.fingerprint.blocking.blockWebGL")}
                </Label>
              </div>
            </div>
          </div>
        )}

        {/* Navigator Properties */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <Label>{t("createProfile.fingerprint.navigator.title")}</Label>
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
            <div className="space-y-2">
              <Label htmlFor="user-agent">
                {t("createProfile.fingerprint.userAgentPlatform.userAgent")}
              </Label>
              <Input
                id="user-agent"
                value={fingerprintConfig["navigator.userAgent"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "navigator.userAgent",
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
                value={fingerprintConfig["navigator.platform"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "navigator.platform",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.userAgentPlatform.platformPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="app-version">
                {t("createProfile.fingerprint.navigator.appVersion")}
              </Label>
              <Input
                id="app-version"
                value={fingerprintConfig["navigator.appVersion"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "navigator.appVersion",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.navigator.appVersionPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="oscpu">
                {t("createProfile.fingerprint.navigator.oscpu")}
              </Label>
              <Input
                id="oscpu"
                value={fingerprintConfig["navigator.oscpu"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "navigator.oscpu",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.navigator.oscpuPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="hardware-concurrency">
                {t("createProfile.fingerprint.hardware.concurrency")}
              </Label>
              <Input
                id="hardware-concurrency"
                type="number"
                value={fingerprintConfig["navigator.hardwareConcurrency"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "navigator.hardwareConcurrency",
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
                value={fingerprintConfig["navigator.maxTouchPoints"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "navigator.maxTouchPoints",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.hardware.maxTouchPointsPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="do-not-track">
                {t("createProfile.fingerprint.language.doNotTrack")}
              </Label>
              <Select
                value={fingerprintConfig["navigator.doNotTrack"] || ""}
                onValueChange={(value) =>
                  updateFingerprintConfig(
                    "navigator.doNotTrack",
                    value || undefined,
                  )
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
            <div className="space-y-2">
              <Label htmlFor="language">
                {t("createProfile.fingerprint.language.primary")}
              </Label>
              <Input
                id="language"
                value={fingerprintConfig["navigator.language"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "navigator.language",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.language.primaryPlaceholder",
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
                value={fingerprintConfig["screen.width"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screen.width",
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
                value={fingerprintConfig["screen.height"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screen.height",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.heightPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="avail-width">
                {t("createProfile.fingerprint.screen.availWidth")}
              </Label>
              <Input
                id="avail-width"
                type="number"
                value={fingerprintConfig["screen.availWidth"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screen.availWidth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.availWidthPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="avail-height">
                {t("createProfile.fingerprint.screen.availHeight")}
              </Label>
              <Input
                id="avail-height"
                type="number"
                value={fingerprintConfig["screen.availHeight"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screen.availHeight",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.availHeightPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="color-depth">
                {t("createProfile.fingerprint.screen.colorDepth")}
              </Label>
              <Input
                id="color-depth"
                type="number"
                value={fingerprintConfig["screen.colorDepth"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screen.colorDepth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.colorDepthPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="pixel-depth">
                {t("createProfile.fingerprint.screen.pixelRatio")}
              </Label>
              <Input
                id="pixel-depth"
                type="number"
                value={fingerprintConfig["screen.pixelDepth"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "screen.pixelDepth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.screen.pixelRatioPlaceholder",
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
              <Label htmlFor="outer-width">
                {t("createProfile.fingerprint.window.outerWidth")}
              </Label>
              <Input
                id="outer-width"
                type="number"
                value={fingerprintConfig["window.outerWidth"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "window.outerWidth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.outerWidthPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="outer-height">
                {t("createProfile.fingerprint.window.outerHeight")}
              </Label>
              <Input
                id="outer-height"
                type="number"
                value={fingerprintConfig["window.outerHeight"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "window.outerHeight",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.outerHeightPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="inner-width">
                {t("createProfile.fingerprint.window.innerWidth")}
              </Label>
              <Input
                id="inner-width"
                type="number"
                value={fingerprintConfig["window.innerWidth"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "window.innerWidth",
                    e.target.value ? parseInt(e.target.value, 10) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.window.innerWidthPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="inner-height">
                {t("createProfile.fingerprint.window.innerHeight")}
              </Label>
              <Input
                id="inner-height"
                type="number"
                value={fingerprintConfig["window.innerHeight"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "window.innerHeight",
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
                value={fingerprintConfig["window.screenX"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "window.screenX",
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
                value={fingerprintConfig["window.screenY"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "window.screenY",
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

        {/* Geolocation */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.geolocation.title")}</Label>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="latitude">
                {t("createProfile.fingerprint.timezone.latitude")}
              </Label>
              <Input
                id="latitude"
                type="number"
                step="any"
                value={fingerprintConfig["geolocation:latitude"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "geolocation:latitude",
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
                value={fingerprintConfig["geolocation:longitude"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "geolocation:longitude",
                    e.target.value ? parseFloat(e.target.value) : undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.timezone.longitudePlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="timezone">
                {t("createProfile.fingerprint.timezone.timezone")}
              </Label>
              <Input
                id="timezone"
                type="text"
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
          </div>
        </div>

        {/* Locale */}
        <div className="space-y-3">
          <Label>{t("createProfile.fingerprint.locale.title")}</Label>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="locale-language">
                {t("createProfile.fingerprint.locale.language")}
              </Label>
              <Input
                id="locale-language"
                value={fingerprintConfig["locale:language"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "locale:language",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.locale.languagePlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="locale-region">
                {t("createProfile.fingerprint.locale.region")}
              </Label>
              <Input
                id="locale-region"
                value={fingerprintConfig["locale:region"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "locale:region",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.locale.regionPlaceholder",
                )}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="locale-script">
                {t("createProfile.fingerprint.locale.script")}
              </Label>
              <Input
                id="locale-script"
                value={fingerprintConfig["locale:script"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "locale:script",
                    e.target.value || undefined,
                  )
                }
                placeholder={t(
                  "createProfile.fingerprint.locale.scriptPlaceholder",
                )}
              />
            </div>
          </div>
        </div>

        {/* WebGL Properties */}
        <div className="space-y-3">
          <Label>WebGL Properties</Label>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="webgl-vendor">WebGL Vendor</Label>
              <Input
                id="webgl-vendor"
                value={fingerprintConfig["webGl:vendor"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "webGl:vendor",
                    e.target.value || undefined,
                  )
                }
                placeholder="e.g., Mesa"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="webgl-renderer">WebGL Renderer</Label>
              <Input
                id="webgl-renderer"
                value={fingerprintConfig["webGl:renderer"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "webGl:renderer",
                    e.target.value || undefined,
                  )
                }
                placeholder="e.g., llvmpipe, or similar"
              />
            </div>
          </div>
        </div>

        {/* WebGL Parameters */}
        <div className="space-y-3">
          <ObjectEditor
            value={
              (fingerprintConfig["webGl:parameters"] as Record<
                string,
                unknown
              >) || {}
            }
            onChange={(value) =>
              updateFingerprintConfig("webGl:parameters", value)
            }
            title="WebGL Parameters"
            readOnly={readOnly}
          />
        </div>

        {/* WebGL2 Parameters */}
        <div className="space-y-3">
          <ObjectEditor
            value={
              (fingerprintConfig["webGl2:parameters"] as Record<
                string,
                unknown
              >) || {}
            }
            onChange={(value) =>
              updateFingerprintConfig("webGl2:parameters", value)
            }
            title="WebGL2 Parameters"
            readOnly={readOnly}
          />
        </div>

        {/* WebGL Shader Precision Formats */}
        <div className="space-y-3">
          <ObjectEditor
            value={
              (fingerprintConfig["webGl:shaderPrecisionFormats"] as Record<
                string,
                unknown
              >) || {}
            }
            onChange={(value) =>
              updateFingerprintConfig("webGl:shaderPrecisionFormats", value)
            }
            title="WebGL Shader Precision Formats"
            readOnly={readOnly}
          />
        </div>

        {/* WebGL2 Shader Precision Formats */}
        <div className="space-y-3">
          <ObjectEditor
            value={
              (fingerprintConfig["webGl2:shaderPrecisionFormats"] as Record<
                string,
                unknown
              >) || {}
            }
            onChange={(value) =>
              updateFingerprintConfig("webGl2:shaderPrecisionFormats", value)
            }
            title="WebGL2 Shader Precision Formats"
            readOnly={readOnly}
          />
        </div>

        {/* Fonts */}
        <div className="space-y-3">
          <Label>Fonts</Label>
          <MultipleSelector
            value={(() => {
              // Handle fonts being either an array or a JSON string (Wayfern format)
              let fontsArray: string[] = [];
              if (fingerprintConfig.fonts) {
                if (Array.isArray(fingerprintConfig.fonts)) {
                  fontsArray = fingerprintConfig.fonts;
                } else if (typeof fingerprintConfig.fonts === "string") {
                  try {
                    const parsed = JSON.parse(fingerprintConfig.fonts);
                    if (Array.isArray(parsed)) {
                      fontsArray = parsed;
                    }
                  } catch {
                    // Invalid JSON, ignore
                  }
                }
              }
              return fontsArray.map((font) => ({
                label: font,
                value: font,
              }));
            })()}
            onChange={(selected: Option[]) =>
              updateFingerprintConfig(
                "fonts",
                selected.map((s: Option) => s.value),
              )
            }
            placeholder="Add fonts..."
            creatable
          />
        </div>

        {/* Battery */}
        <div className="space-y-3">
          <Label>Battery</Label>
          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="battery-charging"
                  checked={fingerprintConfig["battery:charging"] || false}
                  onCheckedChange={(checked) =>
                    updateFingerprintConfig("battery:charging", checked)
                  }
                />
                <Label htmlFor="battery-charging">Charging</Label>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="charging-time">Charging Time</Label>
              <Input
                id="charging-time"
                type="number"
                step="any"
                value={fingerprintConfig["battery:chargingTime"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "battery:chargingTime",
                    e.target.value ? parseFloat(e.target.value) : undefined,
                  )
                }
                placeholder="e.g., 0"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="discharging-time">Discharging Time</Label>
              <Input
                id="discharging-time"
                type="number"
                step="any"
                value={fingerprintConfig["battery:dischargingTime"] || ""}
                onChange={(e) =>
                  updateFingerprintConfig(
                    "battery:dischargingTime",
                    e.target.value ? parseFloat(e.target.value) : undefined,
                  )
                }
                placeholder="e.g., 0"
              />
            </div>
          </div>
        </div>

        {/* Browser Behavior */}
        {/* <div className="space-y-3">
        <Label>Browser Behavior</Label>
        <div className="flex items-center space-x-2">
          <Checkbox
            id="allow-addon-new-tab"
            checked={fingerprintConfig.allowAddonNewTab}
            onCheckedChange={(checked) =>
              updateFingerprintConfig("allowAddonNewTab", checked)
            }
          />
          <Label htmlFor="allow-addon-new-tab">
            Allow browser addons to open new tabs automatically
          </Label>
        </div>
      </div> */}
      </fieldset>
    </div>
  );

  return <div className={`space-y-6 ${className}`}>{renderAdvancedForm()}</div>;
}
