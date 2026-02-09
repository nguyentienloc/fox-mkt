"use client";

import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "next-themes";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { usePermissions } from "@/hooks/use-permissions";
import {
  applyThemeColors,
  clearThemeColors,
  getThemeByColors,
} from "@/lib/themes";
import { LoadingButton } from "./loading-button";
import { Button } from "./ui/button";

interface SettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onIntegrationsOpen: () => void;
}

interface AppSettings {
  theme: string;
  api_enabled: boolean;
  custom_theme?: Record<string, string>;
}

export function SettingsDialog({ isOpen, onClose }: SettingsDialogProps) {
  const { t } = useTranslation();
  const { setTheme } = useTheme();

  const [settings, setSettings] = useState<AppSettings>({
    theme: "light",
    api_enabled: false,
  });

  const [originalSettings, setOriginalSettings] = useState<AppSettings>({
    theme: "light",
    api_enabled: false,
  });

  const [customThemeState, setCustomThemeState] = useState<{
    selectedThemeId: string | null;
    colors: Record<string, string>;
  }>({
    selectedThemeId: null,
    colors: {},
  });

  const [isClearingCache, setIsClearingCache] = useState(false);

  const { isMicrophoneAccessGranted, isCameraAccessGranted } = usePermissions();

  useEffect(() => {
    if (isOpen) {
      const loadSettings = async () => {
        try {
          const savedSettings = await invoke<AppSettings>("get_app_settings");
          setSettings(savedSettings);
          setOriginalSettings(savedSettings);

          if (savedSettings.theme === "custom" && savedSettings.custom_theme) {
            const matchingTheme = getThemeByColors(savedSettings.custom_theme);
            setCustomThemeState({
              selectedThemeId: matchingTheme?.id || null,
              colors: savedSettings.custom_theme,
            });
          }
        } catch (error) {
          console.error("Failed to load settings:", error);
        }
      };
      void loadSettings();
    }
  }, [isOpen]);

  const handleSave = useCallback(async () => {
    try {
      const settingsToSave = {
        ...settings,
        custom_theme:
          settings.theme === "custom" ? customThemeState.colors : undefined,
      };

      await invoke("save_app_settings", { settings: settingsToSave });
      setTheme(settings.theme);

      if (settings.theme === "custom") {
        applyThemeColors(customThemeState.colors);
      } else {
        clearThemeColors();
      }

      setOriginalSettings(settingsToSave);
      onClose();
    } catch (error) {
      console.error("Failed to save settings:", error);
    }
  }, [onClose, setTheme, settings, customThemeState]);

  const updateSetting = useCallback((key: keyof AppSettings, value: any) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
  }, []);

  const handleClearCache = async () => {
    setIsClearingCache(true);
    try {
      await invoke("clear_browser_version_cache");
      toast.success("Đã xóa bộ nhớ đệm thành công");
    } catch (error) {
      console.error("Failed to clear cache:", error);
    } finally {
      setIsClearingCache(false);
    }
  };

  const hasChanges =
    JSON.stringify(settings) !== JSON.stringify(originalSettings) ||
    (settings.theme === "custom" &&
      JSON.stringify(customThemeState.colors) !==
        JSON.stringify(originalSettings.custom_theme));

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-md max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{t("settings.title")}</DialogTitle>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-6 py-4">
          <div className="space-y-4">
            <Label className="text-base font-medium">Giao diện</Label>
            <div className="grid gap-2">
              <Label
                htmlFor="theme-select"
                className="text-sm text-muted-foreground"
              >
                Chủ đề
              </Label>
              <Select
                value={settings.theme}
                onValueChange={(v) => updateSetting("theme", v)}
              >
                <SelectTrigger id="theme-select">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">Sáng</SelectItem>
                  <SelectItem value="dark">Tối</SelectItem>
                  <SelectItem value="system">Hệ thống</SelectItem>
                  <SelectItem value="custom">Tùy chỉnh</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-4">
            <Label className="text-base font-medium">Quyền hệ thống</Label>
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span>Microphone</span>
                <span
                  className={
                    isMicrophoneAccessGranted
                      ? "text-green-500"
                      : "text-red-500"
                  }
                >
                  {isMicrophoneAccessGranted ? "Đã cấp" : "Chưa cấp"}
                </span>
              </div>
              <div className="flex items-center justify-between text-sm">
                <span>Máy ảnh</span>
                <span
                  className={
                    isCameraAccessGranted ? "text-green-500" : "text-red-500"
                  }
                >
                  {isCameraAccessGranted ? "Đã cấp" : "Chưa cấp"}
                </span>
              </div>
            </div>
          </div>

          <div className="space-y-4">
            <Label className="text-base font-medium">Nâng cao</Label>
            <LoadingButton
              isLoading={isClearingCache}
              onClick={handleClearCache}
              variant="outline"
              className="w-full"
            >
              Xóa bộ nhớ đệm phiên bản
            </LoadingButton>
            <p className="text-xs text-muted-foreground">
              Việc này sẽ ép hệ thống cập nhật lại thông tin mới nhất cho tất cả
              trình duyệt.
            </p>
          </div>
        </div>

        <div className="flex justify-end gap-2 pt-4 border-t">
          <Button variant="ghost" onClick={onClose}>
            Hủy
          </Button>
          <Button onClick={handleSave} disabled={!hasChanges}>
            Lưu thay đổi
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
