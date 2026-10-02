"use client";

import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { LuLoaderCircle } from "react-icons/lu";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

const CHECK_INTERVAL_MS = 60 * 60 * 1000;

interface AppUpdateInfo {
  current_version: string;
  new_version: string;
  download_url: string;
  manual_update_required: boolean;
  release_page_url?: string | null;
  force_update: boolean;
  min_supported_version?: string | null;
}

type UpdateStatus = "installing" | "restarting" | "failed" | "manual";

export function ForceUpdateGate() {
  const { t } = useTranslation();
  const [updateInfo, setUpdateInfo] = useState<AppUpdateInfo | null>(null);
  const [status, setStatus] = useState<UpdateStatus>("installing");
  const [error, setError] = useState<string | null>(null);
  const isInstalling = useRef(false);

  const install = useCallback(async (info: AppUpdateInfo) => {
    if (info.manual_update_required) {
      setStatus("manual");
      return;
    }
    if (isInstalling.current) return;
    isInstalling.current = true;
    setStatus("installing");
    setError(null);
    try {
      await invoke("download_and_prepare_app_update", { updateInfo: info });
      setStatus("restarting");
      await invoke("restart_application");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus("failed");
    } finally {
      isInstalling.current = false;
    }
  }, []);

  useEffect(() => {
    const check = async () => {
      try {
        const update = await invoke<AppUpdateInfo | null>(
          "check_for_app_updates",
        );
        if (update?.force_update) {
          setUpdateInfo((prev) => prev ?? update);
        }
      } catch (err) {
        console.error("Failed to check for forced app update:", err);
      }
    };

    void check();
    const intervalId = window.setInterval(() => {
      void check();
    }, CHECK_INTERVAL_MS);
    return () => window.clearInterval(intervalId);
  }, []);

  useEffect(() => {
    if (updateInfo) void install(updateInfo);
  }, [updateInfo, install]);

  if (!updateInfo) return null;

  const releasePageUrl = updateInfo.release_page_url ?? updateInfo.download_url;
  const isBusy = status === "installing" || status === "restarting";

  return (
    <Dialog open>
      <DialogContent
        showCloseButton={false}
        onEscapeKeyDown={(event) => event.preventDefault()}
        onPointerDownOutside={(event) => event.preventDefault()}
        onInteractOutside={(event) => event.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle>{t("forceUpdate.title")}</DialogTitle>
          <DialogDescription>
            {t("forceUpdate.description", {
              current: updateInfo.current_version,
              version: updateInfo.new_version,
            })}
          </DialogDescription>
        </DialogHeader>

        {isBusy && (
          <div className="flex gap-3 items-center text-sm">
            <LuLoaderCircle className="w-4 h-4 animate-spin flex-shrink-0" />
            <span>
              {status === "installing"
                ? t("forceUpdate.installing")
                : t("forceUpdate.restarting")}
            </span>
          </div>
        )}

        {status === "failed" && (
          <div className="space-y-3">
            <p className="text-sm text-destructive">
              {t("forceUpdate.failed")}
              {error ? `: ${error}` : ""}
            </p>
            <div className="flex gap-2 justify-end">
              <Button
                variant="outline"
                onClick={() => void openUrl(releasePageUrl)}
              >
                {t("forceUpdate.openReleasePage")}
              </Button>
              <Button onClick={() => void install(updateInfo)}>
                {t("forceUpdate.retry")}
              </Button>
            </div>
          </div>
        )}

        {status === "manual" && (
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">
              {t("forceUpdate.manual")}
            </p>
            <div className="flex justify-end">
              <Button onClick={() => void openUrl(releasePageUrl)}>
                {t("forceUpdate.openReleasePage")}
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
