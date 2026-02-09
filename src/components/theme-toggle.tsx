"use client";

import { invoke } from "@tauri-apps/api/core";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";
import { LuMoon, LuSun } from "react-icons/lu";
import { clearThemeColors } from "@/lib/themes";

export function ThemeToggle() {
  const { setTheme, resolvedTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return <div className="w-12 h-6 rounded-full bg-muted animate-pulse" />;
  }

  const isDark = resolvedTheme === "dark";

  const toggleTheme = async () => {
    const newTheme = isDark ? "light" : "dark";
    setTheme(newTheme);

    try {
      // Clear custom theme colors if we are switching to light/dark
      clearThemeColors();

      // Load current settings to preserve other fields
      const currentSettings = await invoke<any>("get_app_settings");
      await invoke("save_app_settings", {
        settings: {
          ...currentSettings,
          theme: newTheme,
          custom_theme: undefined, // Reset custom theme when manually toggling light/dark
        },
      });
    } catch (error) {
      console.error("Failed to save theme setting:", error);
    }
  };

  return (
    <button
      type="button"
      onClick={toggleTheme}
      className={`
        relative inline-flex h-6 w-12 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent 
        transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2
        ${isDark ? "bg-slate-700" : "bg-slate-200"}
      `}
      aria-label="Toggle theme"
    >
      <span
        className={`
          pointer-events-none relative inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 
          transition duration-200 ease-in-out
          ${isDark ? "translate-x-6" : "translate-x-0"}
        `}
      >
        <span
          className={`
            absolute inset-0 flex h-full w-full items-center justify-center transition-opacity
            ${isDark ? "opacity-0 duration-100 ease-out" : "opacity-100 duration-200 ease-in"}
          `}
          aria-hidden="true"
        >
          <LuSun className="h-3 w-3 text-orange-500" />
        </span>
        <span
          className={`
            absolute inset-0 flex h-full w-full items-center justify-center transition-opacity
            ${isDark ? "opacity-100 duration-200 ease-in" : "opacity-0 duration-100 ease-out"}
          `}
          aria-hidden="true"
        >
          <LuMoon className="h-3 w-3 text-slate-700" />
        </span>
      </span>
    </button>
  );
}
