"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { LoadingButton } from "@/components/loading-button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { OdooProfile } from "@/odoo/types";
import { RippleButton } from "./ui/ripple";

interface OdooImportDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export function OdooImportDialog({ isOpen, onClose }: OdooImportDialogProps) {
  const { t } = useTranslation();
  const [profiles, setProfiles] = useState<OdooProfile[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [isLoading, setIsLoading] = useState(false);
  const [isImporting, setIsImporting] = useState(false);

  const loadOdooProfiles = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await invoke<{ items: OdooProfile[] }>(
        "list_odoo_profiles",
        {
          offset: 0,
          limit: 1000,
        },
      );
      setProfiles(result.items);
    } catch (error) {
      console.error("Failed to list Odoo profiles:", error);
      toast.error(
        t("odooImport.loadingError") || "Lỗi khi tải profile từ Odoo",
      );
    } finally {
      setIsLoading(false);
    }
  }, [t]);

  useEffect(() => {
    if (isOpen) {
      void loadOdooProfiles();
    }
  }, [isOpen, loadOdooProfiles]);

  const handleToggleSelect = (id: string) => {
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelectedIds(next);
  };

  const handleImport = async () => {
    if (selectedIds.size === 0) return;

    setIsImporting(true);
    try {
      const selectedProfiles = profiles.filter((p) =>
        selectedIds.has(String(p.id)),
      );

      // Map OdooProfile to ZsMktProfile format for the existing import command
      const mappedProfiles = selectedProfiles.map((p) => {
        const getString = (val: any) => (typeof val === "string" ? val : "");

        return {
          id: String(p.id),
          name: p.name,
          fingerprint: {
            userAgent: getString(p.user_agent),
            timezone: getString(p.timezone),
            language: getString(p.language),
            platform: typeof p.platform === "string" ? p.platform : undefined,
          },
          status: "synced",
          version: "v135.0.1-beta.24",
          localPath:
            getString(p.local_path) ||
            `/profiles/${p.name.toLowerCase().replace(/\s+/g, "_")}`,
          proxy: p.proxy_ids?.[0]
            ? {
                protocol: p.proxy_ids[0].giaothuc,
                host: p.proxy_ids[0].ip,
                port: p.proxy_ids[0].port,
                username: p.proxy_ids[0].tendangnhap,
                password: p.proxy_ids[0].matkhau,
              }
            : undefined,
          createdAt: (p as any).createdAt || (p as any).create_date,
          username: getString((p as any).username),
          password: getString((p as any).password),
          browser: getString((p as any).browser) || undefined,
        };
      });

      const count = await invoke<number>("import_zsmkt_profiles_batch", {
        zsProfiles: mappedProfiles,
      });

      toast.success(
        t("profiles.toasts.success.importSuccess", { count }) ||
          `Đã nhập ${count} profile thành công`,
      );
      onClose();
    } catch (error) {
      console.error("Import failed:", error);
      toast.error(
        t("profiles.toasts.error.importFailed") || "Nhập profile thất bại",
      );
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-3xl h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{t("odooImport.title")}</DialogTitle>
        </DialogHeader>

        <div className="flex-1 min-h-0 py-4">
          {isLoading ? (
            <div className="flex items-center justify-center h-full">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
            </div>
          ) : profiles.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              {t("odooImport.empty")}
            </div>
          ) : (
            <ScrollArea className="h-full pr-4">
              <div className="space-y-2">
                {profiles.map((profile) => (
                  <div
                    key={String(profile.id)}
                    className="flex items-center gap-4 p-3 rounded-lg border hover:bg-accent/50 transition-colors"
                  >
                    <Checkbox
                      checked={selectedIds.has(String(profile.id))}
                      onCheckedChange={() =>
                        handleToggleSelect(String(profile.id))
                      }
                    />
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">{profile.name}</div>
                      <div className="text-xs text-muted-foreground truncate">
                        {typeof profile.user_agent === "string"
                          ? profile.user_agent
                          : "Không có User Agent"}
                      </div>
                    </div>
                    {profile.profile_url && (
                      <div className="text-[10px] bg-primary/10 text-primary px-1.5 py-0.5 rounded">
                        {t("odooImport.syncedBadge")}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </ScrollArea>
          )}
        </div>

        <DialogFooter className="gap-2">
          <div className="mr-auto text-sm text-muted-foreground self-center">
            {t("odooImport.selected", { count: selectedIds.size })}
          </div>
          <RippleButton variant="outline" onClick={onClose}>
            {t("common.buttons.cancel")}
          </RippleButton>
          <LoadingButton
            isLoading={isImporting}
            onClick={handleImport}
            disabled={selectedIds.size === 0}
          >
            {t("odooImport.importSelected")}
          </LoadingButton>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
