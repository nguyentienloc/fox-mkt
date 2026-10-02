"use client";

import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { LuLoaderCircle, LuTriangleAlert } from "react-icons/lu";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  BROWSER_DELETED_EVENT,
  useBrowserDownload,
} from "@/hooks/use-browser-download";

const FOXIA_BROWSER = "cloakbrowser";

export function FoxiaBrowserMissingAlert() {
  const { t } = useTranslation();
  const {
    loadDownloadedVersions,
    loadVersions,
    downloadBrowser,
    isBrowserDownloading,
    downloadProgress,
  } = useBrowserDownload();
  const [isInstalled, setIsInstalled] = useState<boolean | null>(null);

  const checkInstalled = useCallback(async () => {
    try {
      const versions = await loadDownloadedVersions(FOXIA_BROWSER);
      setIsInstalled(versions.length > 0);
    } catch {
      setIsInstalled(false);
    }
  }, [loadDownloadedVersions]);

  useEffect(() => {
    void checkInstalled();
  }, [checkInstalled]);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: (() => void)[] = [];
    const register = (promise: Promise<() => void>) => {
      void promise.then((fn) => {
        if (cancelled) fn();
        else unlisteners.push(fn);
      });
    };

    register(
      listen<{ browser: string; stage: string }>(
        "download-progress",
        (event) => {
          if (
            event.payload.browser === FOXIA_BROWSER &&
            event.payload.stage === "completed"
          ) {
            void checkInstalled();
          }
        },
      ),
    );
    register(
      listen<{ browser: string }>(BROWSER_DELETED_EVENT, (event) => {
        if (event.payload.browser === FOXIA_BROWSER) {
          void checkInstalled();
        }
      }),
    );

    return () => {
      cancelled = true;
      for (const unlisten of unlisteners) unlisten();
    };
  }, [checkInstalled]);

  const handleDownload = async () => {
    try {
      const available = await loadVersions(FOXIA_BROWSER);
      if (!available?.length) return;
      await downloadBrowser(FOXIA_BROWSER, available[0].tag_name);
      await checkInstalled();
    } catch (error) {
      console.error("Failed to download Foxia Browser:", error);
    }
  };

  if (isInstalled !== false) return null;

  const isDownloading = isBrowserDownloading(FOXIA_BROWSER);
  const percentage =
    downloadProgress?.browser === FOXIA_BROWSER &&
    downloadProgress.stage === "downloading"
      ? Math.round(downloadProgress.percentage)
      : null;

  return (
    <Alert variant="destructive" className="mb-2 flex items-center gap-3">
      <LuTriangleAlert className="w-5 h-5 flex-shrink-0" />
      <div className="flex-1 min-w-0">
        <AlertTitle>{t("browserMissing.title")}</AlertTitle>
        <AlertDescription>{t("browserMissing.description")}</AlertDescription>
      </div>
      <Button
        size="sm"
        onClick={() => void handleDownload()}
        disabled={isDownloading}
        className="flex-shrink-0"
      >
        {isDownloading && <LuLoaderCircle className="w-4 h-4 animate-spin" />}
        {isDownloading
          ? `${t("browserMissing.downloading")}${percentage !== null ? ` ${percentage}%` : ""}`
          : t("browserMissing.downloadNow")}
      </Button>
    </Alert>
  );
}
