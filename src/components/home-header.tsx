import { useTranslation } from "react-i18next";
import { GoGear, GoKebabHorizontal } from "react-icons/go";
import {
  LuCloud,
  LuDownload,
  LuLogOut,
  LuRefreshCw,
  LuSearch,
  LuUser,
  LuUsers,
  LuX,
} from "react-icons/lu";
import { useAuth } from "@/providers/auth-provider";
import { ThemeToggle } from "./theme-toggle";
import { Button } from "./ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { Input } from "./ui/input";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";

type Props = {
  onSettingsDialogOpen: (open: boolean) => void;
  _onProxyManagementDialogOpen: (open: boolean) => void;
  onGroupManagementDialogOpen: (open: boolean) => void;
  _onImportProfileDialogOpen: (open: boolean) => void;
  _onZsmktImportDialogOpen: (open: boolean) => void;
  onOdooImportDialogOpen: (open: boolean) => void;
  _onSyncConfigDialogOpen: (open: boolean) => void;
  _onIntegrationsDialogOpen: (open: boolean) => void;
  onCheckAppUpdate: () => void;
  appUpdateInfo: any;
  searchQuery: string;
  onSearchQueryChange: (query: string) => void;
};

const HomeHeader = ({
  onSettingsDialogOpen,
  _onProxyManagementDialogOpen,
  onGroupManagementDialogOpen,
  _onImportProfileDialogOpen,
  _onZsmktImportDialogOpen,
  onOdooImportDialogOpen,
  _onSyncConfigDialogOpen,
  _onIntegrationsDialogOpen,
  onCheckAppUpdate,
  appUpdateInfo,
  searchQuery,
  onSearchQueryChange,
}: Props) => {
  const { t } = useTranslation();
  const { isLoggedIn, logout, username, isManager } = useAuth();
  const _handleLogoClick = () => {
    // Trigger the same URL handling logic as if the URL came from the system
    const event = new CustomEvent("url-open-request", {
      detail: "https://github.com/nguyentienloc/fox-mkt",
    });
    window.dispatchEvent(event);
  };
  return (
    <div className="flex justify-between items-center mt-2 w-full px-0">
      <div className="flex gap-3 items-center">
        {/* <button
          type="button"
          className="p-1 cursor-pointer"
          title="Open Foxia-MKT on GitHub"
          onClick={handleLogoClick}
        >
          <Logo className="w-10 h-10 transition-transform duration-300 ease-out will-change-transform hover:scale-110" />
        </button>
        <CardTitle>Foxia</CardTitle> */}
      </div>
      <div className="flex gap-2 items-center">
        <div className="relative">
          <Input
            type="text"
            placeholder={t("header.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => onSearchQueryChange(e.target.value)}
            className="pr-8 pl-10 w-96"
          />
          <LuSearch className="absolute left-3 top-1/2 w-4 h-4 transform -translate-y-1/2 text-muted-foreground" />
          {searchQuery && (
            <button
              type="button"
              onClick={() => onSearchQueryChange("")}
              className="absolute right-2 top-1/2 p-1 rounded-sm transition-colors transform -translate-y-1/2 hover:bg-accent"
              aria-label={t("header.clearSearch")}
            >
              <LuX className="w-4 h-4 text-muted-foreground hover:text-foreground" />
            </button>
          )}
        </div>
        <ThemeToggle />
        {appUpdateInfo && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                size="sm"
                variant="outline"
                onClick={onCheckAppUpdate}
                className="h-[36px] w-[36px] p-0 border-orange-500 text-orange-500 hover:bg-orange-500/10"
              >
                <LuDownload className="w-4 h-4 animate-bounce" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              Bản cập nhật {appUpdateInfo.new_version} đã sẵn sàng
            </TooltipContent>
          </Tooltip>
        )}
        {isLoggedIn && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <span>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <span>
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-[36px] px-2 gap-2"
                      >
                        <LuUser className="w-4 h-4 text-primary" />
                        <span className="text-sm font-medium max-w-[300px] truncate">
                          {username || "User"}
                        </span>
                      </Button>
                    </span>
                  </TooltipTrigger>
                  <TooltipContent>
                    {t("header.menu.odooAccount")}
                  </TooltipContent>
                </Tooltip>
              </span>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={() => void logout()}
                className="text-destructive"
              >
                <LuLogOut className="mr-2 w-4 h-4" />
                {t("header.menu.logoutOdoo")}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <span>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span>
                    <Button
                      size="sm"
                      variant="outline"
                      className="flex gap-2 items-center h-[36px]"
                    >
                      <GoKebabHorizontal className="w-4 h-4" />
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent>{t("header.moreActions")}</TooltipContent>
              </Tooltip>
            </span>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              onClick={() => {
                onSettingsDialogOpen(true);
              }}
            >
              <GoGear className="mr-2 w-4 h-4" />
              {t("header.menu.settings")}
            </DropdownMenuItem>
            {/* Tạm ẩn Proxy
            <DropdownMenuItem
              onClick={() => {
                onProxyManagementDialogOpen(true);
              }}
            >
              <FiWifi className="mr-2 w-4 h-4" />
              {t("header.menu.proxies")}
            </DropdownMenuItem>
            */}
            <DropdownMenuItem
              onClick={() => {
                onGroupManagementDialogOpen(true);
              }}
            >
              <LuUsers className="mr-2 w-4 h-4" />
              {t("header.menu.groups")}
            </DropdownMenuItem>
            {/* Tạm ẩn Dịch vụ đồng bộ
            <DropdownMenuItem
              onClick={() => {
                onSyncConfigDialogOpen(true);
              }}
            >
              <LuCloud className="mr-2 w-4 h-4" />
              {t("header.menu.syncService")}
            </DropdownMenuItem>
            */}
            {/* Tạm ẩn Tích hợp
            <DropdownMenuItem
              onClick={() => {
                onIntegrationsDialogOpen(true);
              }}
            >
              <LuPlug className="mr-2 w-4 h-4" />
              {t("header.menu.integrations")}
            </DropdownMenuItem>
            */}
            {/* Tạm ẩn Nhập từ zs-mkt
            <DropdownMenuItem
              onClick={() => {
                onZsmktImportDialogOpen(true);
              }}
            >
              <FaDownload className="mr-2 w-4 h-4 text-orange-500" />
              {t("header.menu.importZsmkt")}
            </DropdownMenuItem>
            */}
            {isLoggedIn && isManager && (
              <DropdownMenuItem
                onClick={() => {
                  onOdooImportDialogOpen(true);
                }}
              >
                <LuCloud className="mr-2 w-4 h-4 text-primary" />
                {t("header.menu.importOdoo")}
              </DropdownMenuItem>
            )}
            {/* Tạm ẩn Nhập Profile
            <DropdownMenuItem
              onClick={() => {
                onImportProfileDialogOpen(true);
              }}
            >
              <FaDownload className="mr-2 w-4 h-4" />
              {t("header.menu.importProfile")}
            </DropdownMenuItem>
            */}
            <DropdownMenuItem onClick={onCheckAppUpdate}>
              <LuRefreshCw className="mr-2 w-4 h-4" />
              {t("header.menu.checkUpdate")}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
};

export default HomeHeader;
