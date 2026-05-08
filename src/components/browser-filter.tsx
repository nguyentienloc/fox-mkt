"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { FaChrome, FaFirefox } from "react-icons/fa";
import { LuCloud } from "react-icons/lu";
import { LoadingButton } from "@/components/loading-button";
import { Badge } from "@/components/ui/badge";
import { useBrowserDownload } from "@/hooks/use-browser-download";
import type { BrowserReleaseTypes } from "@/types";

export type BrowserFilterType =
  | "all"
  | "camoufox"
  | "wayfern"
  | "cloakbrowser"
  | "cloud";

interface BrowserFilterProps {
  selectedFilter: BrowserFilterType;
  onFilterSelect: (filter: BrowserFilterType) => void;
  counts: Record<BrowserFilterType, number>;
  onCreateProfile?: () => void;
}

export function BrowserFilter({
  selectedFilter,
  onFilterSelect,
  counts,
  onCreateProfile,
}: BrowserFilterProps) {
  const { t } = useTranslation();
  const [releaseTypes, setReleaseTypes] = useState<BrowserReleaseTypes>();

  const {
    isBrowserDownloading,
    downloadBrowser,
    loadDownloadedVersions,
    isVersionDownloaded,
  } = useBrowserDownload();

  const filters: { id: BrowserFilterType; label: string; icon: any }[] = [
    { id: "all", label: "Tất cả", icon: null },
    { id: "camoufox", label: "Firefox", icon: FaFirefox },
    { id: "wayfern", label: "Chromium", icon: FaChrome },
    { id: "cloakbrowser", label: "Foxia Browser", icon: FaChrome },
    { id: "cloud", label: "Đám mây", icon: LuCloud },
  ];

  const handleCreateProfile = () => {
    if (onCreateProfile) {
      onCreateProfile();
    }
  };

  const loadReleaseTypes = useCallback(async () => {
    try {
      const rawReleaseTypes = await invoke<BrowserReleaseTypes>(
        "get_browser_release_types",
        { browserStr: "orbita" },
      );

      await loadDownloadedVersions("orbita");

      const filtered: BrowserReleaseTypes = {};
      if (rawReleaseTypes.stable) filtered.stable = rawReleaseTypes.stable;
      setReleaseTypes(filtered);
    } catch (error) {
      console.error("Failed to load Orbita release types:", error);
    }
  }, [loadDownloadedVersions]);

  useEffect(() => {
    void loadReleaseTypes();
  }, [loadReleaseTypes]);

  const getBestAvailableVersion = useCallback(() => {
    if (!releaseTypes?.stable) return null;
    return { version: releaseTypes.stable, releaseType: "stable" as const };
  }, [releaseTypes]);

  const handleDownload = async () => {
    const bestVersion = getBestAvailableVersion();
    if (!bestVersion) {
      console.error("No Orbita version available for download");
      return;
    }

    try {
      await downloadBrowser("orbita", bestVersion.version);
    } catch (error) {
      console.error("Failed to download Orbita:", error);
    }
  };

  const isBrowserVersionAvailable = useMemo(() => {
    const bestVersion = getBestAvailableVersion();
    return bestVersion && isVersionDownloaded(bestVersion.version);
  }, [isVersionDownloaded, getBestAvailableVersion]);

  const showDownloadButton =
    Boolean(releaseTypes?.stable) &&
    (!isBrowserVersionAvailable || isBrowserDownloading("orbita"));

  return (
    <div className="flex items-center justify-between gap-2 mb-2">
      <div className="flex gap-2 overflow-x-auto pb-1 [scrollbar-width:none] [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden">
        {filters.map((filter) => {
          const Icon = filter.icon;
          const isSelected = selectedFilter === filter.id;

          return (
            <Badge
              key={filter.id}
              variant={isSelected ? "default" : "secondary"}
              className={`flex gap-2 items-center px-3 py-1 cursor-pointer transition-all hover:scale-105 active:scale-95 flex-shrink-0 ${
                isSelected
                  ? "bg-primary text-primary-foreground"
                  : "hover:bg-accent"
              }`}
              onClick={() => onFilterSelect(filter.id)}
            >
              {Icon && <Icon className="w-3.5 h-3.5" />}
              <span>{filter.label}</span>
              <span
                className={`text-[10px] px-1.5 py-0.5 rounded-sm ${
                  isSelected
                    ? "bg-white/20 text-white"
                    : "bg-muted text-muted-foreground"
                }`}
              >
                {counts[filter.id]}
              </span>
            </Badge>
          );
        })}
      </div>

      <div className="flex gap-2 items-center flex-shrink-0 ml-auto">
        {showDownloadButton && (
          <LoadingButton
            onClick={handleDownload}
            isLoading={isBrowserDownloading("orbita")}
            size="sm"
            variant="outline"
            disabled={isBrowserDownloading("orbita")}
            className="h-8"
          >
            {isBrowserDownloading("orbita") ? "Đang tải..." : "Tải Orbita"}
          </LoadingButton>
        )}

        {onCreateProfile && (
          <button
            type="button"
            onClick={handleCreateProfile}
            className="flex items-center justify-center w-8 h-8 rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors flex-shrink-0"
            title={t("header.createProfile")}
          >
            <span className="text-xl leading-none">+</span>
          </button>
        )}
      </div>
    </div>
  );
}
