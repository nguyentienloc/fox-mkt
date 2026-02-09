"use client";

import { invoke } from "@tauri-apps/api/core";
import { usePathname, useRouter } from "next/navigation";
import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useState,
} from "react";
import { LaunchingScreen } from "@/components/launching-screen";

interface AuthContextType {
  isLoggedIn: boolean;
  isLoading: boolean;
  username: string | null;
  isManager: boolean;
  login: (
    baseUrl: string,
    login: string,
    pass: string,
  ) => Promise<{
    session_id?: string;
    name?: string;
    is_quanlytainguyen?: boolean;
  }>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [username, setUsername] = useState<string | null>(null);
  const [isManager, setIsManager] = useState(false);
  const router = useRouter();
  const pathname = usePathname();

  useEffect(() => {
    const checkAuth = async () => {
      try {
        const loggedIn = await invoke<boolean>("is_odoo_logged_in");
        setIsLoggedIn(loggedIn);

        if (loggedIn) {
          const storedUsername = localStorage.getItem("odoo_username");
          if (storedUsername) {
            setUsername(storedUsername);
          }
          const storedIsManager =
            localStorage.getItem("odoo_is_manager") === "true";
          setIsManager(storedIsManager);
        }

        if (!loggedIn && pathname !== "/login") {
          router.push("/login");
          localStorage.removeItem("odoo_username");
          localStorage.removeItem("odoo_is_manager");
        } else if (loggedIn && pathname === "/login") {
          router.push("/");
        }
      } catch (error) {
        console.error("Failed to check auth status:", error);
      } finally {
        setIsLoading(false);
      }
    };

    void checkAuth();
  }, [pathname, router]);

  const login = async (baseUrl: string, loginEmail: string, pass: string) => {
    const result = await invoke<{
      session_id?: string;
      name?: string;
      login?: string;
      is_quanlytainguyen?: boolean;
    }>("odoo_login", {
      baseUrl,
      login: loginEmail,
      password: pass,
    });
    setIsLoggedIn(true);
    const name = result.name || result.login || loginEmail;
    setUsername(name);
    localStorage.setItem("odoo_username", name);

    const manager = !!result.is_quanlytainguyen;
    setIsManager(manager);
    localStorage.setItem("odoo_is_manager", String(manager));

    router.push("/");
    return result;
  };

  const logout = async () => {
    await invoke("odoo_logout");
    setIsLoggedIn(false);
    setUsername(null);
    setIsManager(false);
    localStorage.removeItem("odoo_username");
    localStorage.removeItem("odoo_is_manager");
    router.push("/login");
  };

  const showChildren =
    (isLoggedIn && pathname !== "/login") ||
    (!isLoggedIn && pathname === "/login");

  return (
    <AuthContext.Provider
      value={{ isLoggedIn, isLoading, username, isManager, login, logout }}
    >
      {isLoading || !showChildren ? <LaunchingScreen /> : children}
    </AuthContext.Provider>
  );
}

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) throw new Error("useAuth must be used within AuthProvider");
  return context;
};
