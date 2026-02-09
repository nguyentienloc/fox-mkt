"use client";

import {
  type ColumnDef,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  type RowSelectionState,
  type SortingState,
  useReactTable,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { format } from "date-fns";
import type { Dispatch, SetStateAction } from "react";
import * as React from "react";
import { FiWifi } from "react-icons/fi";
import { IoEllipsisHorizontal } from "react-icons/io5";
import { LuChevronDown, LuChevronUp, LuCloud, LuLoader } from "react-icons/lu";
import { DeleteConfirmationDialog } from "@/components/delete-confirmation-dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useBrowserState } from "@/hooks/use-browser-state";
import { useProxyEvents } from "@/hooks/use-proxy-events";
import { useTableSorting } from "@/hooks/use-table-sorting";
import { getBrowserIcon } from "@/lib/browser-utils";
import { trimName } from "@/lib/name-utils";
import type {
  BrowserProfile,
  ProxyCheckResult,
  TrafficSnapshot,
} from "@/types";
import { ProxyCheckButton } from "./proxy-check-button";
import { Input } from "./ui/input";
import { RippleButton } from "./ui/ripple";

interface ProfilesDataTableProps {
  profiles: BrowserProfile[];
  onLaunchProfile: (profile: BrowserProfile) => void | Promise<void>;
  onKillProfile: (profile: BrowserProfile) => void | Promise<void>;
  onCloneProfile: (profile: BrowserProfile) => void | Promise<void>;
  onDeleteProfile: (
    profile: BrowserProfile,
    deleteFromServer?: boolean,
  ) => void | Promise<void>;
  onRenameProfile: (profileId: string, newName: string) => Promise<void>;
  onConfigureCamoufox: (profile: BrowserProfile) => void;
  onCopyCookiesToProfile?: (profile: BrowserProfile) => void;
  runningProfiles: Set<string>;
  isUpdating: (browser: string) => boolean;
  onDeleteSelectedProfiles: (profileIds: string[]) => Promise<void>;
  onAssignProfilesToGroup: (profileIds: string[]) => void;
  selectedGroupId: string | null;
  selectedProfiles: string[];
  onSelectedProfilesChange: Dispatch<SetStateAction<string[]>>;
  onOpenProfileSyncDialog?: (profile: BrowserProfile) => void;
  onToggleProfileSync?: (profile: BrowserProfile) => void;
  onUploadToOdoo?: (profile: BrowserProfile) => Promise<void>;
  onDownloadFromOdoo?: (profile: BrowserProfile) => Promise<void>;
  onImportCloudProfile?: (profile: any) => Promise<void>;
  uploadingProfiles?: Set<string>;
  appVersion?: string;
  onViewProfileDetails?: (profile: BrowserProfile) => void;
  isManager?: boolean;
}

interface TableMeta {
  selectedProfiles: string[];
  selectableCount: number;
  showCheckboxes: boolean;
  isClient: boolean;
  runningProfiles: Set<string>;
  launchingProfiles: Set<string>;
  stoppingProfiles: Set<string>;
  isUpdating: (browser: string) => boolean;
  browserState: any;

  proxyOverrides: Record<string, string | null>;
  storedProxies: any[];
  handleProxySelection: (profileId: string, proxyId: string | null) => void;
  checkingProfileId: string | null;
  proxyCheckResults: Record<string, ProxyCheckResult>;

  isProfileSelected: (id: string) => boolean;
  handleToggleAll: (checked: boolean) => void;
  handleCheckboxChange: (id: string, checked: boolean) => void;
  handleIconClick: (id: string) => void;

  handleRename: () => void | Promise<void>;
  setProfileToRename: React.Dispatch<
    React.SetStateAction<BrowserProfile | null>
  >;
  setProfileToDelete: React.Dispatch<
    React.SetStateAction<BrowserProfile | null>
  >;
  setNewProfileName: React.Dispatch<React.SetStateAction<string>>;
  setRenameError: React.Dispatch<React.SetStateAction<string | null>>;
  profileToRename: BrowserProfile | null;
  newProfileName: string;
  isRenamingSaving: boolean;

  setLaunchingProfiles: React.Dispatch<React.SetStateAction<Set<string>>>;
  setStoppingProfiles: React.Dispatch<React.SetStateAction<Set<string>>>;
  onKillProfile: (profile: BrowserProfile) => void | Promise<void>;
  onLaunchProfile: (profile: BrowserProfile) => void | Promise<void>;

  onAssignProfilesToGroup?: (profileIds: string[]) => void;
  onCloneProfile?: (profile: BrowserProfile) => void;
  onConfigureCamoufox?: (profile: BrowserProfile) => void;
  onCopyCookiesToProfile?: (profile: BrowserProfile) => void;

  trafficSnapshots: Record<string, TrafficSnapshot>;
  onOpenTrafficDialog?: (profileId: string) => void;

  syncStatuses: Record<string, string>;
  onOpenProfileSyncDialog?: (profile: BrowserProfile) => void;
  onToggleProfileSync?: (profile: BrowserProfile) => void;
  onUploadToOdoo?: (profile: BrowserProfile) => Promise<void>;
  onDownloadFromOdoo?: (profile: BrowserProfile) => Promise<void>;
  onImportCloudProfile?: (profile: any) => Promise<void>;
  uploadingProfiles: Set<string>;
  onViewProfileDetails?: (profile: BrowserProfile) => void;
  isManager: boolean;
}

export function ProfilesDataTableVirtual({
  profiles,
  onLaunchProfile,
  onKillProfile,
  onCloneProfile,
  onDeleteProfile,
  onRenameProfile,
  onConfigureCamoufox,
  onCopyCookiesToProfile,
  runningProfiles,
  isUpdating,
  onAssignProfilesToGroup,
  selectedProfiles,
  onSelectedProfilesChange,
  onUploadToOdoo,
  onDownloadFromOdoo,
  onImportCloudProfile,
  uploadingProfiles = new Set(),
  appVersion,
  onViewProfileDetails,
  isManager = false,
}: ProfilesDataTableProps) {
  const { updateSorting } = useTableSorting();

  const [sorting, setSorting] = React.useState<SortingState>([
    { id: "created_at", desc: true },
  ]);

  const [rowSelection, setRowSelection] = React.useState<RowSelectionState>({});
  const prevSelectedProfilesRef = React.useRef<string[]>(selectedProfiles);

  React.useEffect(() => {
    if (
      prevSelectedProfilesRef.current.length !== selectedProfiles.length ||
      !prevSelectedProfilesRef.current.every((id) =>
        selectedProfiles.includes(id),
      )
    ) {
      const newSelection: RowSelectionState = {};
      for (const profileId of selectedProfiles) {
        newSelection[profileId] = true;
      }
      setRowSelection(newSelection);
      prevSelectedProfilesRef.current = selectedProfiles;
    }
  }, [selectedProfiles]);

  const handleRowSelectionChange = React.useCallback(
    (updater: React.SetStateAction<RowSelectionState>) => {
      setRowSelection((prevSelection) => {
        const newSelection =
          typeof updater === "function" ? updater(prevSelection) : updater;
        const selectedIds = Object.keys(newSelection).filter(
          (id) => newSelection[id],
        );
        const prevIds = Object.keys(prevSelection).filter(
          (id) => prevSelection[id],
        );
        if (
          selectedIds.length !== prevIds.length ||
          !selectedIds.every((id) => prevIds.includes(id))
        ) {
          onSelectedProfilesChange(selectedIds);
        }
        return newSelection;
      });
    },
    [onSelectedProfilesChange],
  );

  const [profileToRename, setProfileToRename] =
    React.useState<BrowserProfile | null>(null);
  const [newProfileName, setNewProfileName] = React.useState("");
  const [_renameError, setRenameError] = React.useState<string | null>(null);
  const [isRenamingSaving, setIsRenamingSaving] = React.useState(false);
  const [profileToDelete, setProfileToDelete] =
    React.useState<BrowserProfile | null>(null);
  const [isDeleting, setIsDeleting] = React.useState(false);
  const [launchingProfiles, setLaunchingProfiles] = React.useState<Set<string>>(
    new Set(),
  );
  const [stoppingProfiles, setStoppingProfiles] = React.useState<Set<string>>(
    new Set(),
  );

  const { storedProxies } = useProxyEvents();
  const [proxyOverrides, _setProxyOverrides] = React.useState<
    Record<string, string | null>
  >({});
  const [showCheckboxes, setShowCheckboxes] = React.useState(false);
  const [_openProxySelectorFor, setOpenProxySelectorFor] = React.useState<
    string | null
  >(null);
  const [checkingProfileId, setCheckingProfileId] = React.useState<
    string | null
  >(null);
  const [proxyCheckResults, setProxyCheckResults] = React.useState<
    Record<string, ProxyCheckResult>
  >({});
  const [trafficSnapshots, setTrafficSnapshots] = React.useState<
    Record<string, TrafficSnapshot>
  >({});
  const [_trafficDialogProfile, setTrafficDialogProfile] = React.useState<{
    id: string;
    name?: string;
  } | null>(null);
  const [syncStatuses, setSyncStatuses] = React.useState<
    Record<string, string>
  >({});

  const handleProxySelection = React.useCallback(
    async (profileId: string, proxyId: string | null) => {
      try {
        await invoke("update_profile_proxy", { profileId, proxyId });
        // setProxyOverrides((prev) => ({ ...prev, [profileId]: proxyId }));
        await emit("profile-updated");
      } catch (error) {
        console.error("Failed to update proxy settings:", error);
      } finally {
        setOpenProxySelectorFor(null);
      }
    },
    [],
  );

  const browserState = useBrowserState(
    profiles,
    runningProfiles,
    isUpdating,
    launchingProfiles,
    stoppingProfiles,
  );

  React.useEffect(() => {
    if (!browserState.isClient) return;
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        unlisten = await listen<{ profile_id: string; status: string }>(
          "profile-sync-status",
          (event) => {
            const { profile_id, status } = event.payload;
            setSyncStatuses((prev) => ({ ...prev, [profile_id]: status }));
          },
        );
      } catch (error) {
        console.error("Failed to listen for sync status events:", error);
      }
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [browserState.isClient]);

  const runningProfileIds = React.useMemo(
    () => Array.from(runningProfiles).sort(),
    [runningProfiles],
  );
  const runningCount = runningProfileIds.length;

  React.useEffect(() => {
    if (!browserState.isClient || runningCount === 0) {
      if (runningCount === 0) setTrafficSnapshots({});
      return;
    }
    const fetchTrafficSnapshots = async () => {
      try {
        const allSnapshots = await invoke<TrafficSnapshot[]>(
          "get_all_traffic_snapshots",
        );
        const newSnapshots: Record<string, TrafficSnapshot> = {};
        for (const snapshot of allSnapshots) {
          if (
            snapshot.profile_id &&
            runningProfileIds.includes(snapshot.profile_id)
          ) {
            const existing = newSnapshots[snapshot.profile_id];
            if (!existing || snapshot.last_update > existing.last_update) {
              newSnapshots[snapshot.profile_id] = snapshot;
            }
          }
        }
        setTrafficSnapshots(newSnapshots);
      } catch (error) {
        console.error("Failed to fetch traffic snapshots:", error);
      }
    };
    void fetchTrafficSnapshots();
    const interval = setInterval(fetchTrafficSnapshots, 1000);
    return () => clearInterval(interval);
  }, [browserState.isClient, runningCount, runningProfileIds]);

  React.useEffect(() => {
    if (!browserState.isClient) return;
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        unlisten = await listen<{ id: string; is_running: boolean }>(
          "profile-running-changed",
          (event) => {
            const { id } = event.payload;
            setLaunchingProfiles((prev) => {
              if (!prev.has(id)) return prev;
              const next = new Set(prev);
              next.delete(id);
              return next;
            });
            setStoppingProfiles((prev) => {
              if (!prev.has(id)) return prev;
              const next = new Set(prev);
              next.delete(id);
              return next;
            });
          },
        );
      } catch (error) {
        console.error("Failed to listen for profile running changes:", error);
      }
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [browserState.isClient]);

  const handleSortingChange = React.useCallback(
    (updater: React.SetStateAction<SortingState>) => {
      if (!browserState.isClient) return;
      const newSorting =
        typeof updater === "function" ? updater(sorting) : updater;
      setSorting(newSorting);
      updateSorting(newSorting);
    },
    [browserState.isClient, sorting, updateSorting],
  );

  const handleRename = React.useCallback(async () => {
    if (!profileToRename || !newProfileName.trim()) return;
    try {
      setIsRenamingSaving(true);
      await onRenameProfile(profileToRename.id, newProfileName.trim());
      setProfileToRename(null);
      setNewProfileName("");
      setRenameError(null);
    } catch (error) {
      setRenameError(
        error instanceof Error ? error.message : "Failed to rename profile",
      );
    } finally {
      setIsRenamingSaving(false);
    }
  }, [profileToRename, newProfileName, onRenameProfile]);

  const handleDelete = async (deleteFromServer?: boolean) => {
    if (!profileToDelete) return;
    setIsDeleting(true);
    const minLoadingTime = new Promise((r) => setTimeout(r, 300));
    try {
      await Promise.all([
        onDeleteProfile(profileToDelete, deleteFromServer),
        minLoadingTime,
      ]);
      setProfileToDelete(null);
    } catch (error) {
      console.error("Failed to delete profile:", error);
    } finally {
      setIsDeleting(false);
    }
  };

  const handleIconClick = React.useCallback(
    (profileId: string) => {
      const profile = profiles.find((p) => p.id === profileId);
      if (!profile || !browserState.canSelectProfile(profile)) return;
      setShowCheckboxes(true);
      const newSet = new Set(selectedProfiles);
      if (newSet.has(profileId)) newSet.delete(profileId);
      else newSet.add(profileId);
      if (newSet.size === 0) setShowCheckboxes(false);
      onSelectedProfilesChange(Array.from(newSet));
    },
    [profiles, browserState, selectedProfiles, onSelectedProfilesChange],
  );

  const handleCheckboxChange = React.useCallback(
    (profileId: string, checked: boolean) => {
      const newSet = new Set(selectedProfiles);
      if (checked) newSet.add(profileId);
      else newSet.delete(profileId);
      if (newSet.size === 0) setShowCheckboxes(false);
      onSelectedProfilesChange(Array.from(newSet));
    },
    [onSelectedProfilesChange, selectedProfiles],
  );

  const handleToggleAll = React.useCallback(
    (checked: boolean) => {
      const newSet = checked
        ? new Set(
            profiles
              .filter((profile) => {
                const isRunning =
                  browserState.isClient && runningProfiles.has(profile.id);
                const isLaunching = launchingProfiles.has(profile.id);
                const isStopping = stoppingProfiles.has(profile.id);
                const isBrowserUpdating = isUpdating(profile.browser);
                return (
                  !isRunning &&
                  !isLaunching &&
                  !isStopping &&
                  !isBrowserUpdating
                );
              })
              .map((profile) => profile.id),
          )
        : new Set<string>();
      setShowCheckboxes(checked);
      onSelectedProfilesChange(Array.from(newSet));
    },
    [
      profiles,
      onSelectedProfilesChange,
      browserState.isClient,
      runningProfiles,
      launchingProfiles,
      stoppingProfiles,
      isUpdating,
    ],
  );

  const tableMeta = React.useMemo<TableMeta>(
    () => ({
      selectedProfiles,
      selectableCount: profiles.length,
      showCheckboxes,
      isClient: browserState.isClient,
      runningProfiles,
      launchingProfiles,
      stoppingProfiles,
      isUpdating,
      browserState,
      proxyOverrides,
      storedProxies,
      handleProxySelection,
      checkingProfileId,
      proxyCheckResults,
      isProfileSelected: (id: string) => selectedProfiles.includes(id),
      handleToggleAll,
      handleCheckboxChange,
      handleIconClick,
      handleRename,
      setProfileToRename,
      setProfileToDelete,
      setNewProfileName,
      setRenameError,
      profileToRename,
      newProfileName,
      isRenamingSaving,
      setLaunchingProfiles,
      setStoppingProfiles,
      onKillProfile,
      onLaunchProfile,
      onAssignProfilesToGroup,
      onCloneProfile,
      onConfigureCamoufox,
      onCopyCookiesToProfile,
      trafficSnapshots,
      onOpenTrafficDialog: (profileId: string) => {
        const profile = profiles.find((p) => p.id === profileId);
        setTrafficDialogProfile({ id: profileId, name: profile?.name });
      },
      syncStatuses,
      onUploadToOdoo,
      onDownloadFromOdoo,
      onImportCloudProfile,
      uploadingProfiles,
      onViewProfileDetails,
      isManager,
    }),
    [
      selectedProfiles,
      profiles,
      showCheckboxes,
      browserState,
      runningProfiles,
      launchingProfiles,
      stoppingProfiles,
      isUpdating,
      proxyOverrides,
      storedProxies,
      handleProxySelection,
      checkingProfileId,
      proxyCheckResults,
      handleRename,
      profileToRename,
      newProfileName,
      isRenamingSaving,
      onKillProfile,
      onLaunchProfile,
      onAssignProfilesToGroup,
      onCloneProfile,
      onConfigureCamoufox,
      onCopyCookiesToProfile,
      trafficSnapshots,
      syncStatuses,
      onDownloadFromOdoo,
      onUploadToOdoo,
      onImportCloudProfile,
      handleToggleAll,
      handleCheckboxChange,
      handleIconClick,
      uploadingProfiles,
      onViewProfileDetails,
      isManager,
    ],
  );

  const columns: ColumnDef<BrowserProfile>[] = React.useMemo(
    () => [
      {
        id: "select",
        header: ({ table }) => {
          const meta = table.options.meta as TableMeta;
          return (
            <Checkbox
              checked={
                meta.selectedProfiles.length === meta.selectableCount &&
                meta.selectableCount !== 0
              }
              onCheckedChange={(value) => meta.handleToggleAll(!!value)}
              aria-label="Select all"
              className="cursor-pointer"
            />
          );
        },
        cell: ({ row, table }) => {
          const meta = table.options.meta as TableMeta;
          const profile = row.original;
          const isCloudOnly = (profile as any).is_cloud_only;
          const isSelected = meta.isProfileSelected(profile.id);
          const IconComponent = getBrowserIcon(profile.browser);
          const isRunning =
            meta.isClient && meta.runningProfiles.has(profile.id);
          const isLaunching = meta.launchingProfiles.has(profile.id);
          const isStopping = meta.stoppingProfiles.has(profile.id);
          const isBrowserUpdating = meta.isUpdating(profile.browser);
          const isUploading = meta.uploadingProfiles?.has(profile.id);
          const isDisabled =
            isRunning ||
            isLaunching ||
            isStopping ||
            isBrowserUpdating ||
            isUploading;

          if (isCloudOnly) {
            return (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="flex justify-center items-center w-4 h-4 text-primary">
                    <LuCloud className="w-4 h-4" />
                  </span>
                </TooltipTrigger>
                <TooltipContent>Cloud Profile</TooltipContent>
              </Tooltip>
            );
          }

          if (isUploading) {
            return (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="flex justify-center items-center w-4 h-4 text-primary animate-spin">
                    <LuLoader className="w-4 h-4" />
                  </span>
                </TooltipTrigger>
                <TooltipContent>
                  <p>Đang đồng bộ dữ liệu...</p>
                </TooltipContent>
              </Tooltip>
            );
          }

          if (isDisabled) {
            return (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="flex justify-center items-center w-4 h-4 cursor-not-allowed">
                    {IconComponent && (
                      <IconComponent className="w-4 h-4 opacity-50" />
                    )}
                  </span>
                </TooltipTrigger>
                <TooltipContent>
                  <p>Profile is busy</p>
                </TooltipContent>
              </Tooltip>
            );
          }

          if (meta.showCheckboxes || isSelected) {
            return (
              <Checkbox
                checked={isSelected}
                onCheckedChange={(value) =>
                  meta.handleCheckboxChange(profile.id, !!value)
                }
                className="w-4 h-4"
              />
            );
          }

          return (
            <button
              type="button"
              className="flex justify-center items-center p-0 border-none cursor-pointer"
              onClick={() => meta.handleIconClick(profile.id)}
            >
              {IconComponent && <IconComponent className="w-4 h-4" />}
            </button>
          );
        },
        size: 40,
      },
      {
        id: "actions",
        cell: ({ row, table }) => {
          const meta = table.options.meta as TableMeta;
          const profile = row.original;
          const isCloudOnly = (profile as any).is_cloud_only;
          const isRunning =
            meta.isClient && meta.runningProfiles.has(profile.id);
          const isLaunching = meta.launchingProfiles.has(profile.id);
          const isStopping = meta.stoppingProfiles.has(profile.id);
          const canLaunch = meta.browserState.canLaunchProfile(profile);

          const isUploading = meta.uploadingProfiles?.has(profile.id);

          if (isCloudOnly) {
            return (
              <RippleButton
                size="sm"
                variant="outline"
                className="min-w-[70px] h-7 border-primary text-primary hover:bg-primary/10"
                disabled={isUploading}
                onClick={() => void meta.onImportCloudProfile?.(profile)}
              >
                {isUploading ? (
                  <div className="w-3 h-3 border border-current animate-spin border-t-transparent rounded-full" />
                ) : (
                  "Tải về"
                )}
              </RippleButton>
            );
          }

          return (
            <RippleButton
              variant={isRunning ? "destructive" : "default"}
              size="sm"
              disabled={!canLaunch || isLaunching || isStopping || isUploading}
              className="min-w-[70px] h-7"
              onClick={() =>
                isRunning
                  ? meta.onKillProfile(profile)
                  : meta.onLaunchProfile(profile)
              }
            >
              {isLaunching || isStopping || isUploading ? (
                <div className="w-3 h-3 border border-current animate-spin border-t-transparent rounded-full" />
              ) : isRunning ? (
                "Dừng"
              ) : (
                "Mở"
              )}
            </RippleButton>
          );
        },
        size: 80,
      },
      {
        accessorKey: "name",
        header: ({ column }) => (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="justify-start p-0 h-auto font-semibold"
          >
            Tên{" "}
            {column.getIsSorted() === "asc" ? (
              <LuChevronUp className="ml-2 w-4 h-4" />
            ) : (
              <LuChevronDown className="ml-2 w-4 h-4" />
            )}
          </Button>
        ),
        cell: ({ row, table }) => {
          const meta = table.options.meta as TableMeta;
          const profile = row.original;
          if (meta.profileToRename?.id === profile.id) {
            return (
              <Input
                autoFocus
                value={meta.newProfileName}
                onChange={(e) => meta.setNewProfileName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && void meta.handleRename()}
                className="h-6 text-sm"
              />
            );
          }
          return (
            <span className="text-left">{trimName(profile.name, 20)}</span>
          );
        },
      },
      {
        id: "sync_status",
        header: "Trạng thái",
        cell: ({ row }) => {
          const profile = row.original;
          const isCloudOnly = (profile as any).is_cloud_only;
          const hasProfileUrl =
            profile.profile_url && profile.profile_url.trim() !== "";

          if (isCloudOnly) {
            return (
              <span className="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300">
                Cloud
              </span>
            );
          }

          if (hasProfileUrl) {
            return (
              <span className="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300">
                Synced
              </span>
            );
          }

          return (
            <span className="inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300">
              Local
            </span>
          );
        },
      },
      {
        id: "local_path",
        header: "Đường dẫn",
        cell: ({ row }) => {
          const profile = row.original;
          const isCloudOnly = (profile as any).is_cloud_only;
          const hasProfileUrl =
            profile.profile_url && profile.profile_url.trim() !== "";
          const localPath =
            profile.absolute_path ||
            profile.note?.match(/original path: (.*)\)/)?.[1] ||
            (hasProfileUrl ? "Data từ Cloud" : "—");

          if (isCloudOnly)
            return (
              <div className="flex items-center gap-2 text-xs text-primary italic">
                <LuCloud className="w-3.5 h-3.5" /> Chưa có dữ liệu local
              </div>
            );
          return (
            <span
              className="text-xs text-muted-foreground truncate block max-w-[150px]"
              title={localPath}
            >
              {localPath}
            </span>
          );
        },
      },
      {
        id: "created_at",
        accessorKey: "created_at",
        header: ({ column }) => (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="justify-start p-0 h-auto font-semibold"
          >
            Ngày tạo{" "}
            {column.getIsSorted() === "asc" ? (
              <LuChevronUp className="ml-2 w-4 h-4" />
            ) : (
              <LuChevronDown className="ml-2 w-4 h-4" />
            )}
          </Button>
        ),
        cell: ({ row }) => {
          const timestamp = (row.original as any).created_at;
          if (!timestamp || timestamp === 0)
            return <span className="text-xs text-muted-foreground">—</span>;
          try {
            return (
              <span className="text-xs text-muted-foreground">
                {format(new Date(timestamp * 1000), "HH:mm dd/MM/yyyy")}
              </span>
            );
          } catch (_e) {
            return <span className="text-xs text-muted-foreground">—</span>;
          }
        },
      },
      {
        id: "proxy",
        header: "Proxy",
        cell: ({ row, table }) => {
          const meta = table.options.meta as TableMeta;
          const profile = row.original;
          const isOdooProfile = Boolean(profile.odoo_id);
          const odooProxy = (profile as any).odoo_proxy;
          const isRunning =
            meta.isClient && meta.runningProfiles.has(profile.id);
          const isDisabled = isRunning;

          if (isOdooProfile) {
            const hasProxy = Boolean(profile.proxy_id || odooProxy);
            const label = odooProxy
              ? `${odooProxy.ip}:${odooProxy.port}`
              : profile.proxy_id
                ? "Odoo Proxy"
                : "No Proxy";
            return (
              <div className="flex gap-2 items-center">
                <div className="flex flex-col">
                  <div className="flex gap-2 items-center text-sm">
                    <FiWifi
                      className={hasProxy ? "text-primary" : "text-muted"}
                    />{" "}
                    {label}
                  </div>
                  {!profile.profile_url && !(profile as any).is_cloud_only && (
                    <span className="text-[10px] text-orange-500 font-bold">
                      (Chưa đồng bộ S3)
                    </span>
                  )}
                </div>
                {hasProxy && !isDisabled && (
                  <ProxyCheckButton
                    proxy={
                      odooProxy
                        ? {
                            id: "temp-odoo",
                            name: "Odoo Proxy",
                            proxy_settings: {
                              proxy_type: odooProxy.giaothuc,
                              host: odooProxy.ip,
                              port: parseInt(odooProxy.port, 10),
                              username: odooProxy.tendangnhap,
                              password: odooProxy.matkhau,
                            },
                          }
                        : (meta.storedProxies.find(
                            (p) => p.id === profile.proxy_id,
                          ) as any)
                    }
                    profileId={profile.id}
                    checkingProfileId={meta.checkingProfileId}
                    cachedResult={meta.proxyCheckResults[profile.id]}
                    setCheckingProfileId={setCheckingProfileId}
                    onCheckComplete={(result) => {
                      setProxyCheckResults((prev) => ({
                        ...prev,
                        [profile.id]: result,
                      }));
                    }}
                    onCheckFailed={(result) => {
                      setProxyCheckResults((prev) => ({
                        ...prev,
                        [profile.id]: result,
                      }));
                    }}
                  />
                )}
              </div>
            );
          }
          return (
            <span className="text-sm text-muted-foreground">
              (Chỉ lưu Local)
            </span>
          );
        },
      },
      {
        id: "settings",
        header: "",
        cell: ({ row, table }) => {
          const meta = table.options.meta as TableMeta;
          const profile = row.original;
          const isCloudOnly = (profile as any).is_cloud_only;
          return (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm">
                  <IoEllipsisHorizontal />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  onClick={() => meta.onViewProfileDetails?.(profile)}
                >
                  Xem chi tiết
                </DropdownMenuItem>
                {!isCloudOnly && meta.isManager && (
                  <DropdownMenuItem
                    onClick={() => meta.onCloneProfile?.(profile)}
                    disabled={
                      meta.runningProfiles.has(profile.id) ||
                      meta.uploadingProfiles.has(profile.id)
                    }
                  >
                    Nhân bản
                  </DropdownMenuItem>
                )}
                {!isCloudOnly && (
                  <DropdownMenuItem
                    onClick={() => meta.onAssignProfilesToGroup?.([profile.id])}
                    disabled={
                      meta.runningProfiles.has(profile.id) ||
                      meta.uploadingProfiles.has(profile.id)
                    }
                  >
                    Thêm vào nhóm
                  </DropdownMenuItem>
                )}
                {!isCloudOnly && (
                  <DropdownMenuItem
                    onClick={() => meta.onUploadToOdoo?.(profile)}
                    disabled={
                      meta.runningProfiles.has(profile.id) ||
                      meta.uploadingProfiles.has(profile.id)
                    }
                  >
                    {meta.uploadingProfiles.has(profile.id)
                      ? "Đang đẩy lên..."
                      : "Đẩy lên Odoo (S3)"}
                  </DropdownMenuItem>
                )}
                {!isCloudOnly && (
                  <DropdownMenuItem
                    onClick={() => meta.onDownloadFromOdoo?.(profile)}
                  >
                    Tải về từ Odoo (S3)
                  </DropdownMenuItem>
                )}
                {isCloudOnly && (
                  <DropdownMenuItem
                    onClick={() => meta.onImportCloudProfile?.(profile)}
                  >
                    Nhập Profile về máy
                  </DropdownMenuItem>
                )}
                {meta.isManager && (
                  <DropdownMenuItem
                    className="text-destructive"
                    onClick={() => meta.setProfileToDelete(profile)}
                  >
                    Xóa
                  </DropdownMenuItem>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          );
        },
        size: 50,
      },
    ],
    [],
  );
  const [statsFilter, setStatsFilter] = React.useState<
    "all" | "local" | "synced"
  >("all");

  const { localCount, syncedCount, filteredData } = React.useMemo(() => {
    const local = profiles.filter((p) => !p.odoo_id);
    const synced = profiles.filter((p) => !!p.odoo_id);

    let data = profiles;
    if (statsFilter === "local") data = local;
    if (statsFilter === "synced") data = synced;

    return {
      localCount: local.length,
      syncedCount: synced.length,
      filteredData: data,
    };
  }, [profiles, statsFilter]);

  const table = useReactTable({
    data: filteredData,
    columns,
    state: {
      sorting,
      rowSelection,
    },
    onSortingChange: handleSortingChange,
    onRowSelectionChange: handleRowSelectionChange,
    getSortedRowModel: getSortedRowModel(),
    getCoreRowModel: getCoreRowModel(),
    getRowId: (row) => row.id,
    meta: tableMeta,
  });

  const { rows } = table.getRowModel();

  // Virtual scroll
  const parentRef = React.useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 52, // Row height estimate
    overscan: 10,
  });

  const virtualRows = virtualizer.getVirtualItems();
  const totalSize = virtualizer.getTotalSize();

  return (
    <div className="flex flex-col h-full border rounded-md overflow-hidden bg-background">
      <div ref={parentRef} className="flex-1 overflow-auto relative">
        <Table>
          <TableHeader className="sticky top-0 bg-background z-10 shadow-sm">
            {table.getHeaderGroups().map((hg) => (
              <TableRow key={hg.id}>
                {hg.headers.map((h) => (
                  <TableHead key={h.id}>
                    {flexRender(h.column.columnDef.header, h.getContext())}
                  </TableHead>
                ))}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {/* Spacer for virtual scroll */}
            {virtualRows.length > 0 && (
              <tr style={{ height: virtualRows[0]?.start || 0 }} />
            )}
            {virtualRows.map((virtualRow) => {
              const row = rows[virtualRow.index];
              return (
                <TableRow
                  key={row.id}
                  className="hover:bg-accent/50"
                  data-index={virtualRow.index}
                  ref={virtualizer.measureElement}
                >
                  {row.getVisibleCells().map((c) => (
                    <TableCell key={c.id}>
                      {flexRender(c.column.columnDef.cell, c.getContext())}
                    </TableCell>
                  ))}
                </TableRow>
              );
            })}
            {/* Bottom spacer */}
            {virtualRows.length > 0 && (
              <tr
                style={{
                  height:
                    totalSize - (virtualRows[virtualRows.length - 1]?.end || 0),
                }}
              />
            )}
          </TableBody>
        </Table>
      </div>
      <div className="p-2 border-t flex items-center gap-2 bg-muted/30">
        <Button
          variant={statsFilter === "all" ? "default" : "outline"}
          size="sm"
          onClick={() => setStatsFilter("all")}
          className={
            statsFilter === "all"
              ? "bg-orange-500 hover:bg-orange-600 text-white border-0"
              : "bg-background"
          }
        >
          <span className="font-bold mr-1">{profiles.length}</span>
          Tổng số Profile
        </Button>
        <Button
          variant={statsFilter === "local" ? "default" : "outline"}
          size="sm"
          onClick={() => setStatsFilter("local")}
          className={
            statsFilter === "local"
              ? "bg-orange-500 hover:bg-orange-600 text-white border-0"
              : "bg-background"
          }
        >
          <span className="font-bold mr-1">{localCount}</span>
          Profile trên máy
        </Button>
        <Button
          variant={statsFilter === "synced" ? "default" : "outline"}
          size="sm"
          onClick={() => setStatsFilter("synced")}
          className={
            statsFilter === "synced"
              ? "bg-orange-500 hover:bg-orange-600 text-white border-0"
              : "bg-background"
          }
        >
          <span className="font-bold mr-1">{syncedCount}</span>
          Profile đã đồng bộ
        </Button>
        <div className="ml-auto text-[10px] text-orange-500/70 font-mono">
          App version: {appVersion || "..."}
        </div>
      </div>
      <DeleteConfirmationDialog
        isOpen={profileToDelete !== null}
        onClose={() => setProfileToDelete(null)}
        onConfirm={handleDelete}
        title="Xóa Profile"
        description={`Bạn có chắc chắn muốn xóa profile "${profileToDelete?.name}"?`}
        confirmButtonText="Xóa ngay"
        isLoading={isDeleting}
        isSynced={!!(profileToDelete?.odoo_id || profileToDelete?.profile_url)}
      />
    </div>
  );
}
