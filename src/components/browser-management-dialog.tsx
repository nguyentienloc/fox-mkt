"use client";

import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { LuDownload, LuRefreshCw, LuTrash2 } from "react-icons/lu";
import {
  BROWSER_DELETED_EVENT,
  useBrowserDownload,
} from "@/hooks/use-browser-download";
import { getBrowserDisplayName, getBrowserIcon } from "@/lib/browser-utils";
import { showErrorToast, showSuccessToast } from "@/lib/toast-utils";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { RippleButton } from "./ui/ripple";
import { ScrollArea } from "./ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "./ui/table";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";

const MANAGED_BROWSERS = ["cloakbrowser"] as const;
const GEOIP_KEY = "geoip";

interface BrowserManagementDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export function BrowserManagementDialog({
  isOpen,
  onClose,
}: BrowserManagementDialogProps) {
  const { t } = useTranslation();
  const {
    downloadBrowser,
    loadVersions,
    loadDownloadedVersions,
    isBrowserDownloading,
  } = useBrowserDownload();

  const [installedMap, setInstalledMap] = useState<Record<string, boolean>>({});
  const [geoipAvailable, setGeoipAvailable] = useState(false);
  const [busy, setBusy] = useState<Record<string, boolean>>({});
  const [isLoading, setIsLoading] = useState(false);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const entries = await Promise.all(
        MANAGED_BROWSERS.map(async (browser) => {
          try {
            const versions = await loadDownloadedVersions(browser);
            return [browser, versions.length > 0] as const;
          } catch {
            return [browser, false] as const;
          }
        }),
      );
      setInstalledMap(Object.fromEntries(entries));

      try {
        const available = await invoke<boolean>("is_geoip_database_available");
        setGeoipAvailable(available);
      } catch {
        setGeoipAvailable(false);
      }
    } finally {
      setIsLoading(false);
    }
  }, [loadDownloadedVersions]);

  useEffect(() => {
    if (isOpen) {
      void refresh();
    }
  }, [isOpen, refresh]);

  const handleDownloadBrowser = useCallback(
    async (browser: string) => {
      const name = getBrowserDisplayName(browser);
      setBusy((prev) => ({ ...prev, [browser]: true }));
      try {
        const available = await loadVersions(browser);
        if (!available || available.length === 0) {
          throw new Error(t("browserManagement.noVersionAvailable"));
        }
        await downloadBrowser(browser, available[0].tag_name);
        await refresh();
      } catch (error) {
        showErrorToast(t("browserManagement.downloadFailed", { name }), {
          description: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setBusy((prev) => ({ ...prev, [browser]: false }));
      }
    },
    [downloadBrowser, loadVersions, refresh, t],
  );

  const handleDeleteBrowser = useCallback(
    async (browser: string) => {
      const name = getBrowserDisplayName(browser);
      setBusy((prev) => ({ ...prev, [browser]: true }));
      try {
        await invoke("delete_browser", { browserStr: browser });
        void emit(BROWSER_DELETED_EVENT, { browser });
        showSuccessToast(t("browserManagement.deleteSuccess", { name }));
        await refresh();
      } catch (error) {
        showErrorToast(t("browserManagement.deleteFailed", { name }), {
          description: error instanceof Error ? error.message : String(error),
        });
      } finally {
        setBusy((prev) => ({ ...prev, [browser]: false }));
      }
    },
    [refresh, t],
  );

  const handleReinstallBrowser = useCallback(
    async (browser: string) => {
      setBusy((prev) => ({ ...prev, [browser]: true }));
      try {
        await invoke("delete_browser", { browserStr: browser });
        void emit(BROWSER_DELETED_EVENT, { browser });
      } catch {
        // ignore delete error, proceed to re-download
      } finally {
        setBusy((prev) => ({ ...prev, [browser]: false }));
      }
      await handleDownloadBrowser(browser);
    },
    [handleDownloadBrowser],
  );

  const handleDownloadGeoip = useCallback(async () => {
    setBusy((prev) => ({ ...prev, [GEOIP_KEY]: true }));
    try {
      await invoke("download_geoip_database");
      showSuccessToast(t("browserManagement.geoipDownloadSuccess"));
      await refresh();
    } catch (error) {
      showErrorToast(t("browserManagement.geoipDownloadFailed"), {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setBusy((prev) => ({ ...prev, [GEOIP_KEY]: false }));
    }
  }, [refresh, t]);

  const handleDeleteGeoip = useCallback(async () => {
    setBusy((prev) => ({ ...prev, [GEOIP_KEY]: true }));
    try {
      await invoke("delete_geoip_database");
      showSuccessToast(t("browserManagement.geoipDeleteSuccess"));
      await refresh();
    } catch (error) {
      showErrorToast(t("browserManagement.geoipDeleteFailed"), {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setBusy((prev) => ({ ...prev, [GEOIP_KEY]: false }));
    }
  }, [refresh, t]);

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("browserManagement.title")}</DialogTitle>
          <DialogDescription>
            {t("browserManagement.description")}
          </DialogDescription>
        </DialogHeader>

        <div className="border rounded-md">
          <ScrollArea className="max-h-[360px]">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("browserManagement.component")}</TableHead>
                  <TableHead className="w-28">
                    {t("browserManagement.status")}
                  </TableHead>
                  <TableHead className="w-32 text-right">
                    {t("profiles.table.actions")}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {MANAGED_BROWSERS.map((browser) => {
                  const Icon = getBrowserIcon(browser);
                  const installed = installedMap[browser];
                  const isBusy = busy[browser] || isBrowserDownloading(browser);
                  return (
                    <TableRow key={browser}>
                      <TableCell className="font-medium">
                        <div className="flex gap-2 items-center">
                          <Icon className="w-4 h-4 text-muted-foreground" />
                          {getBrowserDisplayName(browser)}
                        </div>
                      </TableCell>
                      <TableCell>
                        {isLoading ? (
                          <Badge variant="secondary">
                            {t("common.buttons.loading")}
                          </Badge>
                        ) : installed ? (
                          <Badge variant="secondary">
                            {t("browserManagement.installed")}
                          </Badge>
                        ) : (
                          <Badge variant="outline">
                            {t("browserManagement.notInstalled")}
                          </Badge>
                        )}
                      </TableCell>
                      <TableCell>
                        <div className="flex gap-1 justify-end">
                          {installed ? (
                            <>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="sm"
                                    disabled={isBusy}
                                    onClick={() =>
                                      void handleReinstallBrowser(browser)
                                    }
                                  >
                                    <LuRefreshCw
                                      className={`w-4 h-4 ${isBusy ? "animate-spin" : ""}`}
                                    />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>
                                  {t("browserManagement.reinstall")}
                                </TooltipContent>
                              </Tooltip>
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <Button
                                    variant="ghost"
                                    size="sm"
                                    disabled={isBusy}
                                    onClick={() =>
                                      void handleDeleteBrowser(browser)
                                    }
                                  >
                                    <LuTrash2 className="w-4 h-4 text-destructive" />
                                  </Button>
                                </TooltipTrigger>
                                <TooltipContent>
                                  {t("browserManagement.delete")}
                                </TooltipContent>
                              </Tooltip>
                            </>
                          ) : (
                            <Tooltip>
                              <TooltipTrigger asChild>
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  disabled={isBusy}
                                  onClick={() =>
                                    void handleDownloadBrowser(browser)
                                  }
                                >
                                  <LuDownload
                                    className={`w-4 h-4 ${isBusy ? "animate-bounce" : ""}`}
                                  />
                                </Button>
                              </TooltipTrigger>
                              <TooltipContent>
                                {t("browserManagement.download")}
                              </TooltipContent>
                            </Tooltip>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}

                <TableRow>
                  <TableCell className="font-medium">
                    <div className="flex gap-2 items-center">
                      {t("browserManagement.geoip")}
                    </div>
                  </TableCell>
                  <TableCell>
                    {isLoading ? (
                      <Badge variant="secondary">
                        {t("common.buttons.loading")}
                      </Badge>
                    ) : geoipAvailable ? (
                      <Badge variant="secondary">
                        {t("browserManagement.installed")}
                      </Badge>
                    ) : (
                      <Badge variant="outline">
                        {t("browserManagement.notInstalled")}
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-1 justify-end">
                      {geoipAvailable ? (
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="sm"
                              disabled={busy[GEOIP_KEY]}
                              onClick={() => void handleDeleteGeoip()}
                            >
                              <LuTrash2 className="w-4 h-4 text-destructive" />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>
                            {t("browserManagement.delete")}
                          </TooltipContent>
                        </Tooltip>
                      ) : (
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <Button
                              variant="ghost"
                              size="sm"
                              disabled={busy[GEOIP_KEY]}
                              onClick={() => void handleDownloadGeoip()}
                            >
                              <LuDownload
                                className={`w-4 h-4 ${busy[GEOIP_KEY] ? "animate-bounce" : ""}`}
                              />
                            </Button>
                          </TooltipTrigger>
                          <TooltipContent>
                            {t("browserManagement.download")}
                          </TooltipContent>
                        </Tooltip>
                      )}
                    </div>
                  </TableCell>
                </TableRow>
              </TableBody>
            </Table>
          </ScrollArea>
        </div>

        <DialogFooter>
          <RippleButton variant="outline" onClick={onClose}>
            {t("common.buttons.close")}
          </RippleButton>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
