"use client";
import { invoke } from "@tauri-apps/api/core";
import { Check, Copy, Edit2, Eye, EyeOff, X } from "lucide-react";
import { useEffect, useState } from "react";
import { HiSparkles } from "react-icons/hi2";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { getBrowserIcon } from "@/lib/browser-utils";
import { useAuth } from "@/providers/auth-provider";
import type { BrowserProfile } from "@/types";

interface ProfileDetailsDialogProps {
  isOpen: boolean;
  onClose: () => void;
  profile: BrowserProfile | null;
}

export function ProfileDetailsDialog({
  isOpen,
  onClose,
  profile,
}: ProfileDetailsDialogProps) {
  const { isManager } = useAuth();
  const [showPassword, setShowPassword] = useState(false);
  const [isEditing, setIsEditing] = useState(false);

  // Edit states
  const [editName, setEditName] = useState("");
  const [editUsername, setEditUsername] = useState("");
  const [editPassword, setEditPassword] = useState("");
  const [editUserAgent, setEditUserAgent] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (profile && isOpen) {
      setEditName(profile.name || "");
      setEditUsername(profile.username || "");
      setEditPassword(profile.password || "");
      setEditUserAgent(profile.user_agent || "");
      setIsEditing(false);
    }
  }, [profile, isOpen]);

  if (!profile) return null;

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    toast.success("Đã sao chép vào bộ nhớ tạm");
  };

  const formatDate = (timestamp?: number) => {
    if (!timestamp) return "N/A";
    return new Date(timestamp * 1000).toLocaleString();
  };

  const handleSave = async () => {
    try {
      setIsSaving(true);
      await invoke("update_profile_details", {
        profileId: profile.id,
        name: editName,
        username: editUsername || null,
        password: editPassword || null,
        userAgent: editUserAgent || null,
      });

      // Sync to Odoo if odoo_id exists
      if (profile.odoo_id) {
        try {
          await invoke("update_odoo_profile", {
            profile: {
              id: profile.odoo_id,
              name: editName,
              username: editUsername || null,
              password: editPassword || null,
              userAgent: editUserAgent || null,
            },
          });
          toast.success("Đã đồng bộ thông tin lên Odoo");
        } catch (odooError) {
          console.error("Failed to sync with Odoo:", odooError);
          toast.warning(
            `Lưu local thành công nhưng đồng bộ Odoo thất bại: ${odooError}`,
          );
        }
      }

      toast.success("Cập nhật thông tin thành công");
      setIsEditing(false);
      // The profiles-changed event will handle updating the list
    } catch (error) {
      console.error("Failed to update profile details:", error);
      toast.error(`Cập nhật thất bại: ${error}`);
    } finally {
      setIsSaving(false);
    }
  };

  const generateRandomUserAgent = () => {
    const os =
      profile.camoufox_config?.os || profile.wayfern_config?.os || "windows";

    // Random Chrome versions between 120-131
    const chromeVersion = Math.floor(Math.random() * 12) + 120;
    const chromePatch = Math.floor(Math.random() * 1000);

    const windowsVersions = ["10.0", "11.0"];
    const macVersions = ["10_15_7", "11_0_0", "12_0_0", "13_0_0", "14_0_0"];

    let userAgent = "";

    if (os === "windows") {
      const winVersion =
        windowsVersions[Math.floor(Math.random() * windowsVersions.length)];
      userAgent = `Mozilla/5.0 (Windows NT ${winVersion}; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${chromeVersion}.0.0.${chromePatch} Safari/537.36`;
    } else if (os === "macos") {
      const macVersion =
        macVersions[Math.floor(Math.random() * macVersions.length)];
      userAgent = `Mozilla/5.0 (Macintosh; Intel Mac OS X ${macVersion}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${chromeVersion}.0.0.${chromePatch} Safari/537.36`;
    } else {
      userAgent = `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${chromeVersion}.0.0.${chromePatch} Safari/537.36`;
    }

    setEditUserAgent(userAgent);
    toast.info("Đã tạo User Agent ngẫu nhiên");
  };

  const InfoRow = ({
    label,
    value,
    copyable = false,
    isPassword = false,
    editable = false,
    fieldName,
    extraAction,
  }: {
    label: string;
    value?: string | null;
    copyable?: boolean;
    isPassword?: boolean;
    editable?: boolean;
    fieldName?: "name" | "username" | "password" | "userAgent";
    extraAction?: React.ReactNode;
  }) => {
    const displayValue = value || "N/A";
    const maskedValue = isPassword && !showPassword ? "••••••••" : displayValue;

    if (isEditing && editable) {
      const editValue =
        fieldName === "name"
          ? editName
          : fieldName === "username"
            ? editUsername
            : fieldName === "password"
              ? editPassword
              : editUserAgent;
      const setEditValue =
        fieldName === "name"
          ? setEditName
          : fieldName === "username"
            ? setEditUsername
            : fieldName === "password"
              ? setEditPassword
              : setEditUserAgent;

      return (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label className="text-xs font-semibold text-muted-foreground">
              {label}
            </Label>
            {extraAction}
          </div>
          <div className="flex items-center gap-2">
            <div className="relative flex-1">
              <Input
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                className="bg-background pr-10"
                type={isPassword && !showPassword ? "password" : "text"}
              />
              {isPassword && (
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-0 top-0 h-full px-3 hover:bg-transparent"
                >
                  {showPassword ? (
                    <EyeOff className="w-4 h-4 text-muted-foreground" />
                  ) : (
                    <Eye className="w-4 h-4 text-muted-foreground" />
                  )}
                </Button>
              )}
            </div>
          </div>
        </div>
      );
    }

    return (
      <div className="space-y-1.5">
        <Label className="text-xs font-semibold text-muted-foreground">
          {label}
        </Label>
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Input
              readOnly
              value={maskedValue}
              className="bg-muted/30 border-dashed"
              type={isPassword && !showPassword ? "password" : "text"}
            />
            {isPassword && value && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-0 top-0 h-full px-3 hover:bg-transparent"
              >
                {showPassword ? (
                  <EyeOff className="w-4 h-4 text-muted-foreground" />
                ) : (
                  <Eye className="w-4 h-4 text-muted-foreground" />
                )}
              </Button>
            )}
          </div>
          {copyable && value && (
            <Button
              variant="outline"
              size="icon"
              onClick={() => copyToClipboard(value)}
              className="shrink-0 h-10 w-10 border-muted"
            >
              <Copy className="w-4 h-4" />
            </Button>
          )}
        </div>
      </div>
    );
  };

  const BrowserIcon = getBrowserIcon(profile.browser);

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-lg h-[90vh] flex flex-col p-0 overflow-hidden bg-background shadow-2xl border-none">
        <DialogHeader className="p-6 pb-2 shrink-0">
          <div className="flex items-center justify-between">
            <DialogTitle className="flex items-center gap-2">
              {BrowserIcon && <BrowserIcon className="w-5 h-5" />}
              {profile.name}
            </DialogTitle>
            {!isEditing && isManager && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setIsEditing(true)}
                className="gap-2"
                disabled={profile.is_cloud_only}
                title={
                  profile.is_cloud_only
                    ? "Vui lòng tải profile về máy trước khi sửa"
                    : ""
                }
              >
                <Edit2 className="w-4 h-4" />
                Sửa
              </Button>
            )}
          </div>
        </DialogHeader>

        <ScrollArea className="flex-1 min-h-0">
          <div className="p-6 space-y-6 pb-20">
            {profile.is_cloud_only && (
              <div className="p-3 rounded-md bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-900 flex items-center gap-2 text-xs text-amber-700 dark:text-amber-400">
                <HiSparkles className="w-4 h-4" />
                <span>
                  Profile này đang được lưu trên Cloud. Bạn cần tải về máy để có
                  thể chỉnh sửa thông tin.
                </span>
              </div>
            )}
            {/* Basic Info */}
            <div className="space-y-4">
              <h3 className="text-sm font-semibold border-b pb-2">
                Thông tin cơ bản
              </h3>
              <div className="grid grid-cols-2 gap-4">
                <InfoRow label="ID" value={profile.id} copyable />
                <InfoRow
                  label="Tên"
                  value={profile.name}
                  editable
                  fieldName="name"
                />
                <InfoRow label="Trình duyệt" value={profile.browser} />
                <InfoRow label="Phiên bản" value={profile.version} />
                <InfoRow label="Release Type" value={profile.release_type} />
                <InfoRow
                  label="Ngày tạo"
                  value={formatDate(profile.created_at)}
                />
              </div>
            </div>

            {/* Account Info */}
            <div className="space-y-4">
              <h3 className="text-sm font-semibold border-b pb-2">
                Thông tin tài khoản
              </h3>
              <div className="grid grid-cols-2 gap-4">
                <InfoRow
                  label="Account"
                  value={profile.username}
                  copyable
                  editable
                  fieldName="username"
                />
                <InfoRow
                  label="Password"
                  value={profile.password}
                  copyable
                  isPassword
                  editable
                  fieldName="password"
                />
              </div>
            </div>

            {/* Fingerprint Info */}
            <div className="space-y-4">
              <h3 className="text-sm font-semibold border-b pb-2">
                Dấu vân tay (Fingerprint)
              </h3>
              <div className="space-y-4">
                <InfoRow
                  label="User Agent"
                  value={profile.user_agent}
                  copyable
                  editable
                  fieldName="userAgent"
                  extraAction={
                    isEditing && (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={generateRandomUserAgent}
                        className="h-6 text-[10px] gap-1 text-primary hover:text-primary hover:bg-primary/10 px-2"
                      >
                        <HiSparkles className="w-3 h-3" />
                        Tạo ngẫu nhiên
                      </Button>
                    )
                  }
                />
              </div>
            </div>

            {/* Sync Info */}
            <div className="space-y-4">
              <h3 className="text-sm font-semibold border-b pb-2">Đồng bộ</h3>
              <div className="grid grid-cols-2 gap-4">
                <InfoRow
                  label="Sync"
                  value={profile.sync_enabled ? "Enabled" : "Disabled"}
                />
                <InfoRow
                  label="Last Sync"
                  value={formatDate(profile.last_sync)}
                />
                <InfoRow label="Odoo ID" value={profile.odoo_id} copyable />
                <InfoRow
                  label="Profile URL"
                  value={profile.profile_url}
                  copyable
                />
              </div>
            </div>

            {/* Notes */}
            {profile.note && (
              <div className="space-y-4">
                <h3 className="text-sm font-semibold border-b pb-2">Ghi chú</h3>
                <p className="text-sm text-muted-foreground whitespace-pre-wrap">
                  {profile.note}
                </p>
              </div>
            )}

            {/* Tags */}
            {profile.tags && profile.tags.length > 0 && (
              <div className="space-y-4">
                <h3 className="text-sm font-semibold border-b pb-2">Tags</h3>
                <div className="flex flex-wrap gap-2">
                  {profile.tags.map((tag, idx) => (
                    <span
                      key={idx}
                      className="px-2 py-1 text-xs rounded-full bg-primary/10 text-primary"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        </ScrollArea>

        {isEditing && (
          <div className="flex items-center justify-end gap-2 p-6 border-t bg-background shrink-0 mt-auto shadow-[0_-4px_10px_rgba(0,0,0,0.05)]">
            <Button
              variant="outline"
              onClick={() => setIsEditing(false)}
              disabled={isSaving}
              className="gap-2"
            >
              <X className="w-4 h-4" />
              Hủy
            </Button>
            <Button onClick={handleSave} disabled={isSaving} className="gap-2">
              <Check className="w-4 h-4" />
              {isSaving ? "Đang lưu..." : "Lưu thay đổi"}
            </Button>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
