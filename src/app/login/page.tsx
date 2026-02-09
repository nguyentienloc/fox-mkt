"use client";

import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Logo } from "@/components/icons/logo";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuth } from "@/providers/auth-provider";

export default function LoginPage() {
  const { login } = useAuth();
  const [baseUrl, setBaseUrl] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [rememberMe, setRememberMe] = useState(true);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    // Load saved credentials
    const savedUrl = localStorage.getItem("odoo_url");
    const savedLogin = localStorage.getItem("odoo_login");
    const savedPass = localStorage.getItem("odoo_pass");
    const savedRemember = localStorage.getItem("odoo_remember") === "true";

    if (savedUrl) setBaseUrl(savedUrl);
    if (savedRemember) {
      setRememberMe(true);
      if (savedLogin) setUsername(savedLogin);
      if (savedPass) setPassword(savedPass);
    }
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!baseUrl || !username || !password) {
      toast.error("Please fill in all fields");
      return;
    }

    setIsLoading(true);
    try {
      console.log("Attempting Odoo login to:", baseUrl);
      localStorage.setItem("API_BASE_URL", baseUrl);
      const result = await login(baseUrl, username, password);

      console.log("Odoo Login Result:", result);

      // Store session_id if returned in result
      if (result?.session_id) {
        console.log("Saving session_id to localStorage:", result.session_id);
        localStorage.setItem("session_id", result.session_id);
      } else {
        console.warn("No session_id in login result, checking localStorage...");
        const existingSessionId = localStorage.getItem("session_id");
        console.log("Existing session_id in localStorage:", existingSessionId);
      }

      // Save credentials if rememberMe is checked
      localStorage.setItem("odoo_url", baseUrl);
      localStorage.setItem("odoo_remember", String(rememberMe));
      if (rememberMe) {
        localStorage.setItem("odoo_login", username);
        localStorage.setItem("odoo_pass", password);
      } else {
        localStorage.removeItem("odoo_login");
        localStorage.removeItem("odoo_pass");
      }

      toast.success("Đăng nhập thành công");
    } catch (error) {
      console.error("Login error:", error);
      toast.error(
        `Đăng nhập thất bại: ${error instanceof Error ? error.message : String(error)}`,
      );
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-muted/30">
      <Card className="w-full max-w-md shadow-lg border-none">
        <CardHeader className="space-y-4 flex flex-col items-center">
          <Logo className="w-16 h-16" />
          <div className="text-center">
            <CardTitle className="text-2xl font-bold">
              Chào mừng trở lại
            </CardTitle>
            <CardDescription>
              Đăng nhập tài khoản Odoo để tiếp tục
            </CardDescription>
          </div>
        </CardHeader>
        <form onSubmit={handleSubmit}>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="baseUrl">Địa chỉ Odoo Server</Label>
              <Input
                id="baseUrl"
                type="url"
                value={baseUrl}
                onChange={(e) => setBaseUrl(e.target.value)}
                placeholder="https://odoo.yourdomain.com"
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="username">Email / Tên đăng nhập</Label>
              <Input
                id="username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="name@example.com"
                required
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">Mật khẩu</Label>
              <Input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>
            <div className="flex items-center space-x-2 pt-2">
              <Checkbox
                id="remember"
                checked={rememberMe}
                onCheckedChange={(checked) => setRememberMe(!!checked)}
              />
              <Label
                htmlFor="remember"
                className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
              >
                Ghi nhớ đăng nhập
              </Label>
            </div>
          </CardContent>
          <CardFooter className="pt-2">
            <Button
              className="w-full h-11 text-base mt-2"
              type="submit"
              disabled={isLoading}
            >
              {isLoading ? (
                <div className="flex items-center gap-2">
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-current"></div>
                  Đang đăng nhập...
                </div>
              ) : (
                "Đăng nhập"
              )}
            </Button>
          </CardFooter>
        </form>
      </Card>
    </div>
  );
}
